import { HttpErrorResponse } from "@angular/common/http";
import { of, Subject, throwError } from "rxjs";
import { RecordingDescriptor, RecordingService } from "../recording.service";
import { DecodedFrame, FrameReader } from "./mkv-reader";
import { PlaybackController } from "./playback-controller";

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
    ]);
    recordings.resolveAt.and.returnValue(of(descriptor));
    recordings.download.and.returnValue(of(new Blob(["mkv!"])));
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
});
