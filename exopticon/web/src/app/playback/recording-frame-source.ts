import { RecordingStore } from "./recording-store";
import { RecordingService } from "../recording.service";
import { DecodedFrame, FrameReader, MkvReader } from "./mkv-reader";
import {
  FrameSourceStatus,
  ObsoleteFrameOperation,
  RecordingReadSession,
  RecordingSessionOpening,
} from "./recording-read-session";

export {
  FrameSourceStatus,
  ObsoleteFrameOperation,
} from "./recording-read-session";

/** Supplies frames for one camera independently of playback timing and presentation.
 * Each seek replaces its session, sharing downloads and cached blobs through its recording store.
 * takeFrame transfers ownership to the caller, which must close the sample.
 */
export class RecordingFrameSource {
  private readonly store: RecordingStore;
  private session?: RecordingReadSession;
  private opening?: RecordingSessionOpening;
  private disposed = false;

  constructor(
    private readonly cameraName: string,
    private recordings: RecordingService,
    private openReader: (blob: Blob) => Promise<FrameReader> = MkvReader.open,
  ) {
    this.store = new RecordingStore(recordings);
  }

  get hasFrames(): boolean {
    return this.session?.hasFrames ?? false;
  }
  get nextFrameTime(): number | undefined {
    return this.session?.nextFrameTime;
  }
  get canBuffer(): boolean {
    return this.session?.canBuffer ?? false;
  }
  get canRetryTransition(): boolean {
    return this.session?.canRetryTransition ?? false;
  }

  retryTransition(): void {
    this.session?.retryTransition();
  }
  takeFrame(): DecodedFrame | undefined {
    return this.session?.takeFrame();
  }

  /** A ready result leaves the first eligible frame available via takeFrame. */
  async seek(utcMilliseconds: number): Promise<FrameSourceStatus> {
    if (this.disposed) {
      throw new ObsoleteFrameOperation();
    }
    this.opening?.dispose();
    this.session = undefined;
    const opening = RecordingReadSession.open(
      this.cameraName,
      utcMilliseconds,
      this.recordings,
      this.store,
      this.openReader,
    );
    this.opening = opening;
    const session = await opening.ready;
    // Opening may have completed just before another seek or disposal.
    if (this.disposed || this.opening !== opening) {
      opening.dispose();
      throw new ObsoleteFrameOperation();
    }
    this.session = session;
    return session.status;
  }

  /** Decode one frame or advance a recording; concurrent callers share the work.
   * A ready result permits continued reading; check hasFrames before taking.
   * Consumers can fill the bounded queue while canBuffer is true.
   */
  buffer(): Promise<FrameSourceStatus> {
    if (this.disposed) {
      return Promise.reject(new ObsoleteFrameOperation());
    }
    return this.session?.buffer() ?? Promise.resolve({ kind: "ready" });
  }

  dispose(): void {
    if (this.disposed) {
      return;
    }
    this.disposed = true;
    this.opening?.dispose();
    this.opening = undefined;
    this.store.dispose();
    this.session = undefined;
  }
}
