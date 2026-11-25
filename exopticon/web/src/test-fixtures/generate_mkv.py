#!/usr/bin/env python3
"""Generate small H.264 Matroska browser fixtures with capture-worker tags.

Requires GStreamer Python bindings and the x264, h264parse, and matroskamux
plugins. The files use the capture worker's muxer settings and metadata schema.
"""

import json
import pathlib
import sys

import gi

gi.require_version("Gst", "1.0")
from gi.repository import Gst

Gst.init(None)


def generate(path, session_id, anchor_utc, offset_seconds, width, bframes):
    timeline = {
        "captureSessionId": session_id,
        "clockAnchor": {
            "pipelineRunningTimeNs": offset_seconds * 1_000_000_000,
            "utcTime": anchor_utc,
        },
    }
    pipeline = Gst.parse_launch(
        "videotestsrc num-buffers=18 pattern=ball ! "
        f"video/x-raw,width={width},height=120,framerate=10/1 ! "
        f"x264enc bframes={bframes} key-int-max=10 bitrate=150 speed-preset=ultrafast ! "
        "h264parse config-interval=-1 ! "
        "matroskamux name=mux offset-to-zero=false timecodescale=1000000 "
        f"cluster-timestamp-offset={offset_seconds * 1_000_000_000} ! "
        f"filesink location={path}"
    )
    mux = pipeline.get_by_name("mux")
    tags = Gst.TagList.new_empty()
    tags.add_value(Gst.TagMergeMode.REPLACE, Gst.TAG_COMMENT, json.dumps(timeline))
    mux.merge_tags(tags, Gst.TagMergeMode.REPLACE)
    pipeline.set_state(Gst.State.PLAYING)
    message = pipeline.get_bus().timed_pop_filtered(
        Gst.CLOCK_TIME_NONE, Gst.MessageType.ERROR | Gst.MessageType.EOS
    )
    pipeline.set_state(Gst.State.NULL)
    if message.type == Gst.MessageType.ERROR:
        error, detail = message.parse_error()
        raise RuntimeError(f"{error}: {detail}")


