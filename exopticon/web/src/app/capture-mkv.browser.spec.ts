import { BlobSource, Input, MATROSKA, VideoSampleSink } from "mediabunny";
import { of } from "rxjs";
import { RecordingDescriptor, RecordingService } from "./recording.service";
import { DecodedFrame, FrameReader, MkvReader } from "./playback/mkv-reader";
import { PlaybackController } from "./playback/playback-controller";

interface CaptureTimeline {
  captureSessionId: string;
  clockAnchor: { pipelineRunningTimeNs: number; utcTime: string };
}

async function openFixture(name: string): Promise<Input<BlobSource>> {
  const response = await fetch(`/base/test-fixtures/${name}.mkv`);
  if (!response.ok) {
    throw new Error(`Missing MKV fixture: ${name}`);
  }
  return new Input({
    formats: [MATROSKA],
    source: new BlobSource(await response.blob()),
  });
}

function parseGstreamerComments(raw: unknown): CaptureTimeline {
  if (typeof raw !== "string") {
    throw new Error("Capture COMMENTS tag is missing");
  }
  // GStreamer's matroskamux stores the Comment tag as a quoted, escaped
  // GstStructure string. Record this actual representation in the browser test.
  const json = raw.replace(/^"|"$/g, "").replace(/\\(.)/g, "$1");
  return JSON.parse(json) as CaptureTimeline;
}

