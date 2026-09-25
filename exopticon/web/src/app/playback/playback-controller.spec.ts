import { HttpErrorResponse } from "@angular/common/http";
import { of, Subject, throwError } from "rxjs";
import { RecordingDescriptor, RecordingService } from "../recording.service";
import { DecodedFrame, FrameReader } from "./mkv-reader";
import { PlaybackClock, PlaybackController } from "./playback-controller";

const descriptor: RecordingDescriptor = {
  fileId: 7,
  beginTime: "2026-09-23T10:00:00Z",
  endTime: "2026-09-23T10:00:10Z",
  byteLength: 4,
  contentUrl: "/content",
  nextUrl: "/next",
};

function frame(time: number): DecodedFrame {
  return {
    utcNanoseconds: BigInt(time) * 1_000_000n,
    utcMilliseconds: time,
    sample: {
      close: jasmine.createSpy("close"),
    } as unknown as DecodedFrame["sample"],
  };
}

function reader(): jasmine.SpyObj<FrameReader> {
  return jasmine.createSpyObj<FrameReader>("reader", [
    "firstAtOrAfter",
    "next",
    "dispose",
  ]);
}

describe("PlaybackController inspection", () => {
  let recordings: jasmine.SpyObj<RecordingService>;
  let currentReader: jasmine.SpyObj<FrameReader>;
  let rendered: Array<DecodedFrame | null>;
  let controller: PlaybackController;

  beforeEach(() => {
    recordings = jasmine.createSpyObj<RecordingService>("recordings", [
      "resolveAt",
      "download",
      "getNext",
    ]);
    recordings.resolveAt.and.returnValue(of(descriptor));
    recordings.download.and.returnValue(of(new Blob(["mkv!"])));
    recordings.getNext.and.returnValue(of({ recording: null }));
    currentReader = reader();
    rendered = [];
    controller = new PlaybackController(
      recordings,
      async () => currentReader,
      (value) => rendered.push(value),
    );
    controller.setCamera("front");
  });
  afterEach(() => controller.destroy());

  it("keeps requested and displayed times distinct, then advances one decoded frame", async () => {
    const first = frame(1200);
    const second = frame(1200);
    currentReader.firstAtOrAfter.and.returnValue(Promise.resolve(first));
    currentReader.next.and.returnValue(Promise.resolve(second));

    await controller.seek(1150);
    expect(controller.state).toEqual({
      phase: "paused",
      requestedTime: 1150,
      displayedTime: 1200,
      error: null,
    });
    expect(rendered[rendered.length - 1]).toBe(first);
    await controller.stepForward();
    expect(currentReader.next).toHaveBeenCalledTimes(1);
    expect(rendered[rendered.length - 1]).toBe(second);
    expect(first.sample.close).toHaveBeenCalledTimes(1);
    expect(controller.state.displayedTime).toBe(1200);
  });

  it("serializes steps and disables another step while one is pending", async () => {
    const first = frame(1200);
    const second = frame(1300);
    currentReader.firstAtOrAfter.and.returnValue(Promise.resolve(first));
    await controller.seek(1100);
    let finish!: (value: DecodedFrame) => void;
    currentReader.next.and.returnValue(
      new Promise((resolve) => (finish = resolve)),
    );
    const pending = controller.stepForward();
    expect(controller.state.phase).toBe("stepping");
    await controller.stepForward();
    expect(currentReader.next).toHaveBeenCalledTimes(1);
    finish(second);
    await pending;
    expect(controller.state.displayedTime).toBe(1300);
    expect(controller.state.requestedTime).toBe(1300);
  });

  it("reuses the Blob on another seek, but opens a fresh reader", async () => {
    const firstReader = currentReader;
    firstReader.firstAtOrAfter.and.returnValue(Promise.resolve(frame(1200)));
    await controller.seek(1100);
    currentReader = reader();
    currentReader.firstAtOrAfter.and.returnValue(Promise.resolve(frame(1500)));
    await controller.seek(1450);
    expect(recordings.resolveAt).toHaveBeenCalledTimes(2);
    expect(recordings.download).toHaveBeenCalledTimes(1);
    expect(firstReader.dispose).toHaveBeenCalledTimes(1);
    expect(controller.state.displayedTime).toBe(1500);
  });

  it("cancels obsolete downloads and rejects stale decoded frames", async () => {
    const download = new Subject<Blob>();
    recordings.download.and.returnValues(download, of(new Blob(["new"])));
    currentReader.firstAtOrAfter.and.returnValue(Promise.resolve(frame(1300)));
    const obsolete = controller.seek(1000);
    await Promise.resolve();
    expect(download.observers.length).toBe(1);
    const current = controller.seek(1200);
    await Promise.all([obsolete, current]);
    expect(download.observers.length).toBe(0);
    expect(controller.state.displayedTime).toBe(1300);

    let finish!: (value: DecodedFrame) => void;
    currentReader.next.and.returnValue(
      new Promise((resolve) => (finish = resolve)),
    );
    const pendingStep = controller.stepForward();
    const stale = frame(1400);
    currentReader = reader();
    currentReader.firstAtOrAfter.and.returnValue(Promise.resolve(frame(1600)));
    const newSeek = controller.seek(1500);
    finish(stale);
    await Promise.all([pendingStep, newSeek]);
    expect(stale.sample.close).toHaveBeenCalledTimes(1);
    expect(controller.state.displayedTime).toBe(1600);
  });

  it("clears the image when the API reports a recording gap", async () => {
    currentReader.firstAtOrAfter.and.returnValue(Promise.resolve(frame(1200)));
    await controller.seek(1100);
    recordings.resolveAt.and.returnValue(
      throwError(
        new HttpErrorResponse({
          status: 404,
          error: { error: "unavailable_time" },
        }),
      ),
    );
    await controller.seek(3000);
    expect(controller.state.phase).toBe("unavailable");
    expect(controller.state.requestedTime).toBe(3000);
    expect(controller.state.displayedTime).toBeNull();
    expect(rendered[rendered.length - 1]).toBeNull();
  });

  it("disposes a reader opened after its seek was canceled", async () => {
    let finishOpen!: (reader: FrameReader) => void;
    const lateReader = reader();
    const freshReader = reader();
    freshReader.firstAtOrAfter.and.resolveTo(frame(1600));
    let opens = 0;
    controller.destroy();
    controller = new PlaybackController(
      recordings,
      () =>
        ++opens === 1
          ? new Promise((resolve) => (finishOpen = resolve))
          : Promise.resolve(freshReader),
      (value) => rendered.push(value),
    );
    controller.setCamera("front");
    const obsolete = controller.seek(1100);
    for (let i = 0; i < 4; i++) await Promise.resolve();
    const current = controller.seek(1500);
    finishOpen(lateReader);
    await Promise.all([obsolete, current]);
    expect(lateReader.dispose).toHaveBeenCalledTimes(1);
    expect(controller.state.displayedTime).toBe(1600);
    expect(rendered).not.toContain(
      jasmine.objectContaining({ utcMilliseconds: 1100 }),
    );
  });
});

