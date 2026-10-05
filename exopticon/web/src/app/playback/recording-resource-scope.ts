import { Observable, Subscription } from "rxjs";

export class ObsoleteFrameOperation extends Error {}

/** Owns cancellation and resources, including results arriving after disposal. */
export class RecordingResourceScope {
  private disposed = false;
  private requests = new Set<{ cancel: () => void }>();
  private resources = new Map<object, () => void>();

  dispose(): void {
    if (this.disposed) {
      return;
    }
    this.disposed = true;
    for (const request of this.requests) {
      request.cancel();
    }
    this.requests.clear();
    for (const release of this.resources.values()) {
      release();
    }
    this.resources.clear();
  }

  /** Commit synchronous state changes only while the scope is active. */
  use<T>(action: () => T): T {
    this.assertActive();
    return action();
  }

  transfer(value: object): void {
    this.assertActive();
    this.resources.delete(value);
  }

  private assertActive(): void {
    if (this.disposed) {
      throw new ObsoleteFrameOperation();
    }
  }

  async run<T>(start: () => Promise<T>): Promise<T> {
    this.assertActive();
    try {
      const value = await start();
      this.assertActive();
      return value;
    } catch (error) {
      this.assertActive();
      throw error;
    }
  }

  async acquire<T extends object | null>(
    start: () => Promise<T>,
    release: (value: NonNullable<T>) => void,
  ): Promise<T> {
    this.assertActive();
    try {
      const value = await start();
      if (value !== null) {
        const cleanup = () => release(value as NonNullable<T>);
        if (this.disposed) {
          cleanup();
        } else {
          this.resources.set(value, cleanup);
        }
      }
      this.assertActive();
      return value;
    } catch (error) {
      this.assertActive();
      throw error;
    }
  }

  release(value: object): void {
    const cleanup = this.resources.get(value);
    this.resources.delete(value);
    cleanup?.();
  }

  fetch<T>(source: () => Observable<T>): Promise<T> {
    return this.run(
      () =>
        new Promise<T>((resolve, reject) => {
          let subscription: Subscription | undefined;
          const pending = {
            cancel: () => {
              subscription?.unsubscribe();
              reject(new ObsoleteFrameOperation());
            },
          };
          this.requests.add(pending);
          subscription = source().subscribe({
            next: (value) => {
              this.requests.delete(pending);
              resolve(value);
            },
            error: (error) => {
              this.requests.delete(pending);
              reject(error);
            },
          });
        }),
    );
  }
}