describe("GStreamer capture MKV browser contract", () => {
  it("preserves distinct frames with equal timestamps and variable intervals", async () => {
    const response = await fetch("/base/test-fixtures/capture-irregular.mkv");
    expect(response.ok).toBe(true);
    const reader = await MkvReader.open(await response.blob());
    const base = BigInt(Date.parse("2026-09-23T10:00:04Z")) * 1_000_000n;
    try {
      const first = await reader.firstAtOrAfter(
        Number(base / 1_000_000n) + 150,
      );
      expect(first?.utcNanoseconds).toBe(base + 223_456_789n);
      first?.sample.close();
      const timestamps: bigint[] = [];
      for (let index = 0; index < 6; index++) {
        const next = await reader.next();
        expect(next).not.toBeNull();
        if (!next) break;
        timestamps.push(next.utcNanoseconds - base);
        next.sample.close();
      }
      expect(timestamps).toEqual([
        223_456_789n,
        373_456_789n,
        423_456_789n,
        623_456_789n,
        623_456_789n,
        823_456_789n,
      ]);
    } finally {
      reader.dispose();
      expect(reader.isDisposed).toBe(true);
    }
  });

  it("maps the nanosecond anchor and seeks to the first frame at or after UTC", async () => {
    const response = await fetch("/base/test-fixtures/capture-session-a.mkv");
    expect(response.ok).toBe(true);
    const reader = await MkvReader.open(await response.blob());
    try {
      expect(reader.captureSessionId).toBe(
        "11111111-1111-4111-8111-111111111111",
      );
      const first = await reader.firstAtOrAfter(
        Date.parse("2026-09-23T10:00:00.273Z"),
      );
      expect(first?.utcNanoseconds).toBe(
        BigInt(Date.parse("2026-09-23T10:00:00Z")) * 1_000_000n + 323_456_789n,
      );
      first?.sample.close();
      const next = await reader.next();
      expect(next?.utcNanoseconds).toBe(
        BigInt(Date.parse("2026-09-23T10:00:00Z")) * 1_000_000n + 423_456_789n,
      );
      next?.sample.close();
    } finally {
      reader.dispose();
    }
  });

  it("reads COMMENTS, nonzero timestamps, B-frames, and exact presentation order", async () => {
    const input = await openFixture("capture-session-a");
    try {
      const timeline = parseGstreamerComments(
        (await input.getMetadataTags()).raw?.["COMMENTS"],
      );
      expect(timeline.captureSessionId).toBe(
        "11111111-1111-4111-8111-111111111111",
      );
      expect(timeline.clockAnchor.pipelineRunningTimeNs).toBe(5_000_000_000);
      expect(timeline.clockAnchor.utcTime).toBe(
        "2026-09-23T10:00:00.123456789Z",
      );

      const track = await input.getPrimaryVideoTrack();
      expect(track).not.toBeNull();
      if (!track) return;
      expect(await track.canDecode()).toBe(true);
      expect(await track.getFirstTimestamp()).toBeCloseTo(5, 3);
      const sink = new VideoSampleSink(track);
      const earlier = await sink.getSample(5.15);
      expect(earlier?.timestamp).toBeCloseTo(5.1, 3);
      earlier?.close();
      const iterator = sink.samples(5.15);
      try {
        const timestamps: number[] = [];
        let discarded = 0;
        while (timestamps.length < 4) {
          const result = await iterator.next();
          expect(result.done).toBe(false);
          if (result.done) break;
          if (result.value.timestamp < 5.15) {
            discarded++;
          } else {
            timestamps.push(result.value.timestamp);
          }
          result.value.close();
        }
        expect(discarded).toBe(1);
        expect(timestamps).toEqual([5.2, 5.3, 5.4, 5.5]);
      } finally {
        await iterator.return();
      }
    } finally {
      input.dispose();
    }
  });

  it("opens a restarted session and a changed decoder configuration", async () => {
    const first = await openFixture("capture-session-a");
    const second = await openFixture("capture-session-b");
    try {
      const firstTrack = await first.getPrimaryVideoTrack();
      const secondTrack = await second.getPrimaryVideoTrack();
      expect(firstTrack).not.toBeNull();
      expect(secondTrack).not.toBeNull();
      if (!firstTrack || !secondTrack) return;
      expect(await firstTrack.getCodedWidth()).toBe(160);
      expect(await secondTrack.getCodedWidth()).toBe(128);
      expect(await secondTrack.canDecode()).toBe(true);
      expect(await secondTrack.getFirstTimestamp()).toBeCloseTo(1, 3);
      const timeline = parseGstreamerComments(
        (await second.getMetadataTags()).raw?.["COMMENTS"],
      );
      expect(timeline.captureSessionId).not.toBe(
        parseGstreamerComments(
          (await first.getMetadataTags()).raw?.["COMMENTS"],
        ).captureSessionId,
      );
      const iterator = new VideoSampleSink(secondTrack).samples(1);
      try {
        const result = await iterator.next();
        expect(result.done).toBe(false);
        if (!result.done) {
          expect(result.value.timestamp).toBeCloseTo(1, 3);
          result.value.close();
        }
      } finally {
        await iterator.return();
      }
    } finally {
      first.dispose();
      second.dispose();
    }
  });

  it("releases repeated decode sequences", async () => {
    for (let attempt = 0; attempt < 5; attempt++) {
      const input = await openFixture("capture-session-a");
      try {
        const track = await input.getPrimaryVideoTrack();
        expect(track).not.toBeNull();
        if (!track) continue;
        const iterator = new VideoSampleSink(track).samples(5.25);
        try {
          const result = await iterator.next();
          expect(result.done).toBe(false);
          if (!result.done) result.value.close();
        } finally {
          await iterator.return();
        }
      } finally {
        input.dispose();
        expect(input.disposed).toBe(true);
      }
    }
  });

  it("disposes the adapter input after repeated seeks", async () => {
    const response = await fetch("/base/test-fixtures/capture-session-a.mkv");
    const blob = await response.blob();
    for (let attempt = 0; attempt < 5; attempt++) {
      const reader = await MkvReader.open(blob);
      try {
        const frame = await reader.firstAtOrAfter(
          Date.parse("2026-09-23T10:00:00.273Z"),
        );
        expect(frame).not.toBeNull();
        frame?.sample.close();
      } finally {
        reader.dispose();
        expect(reader.isDisposed).toBe(true);
        expect(await reader.next()).toBeNull();
      }
    }
  });

  it("discards a real decoded frame that arrives after a replacement seek", async () => {
    const response = await fetch("/base/test-fixtures/capture-session-a.mkv");
    const blob = await response.blob();
    const begin = Date.parse("2026-09-23T10:00:00Z");
    const descriptor: RecordingDescriptor = {
      fileId: 1,
      beginTime: new Date(begin).toISOString(),
      endTime: new Date(begin + 2000).toISOString(),
      byteLength: blob.size,
      contentUrl: "/fixture/content",
      nextUrl: "/fixture/next",
    };
    const recordings = jasmine.createSpyObj<RecordingService>("recordings", [
      "resolveAt",
      "download",
      "getNext",
    ]);
    recordings.resolveAt.and.returnValue(of(descriptor));
    recordings.download.and.returnValue(of(blob));
    recordings.getNext.and.returnValue(of({ recording: null }));
    let release!: () => void;
    const held = new Promise<void>((resolve) => (release = resolve));
    let stale: DecodedFrame | null = null;
    let staleClose: jasmine.Spy | undefined;
    let opens = 0;
    const displayed: Array<DecodedFrame | null> = [];
    const controller = new PlaybackController(
      recordings,
      async (content): Promise<FrameReader> => {
        const reader = await MkvReader.open(content);
        if (++opens !== 1) return reader;
        return {
          firstAtOrAfter: async (time) => {
            stale = await reader.firstAtOrAfter(time);
            if (stale)
              staleClose = spyOn(stale.sample, "close").and.callThrough();
            await held;
            return stale;
          },
          next: () => reader.next(),
          dispose: () => reader.dispose(),
        };
      },
      (frame) => displayed.push(frame),
    );
    controller.setCamera("front");
    try {
      const first = controller.seek(begin + 273);
      for (let attempt = 0; attempt < 50 && !stale; attempt++)
        await new Promise((resolve) => setTimeout(resolve, 10));
      expect(stale).not.toBeNull();
      const second = controller.seek(begin + 473);
      await second;
      release();
      await first;
      expect(staleClose).toHaveBeenCalledTimes(1);
      expect(displayed).not.toContain(stale);
      expect(controller.state.displayedTime).toBe(begin + 523);
    } finally {
      release();
      controller.destroy();
    }
  });

  it("crosses a recording gap into a restarted session with a new decoder configuration", async () => {
    const responses = await Promise.all([
      fetch("/base/test-fixtures/capture-session-a.mkv"),
      fetch("/base/test-fixtures/capture-session-b.mkv"),
    ]);
    const [firstBlob, secondBlob] = await Promise.all(
      responses.map((response) => response.blob()),
    );
    const begin = Date.parse("2026-09-23T10:00:00Z");
    const first: RecordingDescriptor = {
      fileId: 1,
      beginTime: new Date(begin).toISOString(),
      endTime: new Date(begin + 1823).toISOString(),
      byteLength: firstBlob.size,
      contentUrl: "/fixture/a",
      nextUrl: "/fixture/next-a",
    };
    const second: RecordingDescriptor = {
      ...first,
      fileId: 2,
      beginTime: new Date(begin + 2123).toISOString(),
      endTime: new Date(begin + 3823).toISOString(),
      byteLength: secondBlob.size,
      contentUrl: "/fixture/b",
      nextUrl: "/fixture/next-b",
    };
    const recordings = jasmine.createSpyObj<RecordingService>("recordings", [
      "resolveAt",
      "download",
      "getNext",
    ]);
    recordings.resolveAt.and.callFake((_camera, at) =>
      of(at >= begin + 2123 ? second : first),
    );
    recordings.download.and.callFake((descriptor) =>
      of(descriptor.fileId === 1 ? firstBlob : secondBlob),
    );
    recordings.getNext.and.callFake((descriptor) =>
      of({ recording: descriptor.fileId === 1 ? second : null }),
    );
    const displayed: DecodedFrame[] = [];
    const controller = new PlaybackController(
      recordings,
      MkvReader.open,
      (frame) => {
        if (frame) displayed.push(frame);
      },
    );
    controller.setCamera("front");
    try {
      await controller.seek(begin + 1723);
      expect(controller.state.displayedTime).toBe(begin + 1723);
      await controller.stepForward();
      expect(controller.state.displayedTime).toBe(begin + 1823);
      await controller.stepForward();
      expect(controller.state.phase).toBe("gap");
      expect(controller.nextRecordingTime).toBe(begin + 2123);
      await controller.jumpToNext();
      expect(controller.state.displayedTime).toBe(begin + 2123);
      expect(displayed[displayed.length - 1].sample.displayWidth).toBe(128);
      expect(recordings.download).toHaveBeenCalledTimes(2);
    } finally {
      controller.destroy();
    }
  });
});
