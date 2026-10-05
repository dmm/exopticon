import { Subject } from "rxjs";
import {
  ObsoleteFrameOperation,
  RecordingResourceScope,
} from "./recording-resource-scope";

describe("RecordingResourceScope", () => {
  it("releases owned resources once and leaves transferred resources with their caller", async () => {
    const scope = new RecordingResourceScope();
    const releaseOwned = jasmine.createSpy("releaseOwned");
    const releaseTransferred = jasmine.createSpy("releaseTransferred");
    const releaseEarly = jasmine.createSpy("releaseEarly");
    const owned = await scope.acquire(async () => ({}), releaseOwned);
    const transferred = await scope.acquire(
      async () => ({}),
      releaseTransferred,
    );
    const early = await scope.acquire(async () => ({}), releaseEarly);
    scope.transfer(transferred);
    scope.release(early);
    scope.dispose();
    scope.dispose();
    scope.release(owned);
    expect(releaseOwned).toHaveBeenCalledTimes(1);
    expect(releaseEarly).toHaveBeenCalledTimes(1);
    expect(releaseTransferred).not.toHaveBeenCalled();
  });

  it("unsubscribes pending requests and prevents new work and state changes", async () => {
    const scope = new RecordingResourceScope();
    const response = new Subject<string>();
    const pending = scope.fetch(() => response);
    const rejected = expectAsync(pending).toBeRejectedWithError(
      ObsoleteFrameOperation,
    );
    expect(response.observers.length).toBe(1);
    scope.dispose();
    await rejected;
    expect(response.observers.length).toBe(0);
    const start = jasmine.createSpy("start").and.resolveTo("value");
    await expectAsync(scope.run(start)).toBeRejectedWithError(
      ObsoleteFrameOperation,
    );
    await expectAsync(scope.acquire(start, () => {})).toBeRejectedWithError(
      ObsoleteFrameOperation,
    );
    expect(start).not.toHaveBeenCalled();
    const change = jasmine.createSpy("change");
    expect(() => scope.use(change)).toThrowError(ObsoleteFrameOperation);
    expect(change).not.toHaveBeenCalled();
  });

  it("releases a resource that arrives after disposal", async () => {
    const scope = new RecordingResourceScope();
    let deliver!: (resource: object) => void;
    const release = jasmine.createSpy("release");
    const pending = scope.acquire(
      () =>
        new Promise<object>((resolve) => {
          deliver = resolve;
        }),
      release,
    );
    const rejected = expectAsync(pending).toBeRejectedWithError(
      ObsoleteFrameOperation,
    );
    scope.dispose();
    const resource = {};
    deliver(resource);
    await rejected;
    expect(release).toHaveBeenCalledOnceWith(resource);
  });
});
