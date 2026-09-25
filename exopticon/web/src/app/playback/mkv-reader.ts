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

interface CaptureMetadata {
  captureSessionId: string;
  clockAnchor: { pipelineRunningTimeNs: string; utcTime: string };
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

function parseMetadata(raw: unknown): CaptureMetadata {
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
  ) as CaptureMetadata;
  if (
    !/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(
      parsed.captureSessionId,
    ) ||
    !/^\d+$/.test(parsed.clockAnchor?.pipelineRunningTimeNs ?? "") ||
    typeof parsed.clockAnchor?.utcTime !== "string"
  ) {
    throw new Error("Invalid capture timeline metadata");
  }
  parseUtcNanoseconds(parsed.clockAnchor.utcTime);
  return parsed;
}

export class MkvReader implements FrameReader {
  readonly captureSessionId: string;
  private iterator?: AsyncGenerator<VideoSample, void, unknown>;
  private disposed = false;

  private constructor(
    private input: Input<BlobSource>,
    private sink: VideoSampleSink,
    metadata: CaptureMetadata,
    private anchorUtcNanoseconds: bigint,
  ) {
    this.captureSessionId = metadata.captureSessionId;
    this.anchorRunningNanoseconds = BigInt(
      metadata.clockAnchor.pipelineRunningTimeNs,
    );
  }

  private anchorRunningNanoseconds: bigint;

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
      const metadata = parseMetadata(
        (await input.getMetadataTags()).raw?.["COMMENTS"],
      );
      return new MkvReader(
        input,
        new VideoSampleSink(track),
        metadata,
        parseUtcNanoseconds(metadata.clockAnchor.utcTime),
      );
    } catch (error) {
      input.dispose();
      throw error;
    }
  }

  async firstAtOrAfter(utcMilliseconds: number): Promise<DecodedFrame | null> {
    await this.closeIterator();
    if (this.disposed) return null;
    const target = BigInt(Math.trunc(utcMilliseconds)) * 1_000_000n;
    const sessionNanoseconds =
      target - this.anchorUtcNanoseconds + this.anchorRunningNanoseconds;
    this.iterator = this.sink.samples(
      Math.max(0, Number(sessionNanoseconds) / 1e9),
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
    // Capture files use a 1 ms Matroska timecode scale. Round at that scale
    // before converting back to integer nanoseconds.
    const sessionNanoseconds =
      BigInt(Math.round(sample.timestamp * 1000)) * 1_000_000n;
    const utcNanoseconds =
      this.anchorUtcNanoseconds +
      sessionNanoseconds -
      this.anchorRunningNanoseconds;
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
