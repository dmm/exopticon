import { of, Subject } from "rxjs";
import { RecordingDescriptor, RecordingService } from "../recording.service";
import { DecodedFrame, FrameReader } from "./mkv-reader";
import {
  ObsoleteFrameOperation,
  RecordingFrameSource,
} from "./recording-frame-source";

const descriptor: RecordingDescriptor = {
  fileId: 1,
  beginTime: "2026-09-23T10:00:00Z",
  endTime: "2026-09-23T10:00:10Z",
  byteLength: 4,
  contentUrl: "/content",
  nextUrl: "/next",
};
const base = Date.parse(descriptor.beginTime);
function frame(offset: number): DecodedFrame {
  return {
    utcMilliseconds: base + offset,
    utcNanoseconds: BigInt(base + offset) * 1_000_000n,
    sample: {
      close: jasmine.createSpy("close"),
    } as unknown as DecodedFrame["sample"],
  };
}

describe("RecordingFrameSource", () => {
  let recordings: jasmine.SpyObj<RecordingService>;
  let reader: jasmine.SpyObj<FrameReader>;
  let source: RecordingFrameSource;
  beforeEach(() => {
    recordings = jasmine.createSpyObj("recordings", [
      "resolveAt",
      "download",
      "getNext",
    ]);
    recordings.resolveAt.and.returnValue(of(descriptor));
    recordings.download.and.returnValue(of(new Blob(["mkv"])));
    recordings.getNext.and.returnValue(of({ recording: null }));
    reader = jasmine.createSpyObj("reader", [
      "firstAtOrAfter",
      "next",
      "dispose",
    ]);
    source = new RecordingFrameSource("front", recordings, async () => reader);
  });
  afterEach(() => source.dispose());

  it("cancels a store-owned prefetch when the source is disposed", async () => {
    const pending = new Subject<Blob>();
    reader.firstAtOrAfter.and.resolveTo(frame(0));
    recordings.getNext.and.returnValue(
      of({ recording: { ...descriptor, fileId: 2 } }),
    );
    recordings.download.and.returnValues(of(new Blob(["current"])), pending);
    await source.seek(base);
    expect(pending.observers.length).toBe(1);
    source.dispose();
    expect(pending.observers.length).toBe(0);
    await expectAsync(source.seek(base)).toBeRejectedWithError(
      ObsoleteFrameOperation,
    );
    expect(recordings.download).toHaveBeenCalledTimes(2);
  });

  it("bounds buffering and closes only frames it still owns", async () => {
    const first = frame(0);
    const queued = Array.from({ length: 6 }, (_, i) => frame((i + 1) * 100));
    reader.firstAtOrAfter.and.resolveTo(first);
    reader.next.and.returnValues(
      ...queued.map((value) => Promise.resolve(value)),
    );
    expect(await source.seek(base)).toEqual({ kind: "ready" });
    expect(source.takeFrame()).toBe(first);
    for (let i = 0; i < 10; i++) {
      await source.buffer();
    }
    expect(reader.next).toHaveBeenCalledTimes(6);
    expect(source.canBuffer).toBe(false);
    expect(source.nextFrameTime).toBe(base + 100);
    source.dispose();
    source.dispose();
    expect(first.sample.close).not.toHaveBeenCalled();
    for (const value of queued) {
      expect(value.sample.close).toHaveBeenCalledTimes(1);
    }
    expect(reader.dispose).toHaveBeenCalledTimes(1);
    first.sample.close();
  });

  it("shares pending decoding and rejects and closes late frames after cancellation", async () => {
    const first = frame(0);
    reader.firstAtOrAfter.and.resolveTo(first);
    await source.seek(base);
    let deliver!: (value: DecodedFrame) => void;
    reader.next.and.returnValue(new Promise((resolve) => (deliver = resolve)));
    const pending = source.buffer();
    expect(source.buffer()).toBe(pending);
    expect(reader.next).toHaveBeenCalledTimes(1);
    const rejected = expectAsync(pending).toBeRejectedWithError(
      ObsoleteFrameOperation,
    );
    source.dispose();
    const late = frame(100);
    deliver(late);
    await rejected;
    expect(first.sample.close).toHaveBeenCalledTimes(1);
    expect(late.sample.close).toHaveBeenCalledTimes(1);
    expect(source.hasFrames).toBe(false);
  });

  it("reports gaps without transferring or closing the caller's last frame", async () => {
    const first = frame(9000);
    reader.firstAtOrAfter.and.resolveTo(first);
    reader.next.and.resolveTo(null);
    recordings.getNext.and.returnValue(
      of({
        recording: {
          ...descriptor,
          fileId: 2,
          beginTime: "2026-09-23T10:00:12.123456Z",
        },
      }),
    );
    await source.seek(base + 9000);
    source.takeFrame();
    expect(await source.buffer()).toEqual({
      kind: "gap",
      nextRecordingTime: base + 12124,
    });
    expect(source.canBuffer).toBe(false);
    expect(source.hasFrames).toBe(false);
    source.dispose();
    expect(first.sample.close).not.toHaveBeenCalled();
    first.sample.close();
  });

  it("downloads an oversized successor only when it is needed", async () => {
    const first = frame(9000);
    const next = frame(10000);
    reader.firstAtOrAfter.and.returnValues(
      Promise.resolve(first),
      Promise.resolve(next),
    );
    reader.next.and.resolveTo(null);
    recordings.getNext.and.returnValues(
      of({
        recording: {
          ...descriptor,
          fileId: 2,
          beginTime: descriptor.endTime,
          endTime: "2026-09-23T10:00:20Z",
          byteLength: 33 * 1024 * 1024,
        },
      }),
      of({ recording: null }),
    );
    await source.seek(base + 9000);
    expect(recordings.download).toHaveBeenCalledTimes(1);
    source.takeFrame();
    expect(await source.buffer()).toEqual({ kind: "ready" });
    expect(recordings.download).toHaveBeenCalledTimes(2);
    expect(source.nextFrameTime).toBe(base + 10000);
    first.sample.close();
  });
});
