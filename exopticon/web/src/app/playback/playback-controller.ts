import { RecordingService } from "../recording.service";
import { DecodedFrame, FrameReader, MkvReader } from "./mkv-reader";
import {
  FrameSourceStatus,
  ObsoleteFrameOperation,
  RecordingFrameSource,
} from "./recording-frame-source";

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
  private frameSource?: RecordingFrameSource;
  private displayed?: DecodedFrame;
  private producer?: Promise<void>;
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
    if (this.destroyed || this.cameraName === cameraName) return;
    this.cancelActive();
    this.cameraName = cameraName;
    this.frameSource?.dispose();
    this.frameSource = cameraName
      ? new RecordingFrameSource(cameraName, this.recordings, this.openReader)
      : undefined;
    this.state = {
      phase: "idle",
      requestedTime: null,
      displayedTime: null,
      error: null,
    };
    this.notify(this.state);
  }

  async seek(utcMilliseconds: number): Promise<void> {
    const frameSource = this.frameSource;
    if (!frameSource || this.destroyed) return;
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
      const result = await frameSource.seek(utcMilliseconds);
      if (!this.current(generation)) return;
      if (result.kind === "ready") {
        this.show(frameSource.takeFrame()!, "paused", utcMilliseconds);
      } else this.applySourceStatus(result);
    } catch (error) {
      if (
        this.current(generation) &&
        !(error instanceof ObsoleteFrameOperation)
      )
        this.fail(error);
    }
  }

  async stepForward(): Promise<void> {
    const frameSource = this.frameSource;
    if (this.state.phase !== "paused" || !frameSource || this.destroyed) return;
    const generation = this.generation;
    this.setPhase("stepping");
    while (
      this.current(generation) &&
      !frameSource.hasFrames &&
      this.isPhase("stepping")
    ) {
      await this.produce();
    }
    if (!this.current(generation) || !this.isPhase("stepping")) return;
    const frame = frameSource.takeFrame()!;
    this.show(frame, "paused", frame.utcMilliseconds);
  }

  play(): void {
    if (this.state.phase !== "paused" || !this.displayed || this.destroyed)
      return;
    this.frozenMediaTime = this.displayed.utcMilliseconds;
    if (this.frameSource?.hasFrames) this.startClock(this.frozenMediaTime);
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
    if (!this.canRetryTransition) return;
    this.frameSource?.retryTransition();
    this.setPhase("paused");
    await this.stepForward();
  }

  get canRetryTransition(): boolean {
    return (
      this.state.phase === "error" && !!this.frameSource?.canRetryTransition
    );
  }

  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    this.cancelActive();
    this.frameSource?.dispose();
    this.frameSource = undefined;
  }

  private desiredMediaTime(): number {
    return this.mediaStart + (this.clock.now() - this.clockStart) * this.speed;
  }

  private startClock(mediaTime: number): void {
    this.mediaStart = mediaTime;
    this.clockStart = this.clock.now();
    this.setPhase("playing");
    this.scheduleFrame();
    if (this.frameSource?.canBuffer) void this.produce();
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
    const frameSource = this.frameSource;
    if (this.state.phase !== "playing" || !frameSource) return;
    const desired = this.desiredMediaTime();
    let latest: DecodedFrame | undefined;
    while (
      this.frameSource?.hasFrames &&
      frameSource.nextFrameTime! <= desired
    ) {
      latest?.sample.close();
      latest = frameSource.takeFrame();
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
    if (!this.frameSource?.hasFrames) {
      this.frozenMediaTime = desired;
      this.setPhase("buffering");
      void this.produce();
      return;
    }
    if (this.frameSource?.canBuffer) void this.produce();
    this.scheduleFrame();
  }

  private produce(): Promise<void> {
    if (this.producer) return this.producer;
    const frameSource = this.frameSource;
    if (!frameSource) return Promise.resolve();
    const generation = this.generation;
    const task = frameSource
      .buffer()
      .then((result) => {
        if (this.current(generation)) this.applySourceStatus(result);
      })
      .catch((error) => {
        if (
          this.current(generation) &&
          !(error instanceof ObsoleteFrameOperation)
        )
          this.fail(error);
      });
    this.producer = task;
    void task.then(() => {
      if (this.producer === task) this.producer = undefined;
      if (!this.current(generation)) return;
      if (this.state.phase === "buffering" && this.frameSource?.hasFrames) {
        this.startClock(this.frozenMediaTime);
      } else if (
        (this.state.phase === "playing" || this.state.phase === "buffering") &&
        this.frameSource?.canBuffer
      ) {
        void this.produce();
      }
    });
    return task;
  }

  private applySourceStatus(result: FrameSourceStatus): void {
    if (result.kind === "ready") return;
    this.stopClock();
    if (result.kind === "gap")
      this.nextRecordingTime = result.nextRecordingTime;
    this.setPhase(result.kind);
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
    this.producer = undefined;
    this.nextRecordingTime = null;
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

  private errorMessage(error: unknown): string {
    if (error instanceof Error) return error.message;
    return "Could not open recording";
  }
}
