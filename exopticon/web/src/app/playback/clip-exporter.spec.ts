import { HttpErrorResponse } from "@angular/common/http";
import {
  BlobSource,
  EncodedPacketSink,
  Input,
  MP4,
  VideoSampleSink,
} from "mediabunny";
import { of, Subject, throwError } from "rxjs";
import { RecordingDescriptor, RecordingService } from "../recording.service";
import {
  buildClipFilename,
  ClipExportCancelledError,
  ClipExportError,
  ClipExportProgress,
  exportClip,
  MAX_CLIP_DURATION_MS,
} from "./clip-exporter";
import { RecordingStore } from "./recording-store";

async function loadFixtureBlob(name: string): Promise<Blob> {
  const response = await fetch(`/base/test-fixtures/${name}.mkv`);
  if (!response.ok) {
    throw new Error(`Missing MKV fixture: ${name}`);
  }
  return response.blob();
}

function makeDescriptor(
  fileId: number,
  beginMs: number,
  endMs: number,
  byteLength: number,
): RecordingDescriptor {
  return {
    fileId,
    beginTime: new Date(beginMs).toISOString(),
    endTime: new Date(endMs).toISOString(),
    byteLength,
    contentUrl: `/fixture/${fileId}/content`,
    nextUrl: `/fixture/${fileId}/next`,
  };
}