describe("PlaybackController continuous playback", () => {
  let recordings: jasmine.SpyObj<RecordingService>;
  let readers: jasmine.SpyObj<FrameReader>[];
  let controller: PlaybackController;
  let rendered: Array<DecodedFrame | null>;
  let now = 0;
  let pendingFrame: (() => void) | undefined;
  const clock: PlaybackClock = {
    now: () => now,
    requestFrame: (callback) => {
      pendingFrame = callback;
      return 1;
    },
    cancelFrame: () => {
      pendingFrame = undefined;
    },
  };
  const nextDescriptor: RecordingDescriptor = {
    ...descriptor,
    fileId: 8,
    beginTime: "2026-09-23T10:00:10Z",
    endTime: "2026-09-23T10:00:20Z",
  };
  const base = Date.parse(descriptor.beginTime);

  async function settle(): Promise<void> {
    for (let i = 0; i < 12; i++) await Promise.resolve();
  }

  function tick(elapsed: number): void {
    now += elapsed;
    const callback = pendingFrame;
    pendingFrame = undefined;
    callback?.();
  }

  beforeEach(() => {
    now = 0;
    pendingFrame = undefined;
    recordings = jasmine.createSpyObj<RecordingService>("recordings", [
      "resolveAt",
      "getNext",
      "download",
    ]);
    recordings.resolveAt.and.returnValue(of(descriptor));
    recordings.getNext.and.returnValue(of({ recording: null }));
    recordings.download.and.returnValue(of(new Blob(["mkv"])));
    readers = [];
    rendered = [];
    controller = new PlaybackController(
      recordings,
      async () => readers.shift()!,
      (value) => rendered.push(value),
      () => {},
      clock,
    );
    controller.setCamera("front");
  });
  afterEach(() => controller.destroy());

  it("presents the latest due frame at speed, then steps to the immediate successor", async () => {
    const first = frame(base);
    const skipped = frame(base + 100);
    const due = frame(base + 200);
    const successor = frame(base + 300);
    const source = reader();
    source.firstAtOrAfter.and.resolveTo(first);
    source.next.and.returnValues(
      Promise.resolve(skipped),
      Promise.resolve(due),
      Promise.resolve(successor),
      new Promise(() => {}),
    );
    readers.push(source);
    await controller.seek(base);
    controller.play();
    await settle();
    controller.setSpeed(2);
    tick(100);
    expect(controller.state.displayedTime).toBe(base + 200);
    expect(skipped.sample.close).toHaveBeenCalledTimes(1);
    controller.pause();
    await controller.stepForward();
    expect(controller.state.displayedTime).toBe(base + 300);
    expect(due.sample.close).toHaveBeenCalledTimes(1);
  });

  it("freezes its clock while decoding is behind", async () => {
    const first = frame(base);
    const second = frame(base + 100);
    const source = reader();
    source.firstAtOrAfter.and.resolveTo(first);
    let deliver!: (value: DecodedFrame) => void;
    source.next.and.returnValue(new Promise((resolve) => (deliver = resolve)));
    readers.push(source);
    await controller.seek(base);
    controller.play();
    expect(controller.state.phase).toBe("buffering");
    now = 10000;
    deliver(second);
    await settle();
    expect(controller.state.phase).toBe("playing");
    expect(controller.state.displayedTime).toBe(base);
    tick(50);
    expect(controller.state.displayedTime).toBe(base);
    tick(50);
    expect(controller.state.displayedTime).toBe(base + 100);
  });

  it("continues into a contiguous recording and skips overlapping UTC footage", async () => {
    const first = frame(base + 9000);
    const duplicate = frame(base + 9000);
    const later = frame(base + 10000);
    const oldReader = reader();
    oldReader.firstAtOrAfter.and.resolveTo(first);
    oldReader.next.and.resolveTo(null);
    const newReader = reader();
    newReader.firstAtOrAfter.and.resolveTo(duplicate);
    newReader.next.and.returnValues(
      Promise.resolve(later),
      new Promise(() => {}),
    );
    readers.push(oldReader, newReader);
    recordings.getNext.and.returnValues(
      of({ recording: nextDescriptor }),
      of({ recording: null }),
    );
    await controller.seek(base + 9000);
    await controller.stepForward();
    expect(controller.state.displayedTime).toBe(base + 10000);
    expect(duplicate.sample.close).toHaveBeenCalledTimes(1);
    expect(oldReader.dispose).toHaveBeenCalledTimes(1);
    expect(recordings.download).toHaveBeenCalledTimes(2);
  });

  it("stops at a gap and jumps only on request", async () => {
    const laterDescriptor = {
      ...nextDescriptor,
      beginTime: "2026-09-23T10:00:12Z",
    };
    const source = reader();
    source.firstAtOrAfter.and.resolveTo(frame(base + 9000));
    source.next.and.resolveTo(null);
    readers.push(source);
    recordings.getNext.and.returnValue(of({ recording: laterDescriptor }));
    await controller.seek(base + 9000);
    await controller.stepForward();
    expect(controller.state.phase).toBe("gap");
    expect(controller.nextRecordingTime).toBe(base + 12000);
    expect(controller.state.displayedTime).toBe(base + 9000);
    expect(recordings.download).toHaveBeenCalledTimes(2);
    const seek = spyOn(controller, "seek").and.resolveTo();
    await controller.jumpToNext();
    expect(seek).toHaveBeenCalledWith(base + 12000);
  });

  it("rounds a microsecond recording start upward when jumping across a gap", async () => {
    const current = { ...descriptor, endTime: "2026-09-23T10:00:12.123000Z" };
    const later = {
      ...nextDescriptor,
      beginTime: "2026-09-23T10:00:12.123456Z",
    };
    const oldReader = reader();
    oldReader.firstAtOrAfter.and.resolveTo(frame(base + 12122));
    oldReader.next.and.resolveTo(null);
    const newReader = reader();
    newReader.firstAtOrAfter.and.resolveTo(frame(base + 12124));
    readers.push(oldReader, newReader);
    recordings.resolveAt.and.callFake((_camera, at) => {
      if (at === base + 12122) return of(current);
      if (at < base + 12123.456)
        return throwError(
          new HttpErrorResponse({
            status: 404,
            error: { error: "unavailable_time" },
          }),
        );
      return of(later);
    });
    recordings.getNext.and.returnValue(of({ recording: later }));
    await controller.seek(base + 12122);
    await controller.stepForward();
    expect(controller.state.phase).toBe("gap");
    expect(controller.nextRecordingTime).toBe(base + 12124);
    await controller.jumpToNext();
    expect(recordings.resolveAt).toHaveBeenCalledWith("front", base + 12124);
    expect(controller.state.phase).toBe("paused");
    expect(controller.state.displayedTime).toBe(base + 12124);
  });

  it("keeps the current frame usable when prefetch fails and retries the transition", async () => {
    const source = reader();
    source.firstAtOrAfter.and.resolveTo(frame(base + 9000));
    source.next.and.resolveTo(null);
    const following = reader();
    following.firstAtOrAfter.and.resolveTo(frame(base + 10000));
    readers.push(source, following);
    recordings.getNext.and.returnValue(of({ recording: nextDescriptor }));
    recordings.download.and.returnValues(
      of(new Blob(["current"])),
      throwError(new Error("Prefetch failed")),
      of(new Blob(["successor"])),
    );
    await controller.seek(base + 9000);
    await settle();
    expect(controller.state.phase).toBe("paused");
    await controller.stepForward();
    expect(controller.state.phase).toBe("error");
    expect(controller.state.displayedTime).toBe(base + 9000);
    expect(controller.canRetryTransition).toBe(true);
    await controller.retryTransition();
    expect(controller.state.phase).toBe("paused");
    expect(controller.state.displayedTime).toBe(base + 10000);
  });

  it("closes displayed, queued, and late decoded frames on camera change", async () => {
    const displayed = frame(base);
    const queued = frame(base + 100);
    const late = frame(base + 200);
    const source = reader();
    source.firstAtOrAfter.and.resolveTo(displayed);
    let finish!: (value: DecodedFrame) => void;
    source.next.and.returnValues(
      Promise.resolve(queued),
      new Promise((resolve) => (finish = resolve)),
    );
    readers.push(source);
    await controller.seek(base);
    controller.play();
    await settle();
    controller.setCamera("back");
    finish(late);
    await settle();
    expect(source.dispose).toHaveBeenCalledTimes(1);
    expect(displayed.sample.close).toHaveBeenCalledTimes(1);
    expect(queued.sample.close).toHaveBeenCalledTimes(1);
    expect(late.sample.close).toHaveBeenCalledTimes(1);
    expect(rendered[rendered.length - 1]).toBeNull();
    expect(controller.state.phase).toBe("idle");
  });

  it("cancels an obsolete prefetch download during a new seek", async () => {
    const oldFrame = frame(base);
    const newFrame = frame(base + 10000);
    const oldReader = reader();
    oldReader.firstAtOrAfter.and.resolveTo(oldFrame);
    const newReader = reader();
    newReader.firstAtOrAfter.and.resolveTo(newFrame);
    readers.push(oldReader, newReader);
    const pendingPrefetch = new Subject<Blob>();
    recordings.resolveAt.and.returnValues(of(descriptor), of(nextDescriptor));
    recordings.getNext.and.returnValues(
      of({ recording: nextDescriptor }),
      of({ recording: null }),
    );
    recordings.download.and.returnValues(
      of(new Blob(["old"])),
      pendingPrefetch,
      of(new Blob(["new"])),
    );
    await controller.seek(base);
    expect(pendingPrefetch.observers.length).toBe(1);
    await controller.seek(base + 10000);
    expect(pendingPrefetch.observers.length).toBe(0);
    expect(oldReader.dispose).toHaveBeenCalledTimes(1);
    expect(oldFrame.sample.close).toHaveBeenCalledTimes(1);
    expect(controller.state.displayedTime).toBe(base + 10000);
  });

  it("closes the retained frame and cancels decoding on destruction", async () => {
    const retained = frame(base);
    const late = frame(base + 100);
    const source = reader();
    source.firstAtOrAfter.and.resolveTo(retained);
    let finish!: (value: DecodedFrame) => void;
    source.next.and.returnValue(new Promise((resolve) => (finish = resolve)));
    readers.push(source);
    await controller.seek(base);
    controller.play();
    controller.destroy();
    finish(late);
    await settle();
    expect(source.dispose).toHaveBeenCalledTimes(1);
    expect(retained.sample.close).toHaveBeenCalledTimes(1);
    expect(late.sample.close).toHaveBeenCalledTimes(1);
    expect(rendered[rendered.length - 1]).toBeNull();
  });
});
