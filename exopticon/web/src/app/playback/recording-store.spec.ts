import { of, Subject, throwError } from "rxjs";
import { RecordingDescriptor, RecordingService } from "../recording.service";
import { ObsoleteFrameOperation } from "./recording-resource-scope";
import { RecordingStore } from "./recording-store";

function descriptor(fileId: number): RecordingDescriptor {
  return {
    fileId,
    beginTime: "2026-09-23T10:00:00Z",
    endTime: "2026-09-23T10:00:10Z",
    byteLength: 4,
    contentUrl: "/content",
    nextUrl: "/next",
  };
}

describe("RecordingStore", () => {
  let recordings: jasmine.SpyObj<RecordingService>;
  let store: RecordingStore;
  beforeEach(() => {
    recordings = jasmine.createSpyObj("recordings", ["download"]);
    recordings.download.and.callFake((file) =>
      of(new Blob([String(file.fileId)])),
    );
    store = new RecordingStore(recordings);
  });
  afterEach(() => store.dispose());

  it("shares pending downloads and retains the completed blob", async () => {
    const response = new Subject<Blob>();
    recordings.download.and.returnValue(response);
    const pending = store.get(descriptor(1));
    expect(store.get(descriptor(1))).toBe(pending);
    const blob = new Blob(["recording"]);
    response.next(blob);
    response.complete();
    expect(await pending).toBe(blob);
    expect(await store.get(descriptor(1))).toBe(blob);
    expect(recordings.download).toHaveBeenCalledTimes(1);
  });

  it("evicts the least recently used completed recording", async () => {
    const first = await store.get(descriptor(1));
    await store.get(descriptor(2));
    expect(await store.get(descriptor(1))).toBe(first);
    await store.get(descriptor(3));
    expect(await store.get(descriptor(1))).toBe(first);
    await store.get(descriptor(2));
    expect(
      recordings.download.calls.allArgs().map(([file]) => file.fileId),
    ).toEqual([1, 2, 3, 2]);
    expect(await first.text()).toBe("1");
  });

  it("allows a failed download to be retried", async () => {
    recordings.download.and.returnValues(
      throwError(new Error("Download failed")),
      of(new Blob(["ok"])),
    );
    await expectAsync(store.get(descriptor(1))).toBeRejectedWithError(
      "Download failed",
    );
    expect(await (await store.get(descriptor(1))).text()).toBe("ok");
    expect(recordings.download).toHaveBeenCalledTimes(2);
  });

  it("cancels all pending downloads on disposal and refuses new requests", async () => {
    const first = new Subject<Blob>();
    const second = new Subject<Blob>();
    recordings.download.and.returnValues(first, second);
    const firstRejected = expectAsync(
      store.get(descriptor(1)),
    ).toBeRejectedWithError(ObsoleteFrameOperation);
    const secondRejected = expectAsync(
      store.get(descriptor(2)),
    ).toBeRejectedWithError(ObsoleteFrameOperation);
    store.dispose();
    store.dispose();
    await Promise.all([firstRejected, secondRejected]);
    expect(first.observers.length).toBe(0);
    expect(second.observers.length).toBe(0);
    await expectAsync(store.get(descriptor(1))).toBeRejectedWithError(
      ObsoleteFrameOperation,
    );
    expect(recordings.download).toHaveBeenCalledTimes(2);
  });
});
