import { HttpErrorResponse } from "@angular/common/http";
import { Observable, Subscription } from "rxjs";
import { RecordingDescriptor, RecordingService } from "../recording.service";
import { DecodedFrame, FrameReader, MkvReader } from "./mkv-reader";

export type PlaybackPhase =
  | "idle"
  | "seeking"
  | "paused"
  | "stepping"
  | "playing"
  | "buffering"
  | "gap"
  | "unavailable"
  | "ended"
  | "error";
export type PlaybackSpeed = 1 | 2 | 4;

export interface PlaybackState {
  phase: PlaybackPhase;
  requestedTime: number | null;
  displayedTime: number | null;
  error: string | null;
}

export interface PlaybackClock {
  now(): number;
  requestFrame(callback: () => void): number;
  cancelFrame(id: number): void;
}

const browserClock: PlaybackClock = {
  now: () => performance.now(),
  requestFrame: (callback) => requestAnimationFrame(callback),
  cancelFrame: (id) => cancelAnimationFrame(id),
};
const MAX_QUEUED_FRAMES = 6;
const MAX_PREFETCH_BYTES = 32 * 1024 * 1024;
const NANOSECONDS_PER_MILLISECOND = 1_000_000n;

function recordingTimeNanoseconds(value: string): bigint {
  const match =
    /^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d{1,9}))?(Z|[+-]\d\d:\d\d)$/.exec(
      value,
    );
  if (!match) throw new Error("Invalid recording interval time");
  const seconds = Date.parse(`${match[1]}${match[3]}`);
  if (!Number.isFinite(seconds))
    throw new Error("Invalid recording interval time");
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

class ObsoleteOperation extends Error {}

export class PlaybackController {
  state: PlaybackState = {
    phase: "idle",
    requestedTime: null,
    displayedTime: null,
    error: null,
  };
  speed: PlaybackSpeed = 1;
  nextRecordingTime: number | null = null;

  private generation = 0;
  private cameraName = "";
  private requests = new Set<{ cancel: () => void }>();
  private reader?: FrameReader;
  private displayed?: DecodedFrame;
  private queue: DecodedFrame[] = [];
  private producer?: Promise<void>;
  private fileExhausted = false;
  private currentFile?: { descriptor: RecordingDescriptor; blob: Blob };
  private prefetched?: { descriptor: RecordingDescriptor; blob: Blob };
  private successor: RecordingDescriptor | null | undefined;
  private prefetchTask?: Promise<void>;
  private prefetchError?: unknown;
  private frameRequest?: number;
  private clockStart = 0;
  private mediaStart = 0;
  private frozenMediaTime = 0;
  private destroyed = false;

  constructor(
    private recordings: RecordingService,
    private openReader: (blob: Blob) => Promise<FrameReader> = MkvReader.open,
    private render: (frame: DecodedFrame | null) => void = () => {},
    private notify: (state: PlaybackState) => void = () => {},
    private clock: PlaybackClock = browserClock,
  ) {}

  setCamera(cameraName: string): void {
    if (this.cameraName === cameraName) return;
    this.cancelActive();
    this.cameraName = cameraName;
    this.currentFile = undefined;
    this.prefetched = undefined;
    this.state = {
      phase: "idle",
      requestedTime: null,
      displayedTime: null,
      error: null,
    };
    this.notify(this.state);
  }