def generate_irregular(path):
    """Exercise equal presentation times and variable intervals after encoding."""
    timeline = {
        "captureSessionId": "33333333-3333-4333-8333-333333333333",
        "clockAnchor": {
            "pipelineRunningTimeNs": 5_000_000_000,
            "utcTime": "2026-09-23T10:00:04.123456789Z",
        },
    }
    pipeline = Gst.parse_launch(
        'appsrc name=src format=time is-live=false block=true '
        'caps="video/x-raw,format=I420,width=64,height=48,framerate=10/1" ! '
        'x264enc bframes=0 key-int-max=10 speed-preset=ultrafast bitrate=150 ! '
        'h264parse config-interval=-1 ! identity name=warp ! '
        'matroskamux name=mux offset-to-zero=false timecodescale=1000000 '
        'cluster-timestamp-offset=5000000000 ! '
        f'filesink location={path}'
    )
    mux = pipeline.get_by_name("mux")
    tags = Gst.TagList.new_empty()
    tags.add_value(Gst.TagMergeMode.REPLACE, Gst.TAG_COMMENT, json.dumps(timeline))
    mux.merge_tags(tags, Gst.TagMergeMode.REPLACE)

    # The encoder drops raw buffers with duplicate PTS. Duplicate two encoded
    # buffers instead so Matroska contains distinct frames at equal timestamps.
    count = 0
    previous_pts = None

    def rewrite_pts(_pad, info):
        nonlocal count, previous_pts
        buffer = info.get_buffer()
        if count in (2, 6):
            buffer.pts = previous_pts
        previous_pts = buffer.pts
        count += 1
        return Gst.PadProbeReturn.OK

    pipeline.get_by_name("warp").get_static_pad("src").add_probe(
        Gst.PadProbeType.BUFFER, rewrite_pts
    )
    source = pipeline.get_by_name("src")
    pipeline.set_state(Gst.State.PLAYING)
    for index, milliseconds in enumerate((0, 100, 200, 250, 300, 500, 600, 700)):
        pixels = bytes([index * 20 + 16]) * (64 * 48) + bytes([128]) * (64 * 48 // 2)
        buffer = Gst.Buffer.new_allocate(None, len(pixels), None)
        buffer.fill(0, pixels)
        buffer.pts = milliseconds * 1_000_000
        buffer.duration = 100_000_000
        if source.emit("push-buffer", buffer) != Gst.FlowReturn.OK:
            raise RuntimeError("Could not push irregular video frame")
    source.emit("end-of-stream")
    message = pipeline.get_bus().timed_pop_filtered(
        Gst.CLOCK_TIME_NONE, Gst.MessageType.ERROR | Gst.MessageType.EOS
    )
    pipeline.set_state(Gst.State.NULL)
    if message.type == Gst.MessageType.ERROR:
        error, detail = message.parse_error()
        raise RuntimeError(f"{error}: {detail}")


def generate_av_fragment(
    source_mkv, path, session_id, anchor_utc, anchor_ns, offset_ns, audio_pipeline
):
    """Mux H.264 video from an existing fixture with synthesized audio into MKV."""
    timeline = {
        "captureSessionId": session_id,
        "clockAnchor": {
            "pipelineRunningTimeNs": anchor_ns,
            "utcTime": anchor_utc,
        },
    }
    pipeline = Gst.parse_launch(
        f'filesrc location="{source_mkv}" ! matroskademux ! h264parse config-interval=-1 ! '
        "identity name=vshift ! video/x-h264,stream-format=avc,alignment=au ! mux.video_0 "
        f"{audio_pipeline} ! identity name=ashift ! mux.audio_0 "
        "matroskamux name=mux offset-to-zero=false timecodescale=1000000 ! "
        f'filesink location="{path}"'
    )
    # Both demuxed video (segment.start=5s) and audiotestsrc start at running time 0;
    # shift both pads to offset_ns.
    pipeline.get_by_name("vshift").get_static_pad("src").set_offset(offset_ns)
    pipeline.get_by_name("ashift").get_static_pad("src").set_offset(offset_ns)
    mux = pipeline.get_by_name("mux")
    tags = Gst.TagList.new_empty()
    tags.add_value(Gst.TagMergeMode.REPLACE, Gst.TAG_COMMENT, json.dumps(timeline))
    mux.merge_tags(tags, Gst.TagMergeMode.REPLACE)
    pipeline.set_state(Gst.State.PLAYING)
    message = pipeline.get_bus().timed_pop_filtered(
        Gst.CLOCK_TIME_NONE, Gst.MessageType.ERROR | Gst.MessageType.EOS
    )
    pipeline.set_state(Gst.State.NULL)
    if message.type == Gst.MessageType.ERROR:
        error, detail = message.parse_error()
        raise RuntimeError(f"{error}: {detail}")


def main():
    output = pathlib.Path(__file__).parent
    if "--av-only" not in sys.argv:
        generate(
            output / "capture-session-a.mkv",
            "11111111-1111-4111-8111-111111111111",
            "2026-09-23T10:00:00.123456789Z",
            5,
            160,
            2,
        )
        generate(
            output / "capture-session-b.mkv",
            "22222222-2222-4222-8222-222222222222",
            "2026-09-23T10:00:02.123456789Z",
            1,
            128,
            0,
        )
        generate_irregular(output / "capture-irregular.mkv")

    opus_audio = (
        "audiotestsrc num-buffers=90 samplesperbuffer=160 ! "
        "audio/x-raw,format=S16LE,rate=8000,channels=1 ! "
        "mulawenc ! mulawdec ! audioconvert ! audioresample ! opusenc"
    )
    generate_av_fragment(
        output / "capture-session-a.mkv",
        output / "capture-av-part1.mkv",
        "44444444-4444-4444-8444-444444444444",
        "2026-09-23T10:00:00.123456789Z",
        5_000_000_000,
        5_000_000_000,
        opus_audio,
    )
    generate_av_fragment(
        output / "capture-session-a.mkv",
        output / "capture-av-part2.mkv",
        "44444444-4444-4444-8444-444444444444",
        "2026-09-23T10:00:00.123456789Z",
        5_000_000_000,
        6_800_000_000,
        opus_audio,
    )
    pcmu_audio = (
        "audiotestsrc num-buffers=90 samplesperbuffer=160 ! "
        "audio/x-raw,format=S16LE,rate=8000,channels=1 ! mulawenc"
    )
    generate_av_fragment(
        output / "capture-session-a.mkv",
        output / "capture-legacy-pcmu.mkv",
        "55555555-5555-4555-8555-555555555555",
        "2026-09-23T10:00:00.123456789Z",
        5_000_000_000,
        5_000_000_000,
        pcmu_audio,
    )


if __name__ == "__main__":
    sys.exit(main())
