import { from } from "rxjs";
import { RecordingStore } from "./recording-store";
import { HttpErrorResponse } from "@angular/common/http";
import { RecordingDescriptor, RecordingService } from "../recording.service";
import { DecodedFrame, FrameReader, MkvReader } from "./mkv-reader";
import { RecordingResourceScope } from "./recording-resource-scope";

export type FrameSourceStatus =
  | { kind: "ready" }
  | { kind: "gap"; nextRecordingTime: number }
  | { kind: "ended" }
  | { kind: "unavailable" };

export { ObsoleteFrameOperation } from "./recording-resource-scope";

const MAX_QUEUED_FRAMES = 6;
const MAX_PREFETCH_BYTES = 32 * 1024 * 1024;
const NANOSECONDS_PER_MILLISECOND = 1_000_000n;

function recordingTimeNanoseconds(value: string): bigint {
  const match =
    /^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d{1,9}))?(Z|[+-]\d\d:\d\d)$/.exec(
      value,
    );
  if (!match) {
    throw new Error("Invalid recording interval time");
  }
  const seconds = Date.parse(`${match[1]}${match[3]}`);
  if (!Number.isFinite(seconds)) {
    throw new Error("Invalid recording interval time");
  }
  return (
    BigInt(seconds) * NANOSECONDS_PER_MILLISECOND +
    BigInt((match[2] ?? "").padEnd(9, "0"))
  );
}

function ceilMilliseconds(nanoseconds: bigint): number {
  const quotient = nanoseconds / NANOSECONDS_PER_MILLISECOND;
  const remainder = nanoseconds % NANOSECONDS_PER_MILLISECOND;
  return Number(quotient + (remainder > 0n ? 1n : 0n));
}

type PrefetchResult =
  | { kind: "missing" }
  | { kind: "available"; descriptor: RecordingDescriptor }
  | { kind: "deferred"; descriptor: RecordingDescriptor }
  | { kind: "lookup-failed"; error: unknown }
  | {
      kind: "download-failed";
      descriptor: RecordingDescriptor;
      error: unknown;
    };

interface OpenRecording {
  descriptor: RecordingDescriptor;
  reader: FrameReader;
  successor: Promise<PrefetchResult>;
}

type ReadingState =
  | { kind: "opening" }
  | { kind: "reading"; recording: OpenRecording }
  | { kind: "exhausted"; recording: OpenRecording }
  | { kind: "gap"; nextRecordingTime: number }
  | { kind: "ended" }
  | { kind: "unavailable" }
  | { kind: "disposed" };

/** Cancellation remains accessible while opening is pending. */
export interface RecordingSessionOpening {
  readonly ready: Promise<RecordingReadSession>;
  dispose(): void;
}

/** One seek's reading state. Async cancellation and ownership belong to scope. */
export class RecordingReadSession {
  private readonly scope = new RecordingResourceScope();
  private state: ReadingState = { kind: "opening" };
  private queue: DecodedFrame[] = [];
  private producer?: Promise<FrameSourceStatus>;
  private minimumTime = 0n;

  private constructor(
    private readonly cameraName: string,
    private recordings: RecordingService,
    private store: RecordingStore,
    private openReader: (blob: Blob) => Promise<FrameReader> = MkvReader.open,
  ) {}

  /** Start opening immediately without exposing a partially initialized session. */
  static open(
    cameraName: string,
    utcMilliseconds: number,
    recordings: RecordingService,
    store: RecordingStore,
    openReader: (blob: Blob) => Promise<FrameReader> = MkvReader.open,
  ): RecordingSessionOpening {
    const session = new RecordingReadSession(
      cameraName,
      recordings,
      store,
      openReader,
    );
    const ready = session.scope
      .run(async () => {
        await session.initialize(utcMilliseconds);
        return session;
      })
      .catch((error) => {
        session.dispose();
        throw error;
      });
    return {
      ready,
      dispose: () => session.dispose(),
    };
  }

  get hasFrames(): boolean {
    return this.queue.length > 0;
  }

  get nextFrameTime(): number | undefined {
    return this.queue[0]?.utcMilliseconds;
  }

  get canBuffer(): boolean {
    switch (this.state.kind) {
      case "reading":
        return this.queue.length < MAX_QUEUED_FRAMES;
      case "exhausted":
        return this.queue.length === 0;
      default:
        return false;
    }
  }

  get canRetryTransition(): boolean {
    return this.state.kind === "exhausted";
  }

  retryTransition(): void {
    if (this.state.kind === "exhausted") {
      const recording = this.state.recording;
      recording.successor = this.prefetch(recording.descriptor);
    }
  }

  takeFrame(): DecodedFrame | undefined {
    const frame = this.queue.shift();
    if (frame) {
      this.scope.transfer(frame);
      this.minimumTime = frame.utcNanoseconds;
    }
    return frame;
  }