  async seek(utcMilliseconds: number): Promise<void> {
    if (!this.cameraName || this.destroyed) return;
    this.cancelActive();
    const generation = this.generation;
    this.state = {
      phase: "seeking",
      requestedTime: utcMilliseconds,
      displayedTime: null,
      error: null,
    };
    this.notify(this.state);
    try {
      const descriptor = await this.fetch(
        this.recordings.resolveAt(this.cameraName, utcMilliseconds),
      );
      if (!this.current(generation)) return;
      if (this.currentFile?.descriptor.fileId !== descriptor.fileId)
        this.currentFile = undefined;
      if (
        this.prefetched?.descriptor.fileId !== descriptor.fileId &&
        !this.currentFile
      )
        this.prefetched = undefined;
      let blob =
        this.currentFile?.descriptor.fileId === descriptor.fileId
          ? this.currentFile.blob
          : this.prefetched?.descriptor.fileId === descriptor.fileId
            ? this.prefetched.blob
            : undefined;
      if (!blob) {
        blob = await this.fetch(this.recordings.download(descriptor));
        if (!this.current(generation)) return;
      }
      this.currentFile = { descriptor, blob };
      if (this.prefetched?.descriptor.fileId === descriptor.fileId)
        this.prefetched = undefined;
      const reader = await this.openReader(blob);
      if (!this.current(generation)) {
        reader.dispose();
        return;
      }
      this.reader = reader;
      this.startPrefetch(generation);
      const frame = await reader.firstAtOrAfter(utcMilliseconds);
      if (!this.current(generation)) {
        frame?.sample.close();
        return;
      }
      if (frame) {
        this.show(frame, "paused", utcMilliseconds);
      } else {
        this.fileExhausted = true;
        while (
          this.current(generation) &&
          this.isPhase("seeking") &&
          !this.queue.length
        ) {
          await this.produce();
        }
        if (!this.current(generation)) return;
        if (this.isPhase("seeking") && this.queue.length) {
          this.show(this.queue.shift()!, "paused", utcMilliseconds);
        } else if (this.isPhase("ended")) {
          this.setPhase("unavailable");
        }
      }
    } catch (error) {
      if (!this.current(generation) || error instanceof ObsoleteOperation)
        return;
      if (
        error instanceof HttpErrorResponse &&
        error.status === 404 &&
        error.error?.error === "unavailable_time"
      ) {
        this.setPhase("unavailable");
      } else {
        this.fail(error);
      }
    }
  }

  async stepForward(): Promise<void> {
    if (this.state.phase !== "paused" || !this.reader || this.destroyed) return;
    const generation = this.generation;
    this.setPhase("stepping");
    while (
      this.current(generation) &&
      !this.queue.length &&
      this.isPhase("stepping")
    ) {
      await this.produce();
    }
    if (!this.current(generation) || !this.isPhase("stepping")) return;
    const frame = this.queue.shift()!;
    this.show(frame, "paused", frame.utcMilliseconds);
  }

  play(): void {
    if (this.state.phase !== "paused" || !this.displayed || this.destroyed)
      return;
    this.frozenMediaTime = this.displayed.utcMilliseconds;
    if (this.queue.length) this.startClock(this.frozenMediaTime);
    else {
      this.setPhase("buffering");
      void this.produce();
    }
  }

  pause(): void {
    if (this.state.phase !== "playing" && this.state.phase !== "buffering")
      return;
    this.stopClock();
    this.state = {
      ...this.state,
      phase: "paused",
      requestedTime: this.state.displayedTime,
    };
    this.notify(this.state);
  }

  setSpeed(speed: PlaybackSpeed): void {
    if (speed !== 1 && speed !== 2 && speed !== 4) return;
    if (this.state.phase === "playing") {
      this.mediaStart = this.desiredMediaTime();
      this.clockStart = this.clock.now();
    }
    this.speed = speed;
    this.notify(this.state);
  }

  async jumpToNext(): Promise<void> {
    if (this.state.phase !== "gap" || this.nextRecordingTime === null) return;
    await this.seek(this.nextRecordingTime);
  }

  async retryTransition(): Promise<void> {
    if (this.state.phase !== "error" || !this.fileExhausted || !this.reader)
      return;
    this.prefetchError = undefined;
    this.setPhase("paused");
    await this.stepForward();
  }

