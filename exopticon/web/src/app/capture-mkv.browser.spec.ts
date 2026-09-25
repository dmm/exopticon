import { BlobSource, Input, MATROSKA, VideoSampleSink } from "mediabunny";

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
});
