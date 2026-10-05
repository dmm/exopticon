import { RecordingDescriptor, RecordingService } from "../recording.service";
import {
  ObsoleteFrameOperation,
  RecordingResourceScope,
} from "./recording-resource-scope";

/** Camera-scoped downloads and a two-recording LRU cache. Sessions borrow
 * blobs; evicting a cache entry does not invalidate a reader already using that
 * blob. Pending downloads survive seeks and are canceled only when the store is
 * disposed.
 */
export class RecordingStore {
  private readonly scope = new RecordingResourceScope();
  private readonly cached = new Map<number, Blob>();
  private readonly pending = new Map<number, Promise<Blob>>();
  private disposed = false;

  constructor(private recordings: RecordingService) {}

  get(descriptor: RecordingDescriptor): Promise<Blob> {
    if (this.disposed) {
      return Promise.reject(new ObsoleteFrameOperation());
    }
    const id = descriptor.fileId;
    const cached = this.cached.get(id);
    if (cached) {
      this.cached.delete(id);
      this.cached.set(id, cached);
      return Promise.resolve(cached);
    }
    const pending = this.pending.get(id);
    if (pending) {
      return pending;
    }
    const task = this.scope
      .fetch(() => this.recordings.download(descriptor))
      .then((blob) =>
        this.scope.use(() => {
          this.cached.set(id, blob);
          while (this.cached.size > 2) {
            this.cached.delete(this.cached.keys().next().value!);
          }
          return blob;
        }),
      );
    this.pending.set(id, task);
    const clear = () => {
      this.pending.delete(id);
    };
    void task.then(clear, clear);
    return task;
  }

  dispose(): void {
    if (this.disposed) {
      return;
    }
    this.disposed = true;
    this.scope.dispose();
    this.pending.clear();
    this.cached.clear();
  }
}
