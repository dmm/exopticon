import {
  BlobSource,
  Input,
  MATROSKA,
  VideoSample,
  VideoSampleSink,
} from "mediabunny";

export interface DecodedFrame {
  sample: VideoSample;
  utcNanoseconds: bigint;
  utcMilliseconds: number;
}

export interface FrameReader {
  firstAtOrAfter(utcMilliseconds: number): Promise<DecodedFrame | null>;
  next(): Promise<DecodedFrame | null>;
  dispose(): void;
}

export interface CaptureTimelineMetadata {
  captureSessionId: string;
  anchorRunningNanoseconds: bigint;
  anchorUtcNanoseconds: bigint;
}

interface RawCaptureMetadata {
  captureSessionId: string;
  clockAnchor: { pipelineRunningTimeNs: string; utcTime: string };
}

export function parseRecordingTimeNanoseconds(value: string): bigint {
  const match =
    /^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d{1,9}))?(Z|[+-]\d\d:\d\d)$/.exec(
      value,
    );
  if (!match) throw new Error("Invalid recording interval time");
  const seconds = Date.parse(`${match[1]}${match[3]}`);
  if (!Number.isFinite(seconds))
    throw new Error("Invalid recording interval time");
  return BigInt(seconds) * 1_000_000n + BigInt((match[2] ?? "").padEnd(9, "0"));
}

function parseUtcNanoseconds(value: string): bigint {
  const match = /^(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d)(?:\.(\d{1,9}))?Z$/.exec(
    value,
  );
  if (!match) throw new Error("Invalid capture clock UTC time");
  const seconds = Date.parse(`${match[1]}Z`);
  if (!Number.isFinite(seconds))
    throw new Error("Invalid capture clock UTC time");
  return BigInt(seconds) * 1_000_000n + BigInt((match[2] ?? "").padEnd(9, "0"));
}

export function parseCaptureMetadata(raw: unknown): CaptureTimelineMetadata {
  if (typeof raw !== "string") throw new Error("Missing capture COMMENTS tag");
  // GStreamer matroskamux quotes and escapes the JSON Comment tag.
  const json = raw.startsWith("{")
    ? raw
    : raw.replace(/^"|"$/g, "").replace(/\\(.)/g, "$1");
  // JSON.parse would round a u64 nanosecond count before BigInt sees it.
  const number = /"pipelineRunningTimeNs"\s*:\s*(\d+)/.exec(json);
  if (!number) throw new Error("Missing capture clock running time");
  const parsed = JSON.parse(
    json.replace(number[0], `"pipelineRunningTimeNs":"${number[1]}"`),
  ) as RawCaptureMetadata;
  if (
    !/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(
      parsed.captureSessionId,
    ) ||
    !/^\d+$/.test(parsed.clockAnchor?.pipelineRunningTimeNs ?? "") ||
    typeof parsed.clockAnchor?.utcTime !== "string"
  ) {
    throw new Error("Invalid capture timeline metadata");
  }
  return {
    captureSessionId: parsed.captureSessionId,
    anchorRunningNanoseconds: BigInt(parsed.clockAnchor.pipelineRunningTimeNs),
    anchorUtcNanoseconds: parseUtcNanoseconds(parsed.clockAnchor.utcTime),
  };
}

export function sessionSecondsToUtcNanoseconds(
  metadata: CaptureTimelineMetadata,
  sessionSeconds: number,
): bigint {
  // Capture files use a 1 ms Matroska timecode scale. Round at that scale
  // before converting back to integer nanoseconds.
  const sessionNanoseconds =
    BigInt(Math.round(sessionSeconds * 1000)) * 1_000_000n;
  return (
    metadata.anchorUtcNanoseconds +
    sessionNanoseconds -
    metadata.anchorRunningNanoseconds
  );
}

export function utcNanosecondsToSessionSeconds(
  metadata: CaptureTimelineMetadata,
  utcNanoseconds: bigint,
): number {
  const sessionNanoseconds =
    utcNanoseconds -
    metadata.anchorUtcNanoseconds +
    metadata.anchorRunningNanoseconds;
  return Math.max(0, Number(sessionNanoseconds) / 1e9);
}

export class MkvReader implements FrameReader {
  readonly captureSessionId: string;
  private iterator?: AsyncGenerator<VideoSample, void, unknown>;
  private disposed = false;

  private constructor(
    private input: Input<BlobSource>,
    private sink: VideoSampleSink,
    private metadata: CaptureTimelineMetadata,
  ) {
    this.captureSessionId = metadata.captureSessionId;
  }

  get isDisposed(): boolean {
    return this.input.disposed;
  }

  static async open(blob: Blob): Promise<MkvReader> {
    const input = new Input({
      formats: [MATROSKA],
      source: new BlobSource(blob),
    });
    try {
      if (!(await input.canRead()))
        throw new Error("Unsupported Matroska file");
      const track = await input.getPrimaryVideoTrack();
      if (!track) throw new Error("Recording has no video track");
      if (!(await track.canDecode()))
        throw new Error("Browser cannot decode this recording");
      const metadata = parseCaptureMetadata(
        (await input.getMetadataTags()).raw?.["COMMENTS"],
      );
      return new MkvReader(input, new VideoSampleSink(track), metadata);
    } catch (error) {
      input.dispose();
      throw error;
    }
  }

  async firstAtOrAfter(utcMilliseconds: number): Promise<DecodedFrame | null> {
    await this.closeIterator();
    if (this.disposed) return null;
    const target = BigInt(Math.trunc(utcMilliseconds)) * 1_000_000n;
    this.iterator = this.sink.samples(
      utcNanosecondsToSessionSeconds(this.metadata, target),
    );
    while (true) {
      const frame = await this.next();
      if (!frame || frame.utcNanoseconds >= target) return frame;
      frame.sample.close();
    }
  }

  async next(): Promise<DecodedFrame | null> {
    if (this.disposed || !this.iterator) return null;
    const iterator = this.iterator;
    const result = await iterator.next();
    if (result.done) return null;
    const sample = result.value;
    if (this.disposed || iterator !== this.iterator) {
      sample.close();
      return null;
    }
    const utcNanoseconds = sessionSecondsToUtcNanoseconds(
      this.metadata,
      sample.timestamp,
    );
    return {
      sample,
      utcNanoseconds,
      utcMilliseconds: Number(utcNanoseconds / 1_000_000n),
    };
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.input.dispose();
    void this.closeIterator();
  }

  private async closeIterator(): Promise<void> {
    const iterator = this.iterator;
    this.iterator = undefined;
    if (iterator) {
      try {
        await iterator.return();
      } catch {
        // Input disposal can cancel a pending iterator read.
      }
    }
  }
}
