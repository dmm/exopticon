import { HttpErrorResponse } from "@angular/common/http";
import { Observable, Subscription } from "rxjs";
import { RecordingDescriptor, RecordingService } from "../recording.service";
import { DecodedFrame, FrameReader, MkvReader } from "./mkv-reader";

export type InspectionPhase =
  | "idle"
  | "seeking"
  | "paused"
  | "stepping"
  | "unavailable"
  | "ended"
  | "error";

export interface InspectionState {
  phase: InspectionPhase;
  requestedTime: number | null;
  displayedTime: number | null;
  error: string | null;
}

class ObsoleteOperation extends Error {}

export class PlaybackController {
  state: InspectionState = {
    phase: "idle",
    requestedTime: null,
    displayedTime: null,
    error: null,
  };

  private generation = 0;
  private cameraName = "";
  private request?: { cancel: () => void };
  private reader?: FrameReader;
  private displayed?: DecodedFrame;
  private cached?: { descriptor: RecordingDescriptor; blob: Blob };
  private destroyed = false;

  constructor(
    private recordings: RecordingService,
    private openReader: (blob: Blob) => Promise<FrameReader> = MkvReader.open,
    private render: (frame: DecodedFrame | null) => void = () => {},
    private notify: (state: InspectionState) => void = () => {},
  ) {}

  setCamera(cameraName: string): void {
    if (this.cameraName === cameraName) return;
    this.cancelActive();
    this.cameraName = cameraName;
    this.cached = undefined;
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
      let blob =
        this.cached?.descriptor.fileId === descriptor.fileId
          ? this.cached.blob
          : undefined;
      if (!blob) {
        blob = await this.fetch(this.recordings.download(descriptor));
        if (!this.current(generation)) return;
        this.cached = { descriptor, blob };
      }
      const reader = await this.openReader(blob);
      if (!this.current(generation)) {
        reader.dispose();
        return;
      }
      this.reader = reader;
      const frame = await reader.firstAtOrAfter(utcMilliseconds);
      if (!this.current(generation)) {
        frame?.sample.close();
        return;
      }
      if (!frame) {
        this.setPhase("unavailable");
        return;
      }
      this.show(frame, "paused", false);
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
        this.state = {
          ...this.state,
          phase: "error",
          error: this.errorMessage(error),
        };
        this.notify(this.state);
      }
    }
  }

  async stepForward(): Promise<void> {
    if (this.state.phase !== "paused" || !this.reader || this.destroyed) return;
    const generation = this.generation;
    this.setPhase("stepping");
    try {
      const frame = await this.reader.next();
      if (!this.current(generation)) {
        frame?.sample.close();
        return;
      }
      if (!frame) {
        this.setPhase("ended");
        return;
      }
      this.show(frame, "paused", true);
    } catch (error) {
      if (!this.current(generation)) return;
      this.state = {
        ...this.state,
        phase: "error",
        error: this.errorMessage(error),
      };
      this.notify(this.state);
    }
  }

  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;
    this.cancelActive();
    this.cached = undefined;
  }

  private show(
    frame: DecodedFrame,
    phase: InspectionPhase,
    advanceRequest: boolean,
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
      requestedTime: advanceRequest
        ? frame.utcMilliseconds
        : this.state.requestedTime,
      displayedTime: frame.utcMilliseconds,
      error: null,
    };
    this.notify(this.state);
  }

  private setPhase(phase: InspectionPhase): void {
    this.state = { ...this.state, phase };
    this.notify(this.state);
  }

  private cancelActive(): void {
    this.generation++;
    this.request?.cancel();
    this.request = undefined;
    this.reader?.dispose();
    this.reader = undefined;
    this.displayed?.sample.close();
    this.displayed = undefined;
    this.render(null);
  }

  private current(generation: number): boolean {
    return !this.destroyed && generation === this.generation;
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
      this.request = pending;
      subscription = source.subscribe({
        next: (value) => {
          if (this.request === pending) this.request = undefined;
          resolve(value);
        },
        error: (error) => {
          if (this.request === pending) this.request = undefined;
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