  get canRetryTransition(): boolean {
    return this.state.phase === "error" && this.fileExhausted && !!this.reader;
  }

  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    this.cancelActive();
    this.currentFile = undefined;
    this.prefetched = undefined;
  }

  private desiredMediaTime(): number {
    return this.mediaStart + (this.clock.now() - this.clockStart) * this.speed;
  }

  private startClock(mediaTime: number): void {
    this.mediaStart = mediaTime;
    this.clockStart = this.clock.now();
    this.setPhase("playing");
    this.scheduleFrame();
    if (this.queue.length < MAX_QUEUED_FRAMES) void this.produce();
  }

  private stopClock(): void {
    if (this.frameRequest !== undefined) {
      this.clock.cancelFrame(this.frameRequest);
      this.frameRequest = undefined;
    }
  }

  private scheduleFrame(): void {
    if (this.frameRequest !== undefined || this.state.phase !== "playing")
      return;
    this.frameRequest = this.clock.requestFrame(() => {
      this.frameRequest = undefined;
      this.present();
    });
  }

  private present(): void {
    if (this.state.phase !== "playing") return;
    const desired = this.desiredMediaTime();
    let latest: DecodedFrame | undefined;
    while (this.queue.length && this.queue[0].utcMilliseconds <= desired) {
      latest?.sample.close();
      latest = this.queue.shift();
    }
    if (latest) {
      try {
        this.show(latest, "playing", desired);
      } catch (error) {
        this.fail(error);
        return;
      }
    } else {
      this.state = { ...this.state, requestedTime: desired };
      this.notify(this.state);
    }
    if (!this.queue.length) {
      this.frozenMediaTime = desired;
      this.setPhase("buffering");
      void this.produce();
      return;
    }
    if (this.queue.length < MAX_QUEUED_FRAMES) void this.produce();
    this.scheduleFrame();
  }

  private produce(): Promise<void> {
    if (this.producer) return this.producer;
    const generation = this.generation;
    const task = this.produceOne(generation).catch((error) => {
      if (this.current(generation) && !(error instanceof ObsoleteOperation))
        this.fail(error);
    });
    this.producer = task;
    void task.then(() => {
      if (this.producer === task) this.producer = undefined;
      if (!this.current(generation)) return;
      if (this.state.phase === "buffering" && this.queue.length) {
        this.startClock(this.frozenMediaTime);
      } else if (
        (this.state.phase === "playing" || this.state.phase === "buffering") &&
        this.queue.length < MAX_QUEUED_FRAMES &&
        !(this.fileExhausted && this.queue.length)
      ) {
        void this.produce();
      }
    });
    return task;
  }

  private async produceOne(generation: number): Promise<void> {
    if (
      !this.current(generation) ||
      !this.reader ||
      this.queue.length >= MAX_QUEUED_FRAMES
    )
      return;
    if (this.fileExhausted) {
      if (!this.queue.length) await this.advanceFile(generation);
      return;
    }
    const frame = await this.reader.next();
    if (!this.current(generation)) {
      frame?.sample.close();
      return;
    }
    if (frame) this.queue.push(frame);
    else {
      this.fileExhausted = true;
      if (!this.queue.length) await this.advanceFile(generation);
    }
  }

  private async advanceFile(generation: number): Promise<void> {
    const current = this.currentFile?.descriptor;
    if (!current) throw new Error("Missing current recording");
    if (this.prefetchTask) await this.prefetchTask;
    if (!this.current(generation)) return;
    if (this.prefetchError && this.successor === undefined)
      throw this.prefetchError;
    // A null found during prefetch can become stale before this file ends.
    if (this.successor == null) {
      const result = await this.fetch(this.recordings.getNext(current));
      if (!this.current(generation)) return;
      this.successor = result.recording;
    }
    const successor = this.successor;
    if (!successor) {
      this.stopClock();
      this.setPhase("ended");
      return;
    }
    const nextStartNanoseconds = recordingTimeNanoseconds(successor.beginTime);
    // Date and the timeline use milliseconds. Round upward so the backend's
    // microsecond-precision availability check accepts a jump to this file.
    const nextStart = ceilMilliseconds(nextStartNanoseconds);
    if (nextStartNanoseconds > recordingTimeNanoseconds(current.endTime)) {
      this.stopClock();
      this.nextRecordingTime = nextStart;
      this.setPhase("gap");
      return;
    }
    if (this.prefetchError) throw this.prefetchError;
    let blob =
      this.prefetched?.descriptor.fileId === successor.fileId
        ? this.prefetched.blob
        : undefined;
    if (!blob) {
      blob = await this.fetch(this.recordings.download(successor));
      if (!this.current(generation)) return;
    }
    const reader = await this.openReader(blob);
    if (!this.current(generation)) {
      reader.dispose();
      return;
    }
    this.reader?.dispose();
    this.reader = reader;
    this.currentFile = { descriptor: successor, blob };
    this.prefetched = undefined;
    this.successor = undefined;
    this.prefetchError = undefined;
    this.fileExhausted = false;
    const minimum =
      this.displayed?.utcNanoseconds ??
      BigInt(Math.trunc(this.state.requestedTime ?? nextStart)) * 1_000_000n;
    let frame = await reader.firstAtOrAfter(Number(minimum / 1_000_000n));
    if (!this.current(generation)) {
      frame?.sample.close();
      return;
    }
    while (frame && frame.utcNanoseconds <= minimum) {
      frame.sample.close();
      frame = await reader.next();
      if (!this.current(generation)) {
        frame?.sample.close();
        return;
      }
    }
    if (frame) this.queue.push(frame);
    else this.fileExhausted = true;
    this.startPrefetch(generation);
  }

  private startPrefetch(generation: number): void {
    const current = this.currentFile?.descriptor;
    if (!current || !this.current(generation)) return;
    this.successor = undefined;
    this.prefetchError = undefined;
    const task = (async () => {
      const result = await this.fetch(this.recordings.getNext(current));
      if (!this.current(generation)) return;
      this.successor = result.recording;
      if (this.prefetched?.descriptor.fileId !== result.recording?.fileId)
        this.prefetched = undefined;
      if (!result.recording || result.recording.byteLength > MAX_PREFETCH_BYTES)
        return;
      if (this.prefetched?.descriptor.fileId === result.recording.fileId)
        return;
      const blob = await this.fetch(this.recordings.download(result.recording));
      if (this.current(generation))
        this.prefetched = { descriptor: result.recording, blob };
    })().catch((error) => {
      if (this.current(generation) && !(error instanceof ObsoleteOperation))
        this.prefetchError = error;
    });
    this.prefetchTask = task;
    void task.then(() => {
      if (this.prefetchTask === task) this.prefetchTask = undefined;
    });
  }

  private show(
    frame: DecodedFrame,
    phase: PlaybackPhase,
    requestedTime: number | undefined,
  ): void {
    this.displayed?.sample.close();
    this.displayed = frame;
    try {
      this.render(frame);
    } catch (error) {
      frame.sample.close();
      this.displayed = undefined;
      throw error;
    }
    this.state = {
      phase,
      requestedTime: requestedTime ?? frame.utcMilliseconds,
      displayedTime: frame.utcMilliseconds,
      error: null,
    };
    this.notify(this.state);
  }

  private setPhase(phase: PlaybackPhase): void {
    this.state = { ...this.state, phase };
    this.notify(this.state);
  }

  private fail(error: unknown): void {
    this.stopClock();
    this.state = {
      ...this.state,
      phase: "error",
      error: this.errorMessage(error),
    };
    this.notify(this.state);
  }

  private cancelActive(): void {
    this.generation++;
    this.stopClock();
    for (const request of this.requests) request.cancel();
    this.requests.clear();
    this.prefetchTask = undefined;
    this.prefetchError = undefined;
    this.successor = undefined;
    this.producer = undefined;
    this.fileExhausted = false;
    this.nextRecordingTime = null;
    this.reader?.dispose();
    this.reader = undefined;
    for (const frame of this.queue) frame.sample.close();
    this.queue = [];
    this.displayed?.sample.close();
    this.displayed = undefined;
    this.render(null);
  }

  private current(generation: number): boolean {
    return !this.destroyed && generation === this.generation;
  }

  private isPhase(phase: PlaybackPhase): boolean {
    return this.state.phase === phase;
  }

  private fetch<T>(source: Observable<T>): Promise<T> {
    return new Promise<T>((resolve, reject) => {
      let subscription: Subscription | undefined;
      const pending = {
        cancel: () => {
          subscription?.unsubscribe();
          reject(new ObsoleteOperation());
        },
      };
      this.requests.add(pending);
      subscription = source.subscribe({
        next: (value) => {
          this.requests.delete(pending);
          resolve(value);
        },
        error: (error) => {
          this.requests.delete(pending);
          reject(error);
        },
      });
    });
  }

  private errorMessage(error: unknown): string {
    if (error instanceof Error) return error.message;
    return "Could not open recording";
  }
}
