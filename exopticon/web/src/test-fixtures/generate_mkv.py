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


def main():
    output = pathlib.Path(__file__).parent
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


if __name__ == "__main__":
    sys.exit(main())
