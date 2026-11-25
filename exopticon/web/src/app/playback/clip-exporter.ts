import { HttpErrorResponse } from "@angular/common/http";
import {
  AudioCodec,
  BlobSource,
  BufferTarget,
  EncodedAudioPacketSource,
  EncodedPacket,
  EncodedPacketSink,
  EncodedVideoPacketSource,
  Input,
  InputAudioTrack,
  InputVideoTrack,
  MATROSKA,
  Mp4OutputFormat,
  Output,
  VideoCodec,
} from "mediabunny";
import { from } from "rxjs";
import { RecordingDescriptor, RecordingService } from "../recording.service";
import {
  CaptureTimelineMetadata,
  parseCaptureMetadata,
  parseRecordingTimeNanoseconds,
  sessionSecondsToUtcNanoseconds,
  utcNanosecondsToSessionSeconds,
} from "./mkv-reader";
import {
  ObsoleteFrameOperation,
  RecordingResourceScope,
} from "./recording-resource-scope";
import { RecordingStore } from "./recording-store";

export const MAX_CLIP_DURATION_MS = 5 * 60 * 1000;

const NANOSECONDS_PER_MILLISECOND = 1_000_000n;
const MAX_FRAGMENTS_PER_CLIP = 200;
const MAX_VIDEO_REORDER_BUFFER = 16;

export class ClipExportError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ClipExportError";
  }
}

export class ClipExportCancelledError extends Error {
  constructor(message = "Clip export cancelled.") {
    super(message);
    this.name = "ClipExportCancelledError";
  }
}

export type ClipExportStage =
  "resolving" | "downloading" | "muxing" | "finalizing";

export interface ClipExportProgress {
  stage: ClipExportStage;
  completedFragments: number;
  totalFragments: number;
}

export interface ClipExportOptions {
  cameraName: string;
  startTimeMs: number;
  endTimeMs: number;
  recordings: RecordingService;
  store?: RecordingStore;
  signal?: AbortSignal;
  onProgress?: (progress: ClipExportProgress) => void;
}

export interface ClipExportResult {
  blob: Blob;
  filename: string;
  actualStartMs: number;
  actualEndMs: number;
  videoPacketCount: number;
  audioPacketCount: number;
  audioOmittedReason?: string;
}

interface VideoStreamSignature {
  codec: VideoCodec;
  codedWidth: number;
  codedHeight: number;
  decoderCodec: string;
  description: Uint8Array | null;
}

interface AudioStreamSignature {
  codec: AudioCodec;
  sampleRate: number;
  numberOfChannels: number;
  decoderCodec: string;
  description: Uint8Array | null;
}

type AudioMode =
  | { kind: "none" }
  | { kind: "omitted"; reason: string }
  | {
      kind: "included";
      signature: AudioStreamSignature;
      decoderConfig: AudioDecoderConfig;
      source: EncodedAudioPacketSource;
    };

export function buildClipFilename(
  cameraName: string,
  startTimeMs: number,
  endTimeMs: number,
): string {
  const startIso = new Date(startTimeMs).toISOString();
  const endIso = new Date(endTimeMs).toISOString();
  return `${cameraName}-${startIso}-${endIso}.mp4`;
}

function toByteArray(
  source: AllowSharedBufferSource | undefined,
): Uint8Array | null {
  if (!source) return null;
  if (source instanceof Uint8Array) {
    return new Uint8Array(source.buffer, source.byteOffset, source.byteLength);
  }
  if (ArrayBuffer.isView(source)) {
    return new Uint8Array(source.buffer, source.byteOffset, source.byteLength);
  }
  return new Uint8Array(source);
}