  /** Open this session once; a ready result leaves its first frame queued. */
  private initialize(utcMilliseconds: number): Promise<FrameSourceStatus> {
    return this.scope.run(async () => {
      this.minimumTime =
        BigInt(Math.trunc(utcMilliseconds)) * NANOSECONDS_PER_MILLISECOND;
      try {
        const descriptor = await this.scope.fetch(() =>
          this.recordings.resolveAt(this.cameraName, utcMilliseconds),
        );
        const recording = await this.open(descriptor);
        const frame = await this.readFrame(() =>
          recording.reader.firstAtOrAfter(utcMilliseconds),
        );
        this.acceptFrame(recording, frame);
        while (!this.hasFrames && this.canBuffer) {
          await this.buffer();
        }
        if (this.status.kind === "ended") {
          this.transition({ kind: "unavailable" });
        }
        return this.status;
      } catch (error) {
        if (
          error instanceof HttpErrorResponse &&
          error.status === 404 &&
          error.error?.error === "unavailable_time"
        ) {
          this.transition({ kind: "unavailable" });
          return this.status;
        }
        throw error;
      }
    });
  }

  buffer(): Promise<FrameSourceStatus> {
    if (this.producer) {
      return this.producer;
    }
    const task = this.scope.run(async () => {
      await this.produceOne();
      return this.status;
    });
    this.producer = task;
    const clear = () => {
      if (this.producer === task) {
        this.producer = undefined;
      }
    };
    void task.then(clear, clear);
    return task;
  }

  dispose(): void {
    this.state = { kind: "disposed" };
    this.queue = [];
    this.scope.dispose();
  }

  get status(): FrameSourceStatus {
    switch (this.state.kind) {
      case "gap":
      case "ended":
      case "unavailable":
        return this.state;
      default:
        return { kind: "ready" };
    }
  }

  private transition(state: ReadingState): void {
    this.scope.use(() => {
      this.state = state;
    });
  }

  private acceptFrame(
    recording: OpenRecording,
    frame: DecodedFrame | null,
  ): void {
    this.scope.use(() => {
      if (frame) {
        this.queue.push(frame);
        this.state = { kind: "reading", recording };
      } else {
        this.state = { kind: "exhausted", recording };
      }
    });
  }

  private async produceOne(): Promise<void> {
    const state = this.state;
    switch (state.kind) {
      case "reading": {
        if (this.queue.length >= MAX_QUEUED_FRAMES) {
          return;
        }
        const frame = await this.readFrame(() => state.recording.reader.next());
        this.acceptFrame(state.recording, frame);
        if (!frame && !this.queue.length) {
          await this.advanceFile(state.recording);
        }
        return;
      }
      case "exhausted": {
        if (!this.queue.length) {
          await this.advanceFile(state.recording);
        }
        return;
      }
      default:
        return;
    }
  }

  private async advanceFile(current: OpenRecording): Promise<void> {
    let result = await this.scope.run(() => current.successor);
    // A missing successor may become available while this file plays.
    if (result.kind === "missing") {
      const next = await this.scope.fetch(() =>
        this.recordings.getNext(current.descriptor),
      );
      result = next.recording
        ? { kind: "deferred", descriptor: next.recording }
        : { kind: "missing" };
    }
    if (result.kind === "lookup-failed") {
      throw result.error;
    }
    if (result.kind === "missing") {
      this.transition({ kind: "ended" });
      return;
    }
    const successor = result.descriptor;
    const nextStart = recordingTimeNanoseconds(successor.beginTime);
    if (nextStart > recordingTimeNanoseconds(current.descriptor.endTime)) {
      // Round upward to satisfy the backend's microsecond availability check.
      this.transition({
        kind: "gap",
        nextRecordingTime: ceilMilliseconds(nextStart),
      });
      return;
    }
    if (result.kind === "download-failed") {
      throw result.error;
    }
    const recording = await this.open(successor);
    this.scope.release(current.reader);
    let frame = await this.readFrame(() =>
      recording.reader.firstAtOrAfter(
        Number(this.minimumTime / NANOSECONDS_PER_MILLISECOND),
      ),
    );
    while (frame && frame.utcNanoseconds <= this.minimumTime) {
      this.scope.release(frame);
      frame = await this.readFrame(() => recording.reader.next());
    }
    this.acceptFrame(recording, frame);
  }

  private async open(descriptor: RecordingDescriptor): Promise<OpenRecording> {
    const blob = await this.getBlob(descriptor);
    const reader = await this.scope.acquire(
      () => this.openReader(blob),
      (value) => value.dispose(),
    );
    const recording = {
      descriptor,
      reader,
      successor: this.prefetch(descriptor),
    };
    this.transition({ kind: "reading", recording });
    return recording;
  }

  private async prefetch(
    current: RecordingDescriptor,
  ): Promise<PrefetchResult> {
    let descriptor: RecordingDescriptor;
    try {
      const next = await this.scope.fetch(() =>
        this.recordings.getNext(current),
      );
      if (!next.recording) {
        return { kind: "missing" };
      }
      descriptor = next.recording;
    } catch (error) {
      return { kind: "lookup-failed", error };
    }
    try {
      if (descriptor.byteLength > MAX_PREFETCH_BYTES) {
        return { kind: "deferred", descriptor };
      }
      await this.getBlob(descriptor);
      return { kind: "available", descriptor };
    } catch (error) {
      return { kind: "download-failed", descriptor, error };
    }
  }

  private getBlob(descriptor: RecordingDescriptor): Promise<Blob> {
    // Unsubscribing cancels this session's wait, not the shared download.
    return this.scope.fetch(() => from(this.store.get(descriptor)));
  }

  private readFrame(
    start: () => Promise<DecodedFrame | null>,
  ): Promise<DecodedFrame | null> {
    return this.scope.acquire(start, (frame) => frame.sample.close());
  }
}
