import { RecordingStore } from "./recording-store";
import { of } from "rxjs";
import { RecordingDescriptor, RecordingService } from "../recording.service";
import { DecodedFrame, FrameReader } from "./mkv-reader";
import {
  ObsoleteFrameOperation,
  RecordingReadSession,
  RecordingSessionOpening,
} from "./recording-read-session";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

const descriptor: RecordingDescriptor = {
  fileId: 1,
  beginTime: "2026-09-23T10:00:00Z",
  endTime: "2026-09-23T10:00:10Z",
  byteLength: 4,
  contentUrl: "/content",
  nextUrl: "/next",
};
const base = Date.parse(descriptor.beginTime);
function frame(): DecodedFrame {
  return {
    utcMilliseconds: base,
    utcNanoseconds: BigInt(base) * 1_000_000n,
    sample: {
      close: jasmine.createSpy("close"),
    } as unknown as DecodedFrame["sample"],
  };
}

describe("RecordingReadSession cancellation", () => {
  let recordings: jasmine.SpyObj<RecordingService>;
  let reader: jasmine.SpyObj<FrameReader>;
  let store: RecordingStore;
  let opening: RecordingSessionOpening;
  beforeEach(() => {
    recordings = jasmine.createSpyObj("recordings", [
      "resolveAt",
      "download",
      "getNext",
    ]);
    recordings.resolveAt.and.returnValue(of(descriptor));
    recordings.download.and.returnValue(of(new Blob(["mkv"])));
    recordings.getNext.and.returnValue(of({ recording: null }));
    store = new RecordingStore(recordings);
    reader = jasmine.createSpyObj("reader", [
      "firstAtOrAfter",
      "next",
      "dispose",
    ]);
  });
  afterEach(() => {
    opening.dispose();
    store.dispose();
  });

  it("returns an initialized session with its first frame ready", async () => {
    const value = frame();
    reader.firstAtOrAfter.and.resolveTo(value);
    opening = RecordingReadSession.open(
      "front",
      base,
      recordings,
      store,
      async () => reader,
    );
    const session = await opening.ready;
    expect(session.status).toEqual({ kind: "ready" });
    expect(session.takeFrame()).toBe(value);
    opening.dispose();
    expect(value.sample.close).not.toHaveBeenCalled();
    expect(reader.dispose).toHaveBeenCalledTimes(1);
    await expectAsync(session.buffer()).toBeRejectedWithError(
      ObsoleteFrameOperation,
    );
    value.sample.close();
  });

  it("cleans up the reader when opening fails", async () => {
    reader.firstAtOrAfter.and.rejectWith(new Error("Decode failed"));
    opening = RecordingReadSession.open(
      "front",
      base,
      recordings,
      store,
      async () => reader,
    );
    await expectAsync(opening.ready).toBeRejectedWithError("Decode failed");
    expect(reader.dispose).toHaveBeenCalledTimes(1);
    opening.dispose();
    expect(reader.dispose).toHaveBeenCalledTimes(1);
  });

  it("owns an opened reader before the initialization continuation resumes", async () => {
    const opened = deferred<FrameReader>();
    const started = deferred<void>();
    opening = RecordingReadSession.open(
      "front",
      base,
      recordings,
      store,
      () => {
        started.resolve();
        return opened.promise;
      },
    );
    const initializing = opening.ready;
    const rejected = expectAsync(initializing).toBeRejectedWithError(
      ObsoleteFrameOperation,
    );
    await started.promise;
    // acquire's continuation runs first; disposal runs before its caller resumes.
    void opened.promise.then(() => opening.dispose());
    opened.resolve(reader);
    await rejected;
    expect(reader.dispose).toHaveBeenCalledTimes(1);
    expect(reader.firstAtOrAfter).not.toHaveBeenCalled();
  });

  it("closes a frame if disposal occurs between acquisition and queue insertion", async () => {
    const decoded = deferred<DecodedFrame>();
    const started = deferred<void>();
    reader.firstAtOrAfter.and.callFake(() => {
      started.resolve();
      return decoded.promise;
    });
    opening = RecordingReadSession.open(
      "front",
      base,
      recordings,
      store,
      async () => reader,
    );
    const initializing = opening.ready;
    const rejected = expectAsync(initializing).toBeRejectedWithError(
      ObsoleteFrameOperation,
    );
    await started.promise;
    void decoded.promise.then(() => opening.dispose());
    const value = frame();
    decoded.resolve(value);
    await rejected;
    expect(value.sample.close).toHaveBeenCalledTimes(1);
    expect(reader.dispose).toHaveBeenCalledTimes(1);
  });

  it("treats late decoding failures as cancellation", async () => {
    const decoded = deferred<DecodedFrame>();
    const started = deferred<void>();
    reader.firstAtOrAfter.and.callFake(() => {
      started.resolve();
      return decoded.promise;
    });
    opening = RecordingReadSession.open(
      "front",
      base,
      recordings,
      store,
      async () => reader,
    );
    const initializing = opening.ready;
    const rejected = expectAsync(initializing).toBeRejectedWithError(
      ObsoleteFrameOperation,
    );
    await started.promise;
    opening.dispose();
    decoded.reject(new Error("Old decoder failed"));
    await rejected;
    expect(recordings.resolveAt).toHaveBeenCalledTimes(1);
    expect(reader.next).not.toHaveBeenCalled();
  });
});