describe("exportClip", () => {
  const baseMs = Date.parse("2026-09-23T10:00:00.123Z");

  function createRecordingsSpy(): jasmine.SpyObj<RecordingService> {
    return jasmine.createSpyObj<RecordingService>("RecordingService", [
      "getRanges",
      "resolveAt",
      "getNext",
      "download",
    ]);
  }

  it("rejects invalid camera names, reversed bounds, and clips longer than 5 minutes", async () => {
    const recordings = createRecordingsSpy();

    await expectAsync(
      exportClip({
        cameraName: "",
        startTimeMs: baseMs,
        endTimeMs: baseMs + 1000,
        recordings,
      }),
    ).toBeRejectedWithError(ClipExportError);

    await expectAsync(
      exportClip({
        cameraName: "front",
        startTimeMs: baseMs + 1000,
        endTimeMs: baseMs + 1000,
        recordings,
      }),
    ).toBeRejectedWithError(ClipExportError);

    await expectAsync(
      exportClip({
        cameraName: "front",
        startTimeMs: baseMs,
        endTimeMs: baseMs + MAX_CLIP_DURATION_MS + 1,
        recordings,
      }),
    ).toBeRejectedWithError(ClipExportError, /5 minutes/);

    expect(recordings.resolveAt).not.toHaveBeenCalled();
  });

  it("rejects unavailable start times, recording gaps, and ranges past the latest finalized fragment", async () => {
    const recordings = createRecordingsSpy();
    recordings.resolveAt.and.returnValue(
      throwError(
        new HttpErrorResponse({
          status: 404,
          error: { error: "unavailable_time" },
        }),
      ),
    );
    await expectAsync(
      exportClip({
        cameraName: "front",
        startTimeMs: baseMs,
        endTimeMs: baseMs + 1000,
        recordings,
      }),
    ).toBeRejectedWithError(ClipExportError, /No finalized recording/);

    const first = makeDescriptor(1, baseMs, baseMs + 1800, 100);
    const afterGap = makeDescriptor(2, baseMs + 2500, baseMs + 4300, 100);
    recordings.resolveAt.and.returnValue(of(first));
    recordings.getNext.and.returnValue(of({ recording: afterGap }));
    await expectAsync(
      exportClip({
        cameraName: "front",
        startTimeMs: baseMs + 100,
        endTimeMs: baseMs + 3000,
        recordings,
      }),
    ).toBeRejectedWithError(ClipExportError, /recording gap/);

    recordings.getNext.and.returnValue(of({ recording: null }));
    await expectAsync(
      exportClip({
        cameraName: "front",
        startTimeMs: baseMs + 100,
        endTimeMs: baseMs + 3000,
        recordings,
      }),
    ).toBeRejectedWithError(ClipExportError, /extends past/);
  });

  it("exports a keyframe-expanded video-only MP4 with B-frames that decodes cleanly", async () => {
    const blob = await loadFixtureBlob("capture-session-a");
    const recordings = createRecordingsSpy();
    const descriptor = makeDescriptor(1, baseMs, baseMs + 1800, blob.size);
    recordings.resolveAt.and.returnValue(of(descriptor));
    recordings.download.and.returnValue(of(blob));
    recordings.getNext.and.returnValue(of({ recording: null }));

    const startTimeMs = baseMs + 350;
    const endTimeMs = baseMs + 1350;
    const result = await exportClip({
      cameraName: "front",
      startTimeMs,
      endTimeMs,
      recordings,
    });

    expect(result.blob.type).toBe("video/mp4");
    expect(result.filename).toBe(
      buildClipFilename("front", startTimeMs, endTimeMs),
    );
    expect(result.audioPacketCount).toBe(0);
    expect(result.audioOmittedReason).toBeUndefined();
    // Keyframes in capture-session-a are at +0ms and +1000ms, so +350ms expands back to +0ms.
    expect(result.actualStartMs).toBe(baseMs);
    expect(result.actualEndMs).toBeGreaterThanOrEqual(baseMs + 1300);

    const outputInput = new Input({
      formats: [MP4],
      source: new BlobSource(result.blob),
    });
    try {
      expect(await outputInput.canRead()).toBe(true);
      const videoTrack = await outputInput.getPrimaryVideoTrack();
      const audioTrack = await outputInput.getPrimaryAudioTrack();
      expect(videoTrack).not.toBeNull();
      expect(audioTrack).toBeNull();
      expect(videoTrack!.codec).toBe("avc");
      expect(videoTrack!.codedWidth).toBe(160);
      expect(videoTrack!.codedHeight).toBe(120);

      const sink = new VideoSampleSink(videoTrack!);
      const timestamps: number[] = [];
      for await (const sample of sink.samples()) {
        timestamps.push(Math.round(sample.timestamp * 1000));
        sample.close();
      }
      expect(timestamps.length).toBeGreaterThanOrEqual(14);
      expect(timestamps[0]).toBe(0);
      // Every frame through the final emitted reference frame must be contiguous
      // (100 ms intervals at 10 fps) with no dropped mid-mini-GOP B-frames.
      for (let i = 1; i < timestamps.length; i++) {
        expect(timestamps[i] - timestamps[i - 1]).toBe(100);
      }
    } finally {
      outputInput.dispose();
    }
  });

  it("keeps remaining B-frames in the final mini-GOP when cutting mid-reorder window", async () => {
    const blob = await loadFixtureBlob("capture-session-a");
    const recordings = createRecordingsSpy();
    const descriptor = makeDescriptor(1, baseMs, baseMs + 1800, blob.size);
    recordings.resolveAt.and.returnValue(of(descriptor));
    recordings.download.and.returnValue(of(blob));
    recordings.getNext.and.returnValue(of({ recording: null }));

    // Test every 100 ms cut point across the B-frame mini-GOPs in capture-session-a (bframes=2, 10 fps).
    for (const offsetMs of [150, 250, 450, 550, 1150, 1250]) {
      const result = await exportClip({
        cameraName: "front",
        startTimeMs: baseMs,
        endTimeMs: baseMs + offsetMs,
        recordings,
      });
      const outputInput = new Input({
        formats: [MP4],
        source: new BlobSource(result.blob),
      });
      try {
        const videoTrack = await outputInput.getPrimaryVideoTrack();
        const sink = new VideoSampleSink(videoTrack!);
        const timestamps: number[] = [];
        for await (const sample of sink.samples()) {
          timestamps.push(Math.round(sample.timestamp * 1000));
          sample.close();
        }
        expect(timestamps[0]).toBe(0);
        expect(timestamps[timestamps.length - 1]).toBeGreaterThanOrEqual(
          offsetMs - 50,
        );
        for (let i = 1; i < timestamps.length; i++) {
          expect(timestamps[i] - timestamps[i - 1])
            .withContext(`offsetMs=${offsetMs}, index=${i}`)
            .toBe(100);
        }
      } finally {
        outputInput.dispose();
      }
    }
  });

  it("exports a multi-fragment H.264 + Opus MP4, reports progress, and reuses RecordingStore", async () => {
    const [part1Blob, part2Blob] = await Promise.all([
      loadFixtureBlob("capture-av-part1"),
      loadFixtureBlob("capture-av-part2"),
    ]);
    const part1 = makeDescriptor(10, baseMs, baseMs + 1800, part1Blob.size);
    const part2 = makeDescriptor(
      11,
      baseMs + 1800,
      baseMs + 3600,
      part2Blob.size,
    );

    const recordings = createRecordingsSpy();
    recordings.resolveAt.and.returnValue(of(part1));
    recordings.getNext.and.callFake((desc) =>
      of({ recording: desc.fileId === 10 ? part2 : null }),
    );
    recordings.download.and.callFake((desc) =>
      of(desc.fileId === 10 ? part1Blob : part2Blob),
    );

    const store = new RecordingStore(recordings);
    const progress: ClipExportProgress[] = [];
    try {
      // Request starts at +1250ms (expands to +1000ms keyframe in part1) and ends at +2550ms in part2.
      const result = await exportClip({
        cameraName: "front",
        startTimeMs: baseMs + 1250,
        endTimeMs: baseMs + 2550,
        recordings,
        store,
        onProgress: (update) => progress.push(update),
      });

      expect(result.videoPacketCount).toBeGreaterThanOrEqual(15);
      expect(result.audioPacketCount).toBeGreaterThan(20);
      expect(result.audioOmittedReason).toBeUndefined();
      expect(result.actualStartMs).toBe(baseMs + 1000);
      expect(result.actualEndMs).toBeGreaterThanOrEqual(baseMs + 2500);
      expect(progress.map((p) => p.stage)).toContain("resolving");
      expect(progress.map((p) => p.stage)).toContain("downloading");
      expect(progress.map((p) => p.stage)).toContain("muxing");
      expect(progress[progress.length - 1]).toEqual({
        stage: "finalizing",
        completedFragments: 2,
        totalFragments: 2,
      });

      const outputInput = new Input({
        formats: [MP4],
        source: new BlobSource(result.blob),
      });
      try {
        expect(await outputInput.canRead()).toBe(true);
        const videoTrack = await outputInput.getPrimaryVideoTrack();
        const audioTrack = await outputInput.getPrimaryAudioTrack();
        expect(videoTrack?.codec).toBe("avc");
        expect(audioTrack?.codec).toBe("opus");
        expect(audioTrack?.numberOfChannels).toBe(1);
        const duration = await outputInput.computeDuration();
        expect(duration).toBeGreaterThan(1.4);
        expect(duration).toBeLessThan(2.2);

        // Verify multi-fragment MP4 decodes cleanly across the fragment join
        // and that Opus audio starts aligned with video and spans the clip.
        const videoSink = new VideoSampleSink(videoTrack!);
        const decodedVideoMs: number[] = [];
        for await (const sample of videoSink.samples()) {
          decodedVideoMs.push(Math.round(sample.timestamp * 1000));
          sample.close();
        }
        expect(decodedVideoMs.length).toBeGreaterThanOrEqual(15);
        expect(decodedVideoMs[0]).toBe(0);
        for (let i = 1; i < decodedVideoMs.length; i++) {
          expect(decodedVideoMs[i]).toBeGreaterThanOrEqual(
            decodedVideoMs[i - 1],
          );
        }

        const audioSink = new EncodedPacketSink(audioTrack!);
        const firstAudio = await audioSink.getFirstPacket();
        const lastAudio = await audioSink.getPacket(Infinity);
        expect(firstAudio).not.toBeNull();
        expect(lastAudio).not.toBeNull();
        expect(firstAudio!.timestamp).toBeGreaterThanOrEqual(0);
        expect(firstAudio!.timestamp).toBeLessThan(0.05);
        expect(lastAudio!.timestamp).toBeGreaterThan(1.4);
      } finally {
        outputInput.dispose();
      }

      // Second export over the same two fragments hits the shared RecordingStore cache.
      expect(recordings.download).toHaveBeenCalledTimes(2);
      await exportClip({
        cameraName: "front",
        startTimeMs: baseMs + 1300,
        endTimeMs: baseMs + 2400,
        recordings,
        store,
      });
      expect(recordings.download).toHaveBeenCalledTimes(2);
    } finally {
      store.dispose();
    }
  });

  it("rejects incompatible video or audio stream parameter changes across fragments", async () => {
    const [sessionA, sessionB, avPart2] = await Promise.all([
      loadFixtureBlob("capture-session-a"),
      loadFixtureBlob("capture-session-b"),
      loadFixtureBlob("capture-av-part2"),
    ]);

    const first = makeDescriptor(1, baseMs, baseMs + 1800, sessionA.size);
    const changedVideo = makeDescriptor(
      2,
      baseMs + 1800,
      baseMs + 3600,
      sessionB.size,
    );
    const recordings = createRecordingsSpy();
    recordings.resolveAt.and.returnValue(of(first));
    recordings.getNext.and.returnValue(of({ recording: changedVideo }));
    recordings.download.and.callFake((desc) =>
      of(desc.fileId === 1 ? sessionA : sessionB),
    );

    await expectAsync(
      exportClip({
        cameraName: "front",
        startTimeMs: baseMs + 100,
        endTimeMs: baseMs + 2500,
        recordings,
      }),
    ).toBeRejectedWithError(ClipExportError, /Video stream parameters changed/);

    // Video-only fragment followed by a fragment with an Opus audio track.
    const changedAudio = makeDescriptor(
      3,
      baseMs + 1800,
      baseMs + 3600,
      avPart2.size,
    );
    recordings.getNext.and.returnValue(of({ recording: changedAudio }));
    recordings.download.and.callFake((desc) =>
      of(desc.fileId === 1 ? sessionA : avPart2),
    );

    await expectAsync(
      exportClip({
        cameraName: "front",
        startTimeMs: baseMs + 100,
        endTimeMs: baseMs + 2500,
        recordings,
      }),
    ).toBeRejectedWithError(ClipExportError, /Audio stream parameters changed/);
  });

  it("exports video-only MP4 with a warning for legacy PCMU MKV recordings", async () => {
    const legacyBlob = await loadFixtureBlob("capture-legacy-pcmu");
    const descriptor = makeDescriptor(
      20,
      baseMs,
      baseMs + 1800,
      legacyBlob.size,
    );
    const recordings = createRecordingsSpy();
    recordings.resolveAt.and.returnValue(of(descriptor));
    recordings.download.and.returnValue(of(legacyBlob));
    recordings.getNext.and.returnValue(of({ recording: null }));

    const result = await exportClip({
      cameraName: "front",
      startTimeMs: baseMs + 100,
      endTimeMs: baseMs + 1500,
      recordings,
    });

    expect(result.videoPacketCount).toBeGreaterThan(0);
    expect(result.audioPacketCount).toBe(0);
    expect(result.audioOmittedReason).toContain("exported video only");
  });

  it("cancels in-flight fragment downloads cleanly when aborted", async () => {
    const pending = new Subject<Blob>();
    const descriptor = makeDescriptor(30, baseMs, baseMs + 1800, 100);
    const recordings = createRecordingsSpy();
    recordings.resolveAt.and.returnValue(of(descriptor));
    recordings.download.and.returnValue(pending);

    const abortController = new AbortController();
    const promise = exportClip({
      cameraName: "front",
      startTimeMs: baseMs + 100,
      endTimeMs: baseMs + 1000,
      recordings,
      signal: abortController.signal,
    });

    for (let i = 0; i < 20 && pending.observers.length === 0; i++) {
      await new Promise((resolve) => setTimeout(resolve, 5));
    }
    expect(pending.observers.length).toBe(1);

    abortController.abort();
    await expectAsync(promise).toBeRejectedWithError(ClipExportCancelledError);
    expect(pending.observers.length).toBe(0);
  });

  it("preserves equal-timestamp frames within a single irregular fragment", async () => {
    const irregularBlob = await loadFixtureBlob("capture-irregular");
    const irregularBaseMs = Date.parse("2026-09-23T10:00:04.123Z");
    const descriptor = makeDescriptor(
      40,
      irregularBaseMs,
      irregularBaseMs + 1000,
      irregularBlob.size,
    );
    const recordings = createRecordingsSpy();
    recordings.resolveAt.and.returnValue(of(descriptor));
    recordings.download.and.returnValue(of(irregularBlob));
    recordings.getNext.and.returnValue(of({ recording: null }));

    const result = await exportClip({
      cameraName: "front",
      startTimeMs: irregularBaseMs,
      endTimeMs: irregularBaseMs + 800,
      recordings,
    });

    expect(result.videoPacketCount).toBe(8);
    const outputInput = new Input({
      formats: [MP4],
      source: new BlobSource(result.blob),
    });
    try {
      const videoTrack = await outputInput.getPrimaryVideoTrack();
      const sink = new VideoSampleSink(videoTrack!);
      const timestamps: number[] = [];
      for await (const sample of sink.samples()) {
        timestamps.push(Math.round(sample.timestamp * 1000));
        sample.close();
      }
      expect(timestamps).toEqual([0, 100, 100, 250, 300, 500, 500, 700]);
    } finally {
      outputInput.dispose();
    }
  });

  it("rejects overlapping recordings across fragment boundaries", async () => {
    const part1Blob = await loadFixtureBlob("capture-av-part1");
    const first = makeDescriptor(50, baseMs, baseMs + 1800, part1Blob.size);
    // Descriptor extends past first.endTime, but its actual media starts at baseMs (< first's last frame).
    const overlapping = makeDescriptor(
      51,
      baseMs + 1500,
      baseMs + 3300,
      part1Blob.size,
    );
    const recordings = createRecordingsSpy();
    recordings.resolveAt.and.returnValue(of(first));
    recordings.getNext.and.returnValue(of({ recording: overlapping }));
    recordings.download.and.returnValue(of(part1Blob));

    await expectAsync(
      exportClip({
        cameraName: "front",
        startTimeMs: baseMs + 100,
        endTimeMs: baseMs + 2500,
        recordings,
      }),
    ).toBeRejectedWithError(ClipExportError, /overlapping recordings/);

    // Descriptor completely contained inside the previous fragment is rejected before download.
    const contained = makeDescriptor(52, baseMs + 200, baseMs + 1500, 100);
    recordings.getNext.and.returnValue(of({ recording: contained }));
    recordings.download.calls.reset();
    await expectAsync(
      exportClip({
        cameraName: "front",
        startTimeMs: baseMs + 100,
        endTimeMs: baseMs + 2500,
        recordings,
      }),
    ).toBeRejectedWithError(ClipExportError, /overlapping recordings/);
    expect(recordings.download).not.toHaveBeenCalled();
  });
});