function bytesEqual(a: Uint8Array | null, b: Uint8Array | null): boolean {
  if (a === b) return true;
  if (!a || !b || a.byteLength !== b.byteLength) return false;
  for (let i = 0; i < a.byteLength; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

function videoSignaturesMatch(
  expected: VideoStreamSignature,
  actual: VideoStreamSignature,
): boolean {
  return (
    expected.codec === actual.codec &&
    expected.codedWidth === actual.codedWidth &&
    expected.codedHeight === actual.codedHeight &&
    expected.decoderCodec === actual.decoderCodec &&
    bytesEqual(expected.description, actual.description)
  );
}

function audioSignaturesMatch(
  expected: AudioStreamSignature,
  actual: AudioStreamSignature,
): boolean {
  return (
    expected.codec === actual.codec &&
    expected.sampleRate === actual.sampleRate &&
    expected.numberOfChannels === actual.numberOfChannels &&
    expected.decoderCodec === actual.decoderCodec &&
    bytesEqual(expected.description, actual.description)
  );
}

function assertNotAborted(signal?: AbortSignal): void {
  if (signal?.aborted) {
    throw new ClipExportCancelledError();
  }
}

async function resolveContinuousFragments(
  cameraName: string,
  startTimeMs: number,
  startNs: bigint,
  endNs: bigint,
  recordings: RecordingService,
  scope: RecordingResourceScope,
  signal?: AbortSignal,
): Promise<RecordingDescriptor[]> {
  let first: RecordingDescriptor;
  try {
    first = await scope.fetch(() =>
      recordings.resolveAt(cameraName, startTimeMs),
    );
  } catch (error) {
    if (signal?.aborted || error instanceof ObsoleteFrameOperation) {
      throw new ClipExportCancelledError();
    }
    if (
      error instanceof HttpErrorResponse &&
      error.status === 404 &&
      error.error?.error === "unavailable_time"
    ) {
      throw new ClipExportError(
        "No finalized recording is available at the selected start time.",
      );
    }
    throw error;
  }

  const firstBeginNs = parseRecordingTimeNanoseconds(first.beginTime);
  if (firstBeginNs > startNs) {
    throw new ClipExportError(
      "Selected range begins before the available recording.",
    );
  }

  const fragments: RecordingDescriptor[] = [first];
  let current = first;
  let currentEndNs = parseRecordingTimeNanoseconds(current.endTime);

  while (currentEndNs < endNs) {
    assertNotAborted(signal);
    if (fragments.length >= MAX_FRAGMENTS_PER_CLIP) {
      throw new ClipExportError(
        "Selected range spans too many recording fragments to export.",
      );
    }
    const nextResponse = await scope.fetch(() => recordings.getNext(current));
    const next = nextResponse.recording;
    if (!next) {
      throw new ClipExportError(
        "Selected range extends past the end of available finalized recordings.",
      );
    }
    if (next.fileId === current.fileId) {
      throw new ClipExportError(
        "Recording sequence did not advance across fragments.",
      );
    }
    const nextBeginNs = parseRecordingTimeNanoseconds(next.beginTime);
    if (nextBeginNs > currentEndNs) {
      throw new ClipExportError(
        "Selected range contains a recording gap. Choose a continuous range without gaps.",
      );
    }
    const nextEndNs = parseRecordingTimeNanoseconds(next.endTime);
    if (nextEndNs <= currentEndNs) {
      throw new ClipExportError(
        "Selected range contains overlapping recordings. Choose a range without overlapping capture sessions.",
      );
    }
    fragments.push(next);
    current = next;
    currentEndNs = nextEndNs;
  }

  return fragments;
}

async function readVideoSignature(
  track: InputVideoTrack,
  supportedCodecs: VideoCodec[],
): Promise<{
  signature: VideoStreamSignature;
  decoderConfig: VideoDecoderConfig;
}> {
  if (!track.codec || !supportedCodecs.includes(track.codec)) {
    throw new ClipExportError(
      `Unsupported video codec for MP4 export: ${track.codec ?? "unknown"}.`,
    );
  }
  const decoderConfig = await track.getDecoderConfig();
  if (!decoderConfig) {
    throw new ClipExportError("Recording video track is missing decoder info.");
  }
  return {
    signature: {
      codec: track.codec,
      codedWidth: track.codedWidth,
      codedHeight: track.codedHeight,
      decoderCodec: decoderConfig.codec,
      description: toByteArray(decoderConfig.description),
    },
    decoderConfig,
  };
}

async function readAudioSignature(
  track: InputAudioTrack,
  supportedCodecs: AudioCodec[],
): Promise<
  | {
      supported: true;
      signature: AudioStreamSignature;
      decoderConfig: AudioDecoderConfig;
    }
  | { supported: false; reason: string }
> {
  if (!track.codec || !supportedCodecs.includes(track.codec)) {
    return {
      supported: false,
      reason:
        "Recording uses an audio format not supported for MP4 export; exported video only.",
    };
  }
  const decoderConfig = await track.getDecoderConfig();
  if (!decoderConfig) {
    return {
      supported: false,
      reason:
        "Audio track configuration could not be read; exported video only.",
    };
  }
  return {
    supported: true,
    signature: {
      codec: track.codec,
      sampleRate: track.sampleRate,
      numberOfChannels: track.numberOfChannels,
      decoderCodec: decoderConfig.codec,
      description: toByteArray(decoderConfig.description),
    },
    decoderConfig,
  };
}

async function downloadFragment(
  descriptor: RecordingDescriptor,
  recordings: RecordingService,
  scope: RecordingResourceScope,
  store?: RecordingStore,
): Promise<Blob> {
  if (store) {
    return scope.fetch(() => from(store.get(descriptor)));
  }
  return scope.fetch(() => recordings.download(descriptor));
}

export async function exportClip(
  options: ClipExportOptions,
): Promise<ClipExportResult> {
  const {
    cameraName,
    startTimeMs,
    endTimeMs,
    recordings,
    store,
    signal,
    onProgress,
  } = options;

  assertNotAborted(signal);

  if (!cameraName) {
    throw new ClipExportError("No camera selected.");
  }
  if (!Number.isFinite(startTimeMs) || !Number.isFinite(endTimeMs)) {
    throw new ClipExportError("Invalid clip start or end time.");
  }
  const startMs = Math.trunc(startTimeMs);
  const endMs = Math.trunc(endTimeMs);
  if (endMs <= startMs) {
    throw new ClipExportError("End time must be after start time.");
  }
  if (endMs - startMs > MAX_CLIP_DURATION_MS) {
    throw new ClipExportError("Clip duration cannot exceed 5 minutes.");
  }

  const startNs = BigInt(startMs) * NANOSECONDS_PER_MILLISECOND;
  const endNs = BigInt(endMs) * NANOSECONDS_PER_MILLISECOND;

  const scope = new RecordingResourceScope();
  const onAbort = () => scope.dispose();
  signal?.addEventListener("abort", onAbort, { once: true });

  const format = new Mp4OutputFormat({ fastStart: "in-memory" });
  const supportedVideoCodecs = format.getSupportedVideoCodecs();
  const supportedAudioCodecs = format.getSupportedAudioCodecs();
  const target = new BufferTarget();
  let output: Output<Mp4OutputFormat, BufferTarget> | undefined;
  let outputStarted = false;

  try {
    onProgress?.({
      stage: "resolving",
      completedFragments: 0,
      totalFragments: 0,
    });
    const fragments = await resolveContinuousFragments(
      cameraName,
      startMs,
      startNs,
      endNs,
      recordings,
      scope,
      signal,
    );

    let videoSignature: VideoStreamSignature | undefined;
    let videoDecoderConfig: VideoDecoderConfig | undefined;
    let videoSource: EncodedVideoPacketSource | undefined;
    let audioMode: AudioMode | undefined;

    let anchorUtcNs: bigint | undefined;
    let firstVideoWritten = false;
    let firstAudioWritten = false;
    let videoPacketCount = 0;
    let audioPacketCount = 0;
    let maxVideoRebasedSec: number | undefined;
    let minAllowedVideoRebasedSec = 0;
    let lastAudioRebasedSec: number | undefined;
    let previousFragmentMaxVideoUtcNs: bigint | undefined;
    let previousFragmentMaxAudioUtcNs: bigint | undefined;
    let maxEmittedVideoUtcNs: bigint | undefined;
    let reachedVideoEnd = false;

    for (let index = 0; index < fragments.length; index++) {
      assertNotAborted(signal);
      const descriptor = fragments[index];

      onProgress?.({
        stage: "downloading",
        completedFragments: index,
        totalFragments: fragments.length,
      });
      const blob = await downloadFragment(descriptor, recordings, scope, store);
      assertNotAborted(signal);

      onProgress?.({
        stage: "muxing",
        completedFragments: index,
        totalFragments: fragments.length,
      });

      const input = new Input({
        formats: [MATROSKA],
        source: new BlobSource(blob),
      });
      try {
        if (!(await input.canRead())) {
          throw new ClipExportError("Recording fragment is not readable.");
        }
        const metadata: CaptureTimelineMetadata = parseCaptureMetadata(
          (await input.getMetadataTags()).raw?.["COMMENTS"],
        );
        const videoTrack = await input.getPrimaryVideoTrack();
        if (!videoTrack) {
          throw new ClipExportError("Recording fragment has no video track.");
        }
        const videoInfo = await readVideoSignature(
          videoTrack,
          supportedVideoCodecs,
        );
        const audioTrack = await input.getPrimaryAudioTrack();
        const audioInfo = audioTrack
          ? await readAudioSignature(audioTrack, supportedAudioCodecs)
          : null;

        if (index === 0) {
          videoSignature = videoInfo.signature;
          videoDecoderConfig = videoInfo.decoderConfig;
          videoSource = new EncodedVideoPacketSource(videoSignature.codec);

          output = new Output({ format, target });
          output.addVideoTrack(videoSource);

          if (!audioInfo) {
            audioMode = { kind: "none" };
          } else if (!audioInfo.supported) {
            audioMode = { kind: "omitted", reason: audioInfo.reason };
          } else {
            const audioSource = new EncodedAudioPacketSource(
              audioInfo.signature.codec,
            );
            output.addAudioTrack(audioSource);
            audioMode = {
              kind: "included",
              signature: audioInfo.signature,
              decoderConfig: audioInfo.decoderConfig,
              source: audioSource,
            };
          }

          await output.start();
          outputStarted = true;
        } else {
          if (!videoSignaturesMatch(videoSignature!, videoInfo.signature)) {
            throw new ClipExportError(
              "Video stream parameters changed within the selected range. Choose a shorter range without camera restarts.",
            );
          }
          if (audioMode!.kind === "included") {
            if (
              !audioInfo ||
              !audioInfo.supported ||
              !audioSignaturesMatch(audioMode!.signature, audioInfo.signature)
            ) {
              throw new ClipExportError(
                "Audio stream parameters changed within the selected range. Choose a shorter range without camera restarts.",
              );
            }
          } else if (audioInfo && audioInfo.supported) {
            throw new ClipExportError(
              "Audio stream parameters changed within the selected range. Choose a shorter range without camera restarts.",
            );
          }
        }

        const videoSink = new EncodedPacketSink(videoTrack);
        let startVideoPacket: EncodedPacket | null = null;
        if (index === 0) {
          const startSessionSec = utcNanosecondsToSessionSeconds(
            metadata,
            startNs,
          );
          startVideoPacket =
            (await videoSink.getKeyPacket(startSessionSec, {
              verifyKeyPackets: true,
            })) ??
            (await videoSink.getFirstKeyPacket({ verifyKeyPackets: true }));
          if (!startVideoPacket) {
            throw new ClipExportError(
              "Could not find a video keyframe to start the clip.",
            );
          }
          anchorUtcNs = sessionSecondsToUtcNanoseconds(
            metadata,
            startVideoPacket.timestamp,
          );
        } else {
          startVideoPacket = await videoSink.getFirstPacket({
            verifyKeyPackets: true,
          });
          if (!startVideoPacket) {
            throw new ClipExportError(
              "Recording fragment has no video packets.",
            );
          }
          if (startVideoPacket.type !== "key") {
            throw new ClipExportError(
              "Recording fragment does not begin on a video keyframe.",
            );
          }
          const firstUtcNs = sessionSecondsToUtcNanoseconds(
            metadata,
            startVideoPacket.timestamp,
          );
          if (
            previousFragmentMaxVideoUtcNs !== undefined &&
            firstUtcNs < previousFragmentMaxVideoUtcNs
          ) {
            throw new ClipExportError(
              "Selected range contains overlapping recordings. Choose a range without overlapping capture sessions.",
            );
          }
        }

        const videoIter = videoSink.packets(startVideoPacket, undefined, {
          verifyKeyPackets: true,
        });
        let fragmentMaxVideoUtcNs = previousFragmentMaxVideoUtcNs;
        const pendingBeyondEnd: Array<{
          packet: EncodedPacket;
          utcNs: bigint;
        }> = [];

        const writeVideoPacket = async (
          packet: EncodedPacket,
          utcNs: bigint,
        ) => {
          if (
            firstVideoWritten &&
            packet.type === "key" &&
            maxVideoRebasedSec !== undefined
          ) {
            minAllowedVideoRebasedSec = maxVideoRebasedSec;
          }
          const rawRebasedSec = Math.max(0, Number(utcNs - anchorUtcNs!) / 1e9);
          const rebasedSec = Math.max(rawRebasedSec, minAllowedVideoRebasedSec);
          const cloned = packet.clone({ timestamp: rebasedSec });
          await videoSource!.add(
            cloned,
            firstVideoWritten
              ? undefined
              : { decoderConfig: videoDecoderConfig },
          );
          firstVideoWritten = true;
          videoPacketCount++;
          maxVideoRebasedSec =
            maxVideoRebasedSec === undefined
              ? rebasedSec
              : Math.max(maxVideoRebasedSec, rebasedSec);
          if (
            fragmentMaxVideoUtcNs === undefined ||
            utcNs > fragmentMaxVideoUtcNs
          ) {
            fragmentMaxVideoUtcNs = utcNs;
          }
          if (
            maxEmittedVideoUtcNs === undefined ||
            utcNs > maxEmittedVideoUtcNs
          ) {
            maxEmittedVideoUtcNs = utcNs;
          }
        };

        try {
          for await (const packet of videoIter) {
            assertNotAborted(signal);
            const utcNs = sessionSecondsToUtcNanoseconds(
              metadata,
              packet.timestamp,
            );
            if (
              previousFragmentMaxVideoUtcNs !== undefined &&
              utcNs < previousFragmentMaxVideoUtcNs
            ) {
              throw new ClipExportError(
                "Selected range contains overlapping recordings. Choose a range without overlapping capture sessions.",
              );
            }
            const effectiveVideoEndNs =
              maxEmittedVideoUtcNs !== undefined && maxEmittedVideoUtcNs > endNs
                ? maxEmittedVideoUtcNs
                : endNs;
            if (utcNs <= effectiveVideoEndNs) {
              for (const pending of pendingBeyondEnd) {
                await writeVideoPacket(pending.packet, pending.utcNs);
              }
              pendingBeyondEnd.length = 0;
              await writeVideoPacket(packet, utcNs);
            } else {
              if (packet.type === "key") {
                reachedVideoEnd = true;
                break;
              }
              pendingBeyondEnd.push({ packet, utcNs });
              if (pendingBeyondEnd.length > MAX_VIDEO_REORDER_BUFFER) {
                reachedVideoEnd = true;
                break;
              }
            }
          }
        } finally {
          await videoIter.return().catch(() => {});
        }
        previousFragmentMaxVideoUtcNs = fragmentMaxVideoUtcNs;

        if (audioMode!.kind === "included" && audioTrack) {
          const audioSink = new EncodedPacketSink(audioTrack);
          let startAudioPacket: EncodedPacket | null;
          if (index === 0) {
            const anchorSessionSec = utcNanosecondsToSessionSeconds(
              metadata,
              anchorUtcNs!,
            );
            startAudioPacket =
              (await audioSink.getPacket(anchorSessionSec)) ??
              (await audioSink.getFirstPacket());
          } else {
            startAudioPacket = await audioSink.getFirstPacket();
          }

          if (startAudioPacket) {
            const audioEndNs =
              maxEmittedVideoUtcNs !== undefined && maxEmittedVideoUtcNs > endNs
                ? maxEmittedVideoUtcNs
                : endNs;
            const audioIter = audioSink.packets(startAudioPacket);
            let fragmentMaxAudioUtcNs = previousFragmentMaxAudioUtcNs;
            try {
              for await (const packet of audioIter) {
                assertNotAborted(signal);
                const utcNs = sessionSecondsToUtcNanoseconds(
                  metadata,
                  packet.timestamp,
                );
                if (utcNs < anchorUtcNs!) {
                  continue;
                }
                if (
                  previousFragmentMaxAudioUtcNs !== undefined &&
                  utcNs <= previousFragmentMaxAudioUtcNs
                ) {
                  continue;
                }
                if (utcNs > audioEndNs) {
                  break;
                }
                const rawRebasedSec = Number(utcNs - anchorUtcNs!) / 1e9;
                const rebasedSec =
                  lastAudioRebasedSec !== undefined &&
                  rawRebasedSec < lastAudioRebasedSec
                    ? lastAudioRebasedSec
                    : rawRebasedSec;
                const cloned = packet.clone({ timestamp: rebasedSec });
                await audioMode!.source.add(
                  cloned,
                  firstAudioWritten
                    ? undefined
                    : { decoderConfig: audioMode!.decoderConfig },
                );
                firstAudioWritten = true;
                audioPacketCount++;
                lastAudioRebasedSec = rebasedSec;
                if (
                  fragmentMaxAudioUtcNs === undefined ||
                  utcNs > fragmentMaxAudioUtcNs
                ) {
                  fragmentMaxAudioUtcNs = utcNs;
                }
              }
            } finally {
              await audioIter.return().catch(() => {});
            }
            previousFragmentMaxAudioUtcNs = fragmentMaxAudioUtcNs;
          }
        }
      } finally {
        input.dispose();
      }

      if (reachedVideoEnd) {
        break;
      }
    }

    if (
      !output ||
      !videoPacketCount ||
      anchorUtcNs === undefined ||
      maxEmittedVideoUtcNs === undefined
    ) {
      throw new ClipExportError(
        "No video frames were found in the selected range.",
      );
    }

    assertNotAborted(signal);
    onProgress?.({
      stage: "finalizing",
      completedFragments: fragments.length,
      totalFragments: fragments.length,
    });
    await output.finalize();
    outputStarted = false;

    if (!target.buffer) {
      throw new ClipExportError("Failed to generate MP4 output.");
    }

    const actualStartMs = Number(anchorUtcNs / NANOSECONDS_PER_MILLISECOND);
    const actualEndMs = Number(
      maxEmittedVideoUtcNs / NANOSECONDS_PER_MILLISECOND,
    );
    return {
      blob: new Blob([target.buffer], { type: "video/mp4" }),
      filename: buildClipFilename(cameraName, startMs, endMs),
      actualStartMs,
      actualEndMs,
      videoPacketCount,
      audioPacketCount,
      audioOmittedReason:
        audioMode?.kind === "omitted" ? audioMode.reason : undefined,
    };
  } catch (error) {
    if (output && outputStarted) {
      await output.cancel().catch(() => {});
    }
    if (signal?.aborted || error instanceof ObsoleteFrameOperation) {
      throw new ClipExportCancelledError();
    }
    throw error;
  } finally {
    signal?.removeEventListener("abort", onAbort);
    scope.dispose();
  }
}
