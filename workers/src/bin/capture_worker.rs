/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2025 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

//! Exopticon is a free video surveillance system

// to avoid the warning from diesel macros
#![allow(proc_macro_derive_resolution_fallback)]
#![deny(
    nonstandard_style,
    warnings,
    rust_2018_idioms,
    unused,
    future_incompatible,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::integer_division)]
#![allow(clippy::missing_inline_in_public_items)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::implicit_return)]
#![allow(clippy::print_stdout)]
#![allow(clippy::expect_used)]
#![allow(clippy::future_not_send)]
#![allow(clippy::too_many_lines)]

use std::{
    fs::create_dir_all,
    path::PathBuf,
    sync::{Arc, Mutex, Weak},
    time::{Duration, Instant},
};

use chrono::{SecondsFormat, Utc};
use exserial::{
    exlog::ExLog,
    models::{self, CaptureMessage},
};
use gstreamer::{
    self as gst, Bin, Element, Pad,
    glib::object::{Cast, ObjectExt},
    prelude::{ElementExt, ElementExtManual, GstBinExtManual, GstObjectExt, PadExt},
};
use gstreamer_app::{AppSink, AppSinkCallbacks};
use log::{debug, error, info};
use uuid::Uuid;

static LOGGER: ExLog = ExLog;
static TIMEOUT: Duration = Duration::from_secs(10);

/// `CaptureWorker` state
#[derive(Debug)]
struct CustomData {
    /// top level path to write video for this stream
    storage_path: PathBuf,
    /// splitmuxsink and associated video sink pad, used by gstreamer
    /// callbacks
    mkv_sink_pad: (Element, Pad),
    /// set once video pad is connected
    video_appsink: Option<AppSink>,
    /// set if audio stream is provided
    audio_appsink: Option<AppSink>,
    /// current filename
    current_filename: Option<String>,
    /// `last_frame_time` is set to detect a hung process not
    /// producing frames
    last_frame_time: Instant,
}

impl CustomData {
    pub fn new(storage_path: PathBuf) -> Self {
        let mkv_sink = gst::ElementFactory::make("splitmuxsink")
            .name("sink")
            .property("async-finalize", true)
            .property("max-size-bytes", 15_000_000u64)
            .property_from_str("muxer-factory", "matroskamux")
            .build()
            .expect("Could not create sink element.");

        // We request the video pad here because if we wait until we
        // get the on_pad_connect signal from `rtspsrc` then we might
        // get the audio first if you try to sink video after audio
        // `splitmuxsink` complains it already set the headers.
        let video_sink_pad = mkv_sink
            .request_pad_simple("video")
            .expect("Failed to get video sink pad from convert");

        Self {
            storage_path,
            mkv_sink_pad: (mkv_sink, video_sink_pad),
            video_appsink: None,
            audio_appsink: None,
            current_filename: None,
            last_frame_time: Instant::now(),
        }
    }
}

fn uuid_to_filename(parent_path: &std::path::Path, uuid: Uuid) -> std::path::PathBuf {
    let uuid_ts = uuid.get_timestamp().expect("failed to get uuid timestamp");
    let (secs, nsecs) = uuid_ts.to_unix();

    let secs = i64::try_from(secs).expect("overflow converting secs timestamp");

    let ts =
        chrono::DateTime::from_timestamp(secs, nsecs).expect("failed to build chrono datetime");

    let path = format!("{}/{}.mkv", ts.format("%Y/%m/%d/%H"), uuid);

    parent_path.join(&path)
}

fn create_video_branch(depay_name: &str, parser_name: &str) -> Bin {
    let bin = gst::Bin::with_name("video_sink_bin");

    let depay = gst::ElementFactory::make(depay_name)
        .name("video_depay")
        .property("request-keyframe", true)
        .build()
        .expect("Failed to build depay element.");

    let tee_parser = gst::ElementFactory::make(parser_name)
        .name("tee_parser")
        .build()
        .expect("failed to create tee parser");

    bin.add_many([&depay, &tee_parser])
        .expect("Failed to add depay");

    tee_parser.set_property("disable-passthrough", true);
    tee_parser.set_property("config-interval", -1i32);

    depay
        .sync_state_with_parent()
        .expect("Failed to sync depay state with parent");

    tee_parser
        .sync_state_with_parent()
        .expect("Failed to sync tee_parser state with parent");

    let depay_sink_pad = depay
        .static_pad("sink")
        .expect("Failed to get depay sink pad.");

    let tee = gst::ElementFactory::make("tee")
        .name("video_tee")
        .build()
        .expect("Failed to create video_tee element");

    let mkv_queue = gst::ElementFactory::make("queue")
        .name("mkv_video_queue")
        .build()
        .expect("Failed to create video_queue element");

    let app_queue = gst::ElementFactory::make("queue")
        .name("app_video_queue")
        .build()
        .expect("failed to build app video queue element");

    bin.add_many([&tee, &mkv_queue, &app_queue])
        .expect("Failed to add elements to bin");

    gst::Element::link_many([&depay, &tee_parser, &tee])
        .expect("failed to link depay,tee_parser,tee");

    let tee_mkv_pad = tee
        .request_pad_simple("src_%u")
        .expect("Failed to get tee mkv pad");

    let mkv_queue_pad = mkv_queue
        .static_pad("sink")
        .expect("Failed to get mkv video queue pad");

    tee_mkv_pad
        .link(&mkv_queue_pad)
        .expect("Failed to link tee mkv pad");

    let tee_app_pad = tee
        .request_pad_simple("src_%u")
        .expect("failed to get tee app pad");

    let app_queue_pad = app_queue
        .static_pad("sink")
        .expect("failed to get app video queue pad");

    tee_app_pad
        .link(&app_queue_pad)
        .expect("failed to link tee app pad");

    let mkv_parser = gst::ElementFactory::make(parser_name)
        .name("mkv_parser")
        .build()
        .expect("failed to build mkv parser");

    let appsink_parser = gst::ElementFactory::make(parser_name)
        .name("appsink_parser")
        .build()
        .expect("failed to build appsink parser");

    bin.add_many([&mkv_parser, &appsink_parser])
        .expect("failed to add video bin parsers");

    appsink_parser.set_property("config-interval", -1i32);
    appsink_parser.set_property("disable-passthrough", true);

    mkv_parser
        .sync_state_with_parent()
        .expect("Failed to sync mkv_parser state with parent");

    appsink_parser
        .sync_state_with_parent()
        .expect("Failed to sync appsink_parser state with parent");

    mkv_queue
        .link(&mkv_parser)
        .expect("failed to link mkv parser");

    // For appsink video we need to add a caps filter to ensure the
    // NALs are in the annexb format.

    // Create a caps filter for Annex B format
    let h264_caps = gst::Caps::builder("video/x-h264")
        .field("alignment", "au")
        .field("stream-format", "byte-stream") // This is the Annex B format
        .build();

    let capsfilter = gst::ElementFactory::make("capsfilter")
        .property("caps", h264_caps)
        .build()
        .expect("Failed to create capsfilter");

    bin.add_many([&capsfilter])
        .expect("Failed to add capsfilter");
    capsfilter.sync_state_with_parent().expect("Failed sync");

    gstreamer::Element::link_many([&app_queue, &appsink_parser, &capsfilter])
        .expect("failed to link mkv queue");

    // Ghost Pads

    let ghost_sink = gst::GhostPad::builder_with_target(&depay_sink_pad)
        .expect("failed to get ghost sink builder")
        .name("sink")
        .build();

    bin.add_pad(&ghost_sink).expect("failed to add ghost sink");

    let mkv_src = mkv_parser
        .static_pad("src")
        .expect("failed to get mkv parser src pad");

    let ghost_src = gst::GhostPad::builder_with_target(&mkv_src)
        .expect("Failed to get ghost src builder")
        .name("mkv_src")
        .build();

    ghost_src
        .set_active(true)
        .expect("failed to set ghost_src as active");

    bin.add_pad(&ghost_src).expect("failed to add ghost src");

    let app_src = capsfilter
        .static_pad("src")
        .expect("failed to get app queue src pad");

    let app_ghost_src = gst::GhostPad::builder_with_target(&app_src)
        .expect("Failed to get app ghost src builder")
        .name("app_src")
        .build();

    app_ghost_src
        .set_active(true)
        .expect("failed to set app_ghost_src as active");

    bin.add_pad(&app_ghost_src)
        .expect("failed to add app ghost src");

    bin
}

fn create_audio_branch(depay_name: &str, parser_name: Option<&str>) -> Bin {
    let bin = gst::Bin::with_name("audio_sink_bin");

    let depay = gst::ElementFactory::make(depay_name)
        .name(depay_name)
        .build()
        .expect("Failed to build depay element.");

    bin.add_many([&depay]).expect("Failed to add depay");

    depay
        .sync_state_with_parent()
        .expect("Failed to sync depay state with parent");

    let depay_sink_pad = depay
        .static_pad("sink")
        .expect("Failed to get depay sink pad.");

    let tee = gst::ElementFactory::make("tee")
        .name("video_tee")
        .build()
        .expect("Failed to create video_tee element");

    let mkv_queue = gst::ElementFactory::make("queue")
        .name("mkv_video_queue")
        .build()
        .expect("Failed to create video_queue element");

    let app_queue = gst::ElementFactory::make("queue")
        .name("app_audio_queue")
        .build()
        .expect("failed to build app audio queue element");

    bin.add_many([&tee, &mkv_queue, &app_queue])
        .expect("Failed to add elements to bin");

    if let Some(parser_name) = parser_name {
        let tee_parser = gst::ElementFactory::make(parser_name)
            .name("audio_tee_parser")
            .build()
            .expect("failed to create audio tee parser");

        bin.add_many([&tee_parser])
            .expect("Failed to add tee_parser");

        tee_parser
            .sync_state_with_parent()
            .expect("Failed to sync tee_parser state with parent");

        gst::Element::link_many([&depay, &tee_parser, &tee])
            .expect("failed to link depay,tee_parser,tee");
    } else {
        // We don't need a parser, eg for pcm
        gst::Element::link_many([&depay, &tee]).expect("failed to link depay,tee");
    }

    let tee_mkv_pad = tee
        .request_pad_simple("src_%u")
        .expect("Failed to get tee mkv pad");

    let mkv_queue_pad = mkv_queue
        .static_pad("sink")
        .expect("Failed to get mkv video queue pad");

    tee_mkv_pad
        .link(&mkv_queue_pad)
        .expect("Failed to link tee mkv pad");

    let tee_app_pad = tee
        .request_pad_simple("src_%u")
        .expect("failed to get tee app pad");

    let app_queue_pad = app_queue
        .static_pad("sink")
        .expect("failed to get app video queue pad");

    tee_app_pad
        .link(&app_queue_pad)
        .expect("failed to link tee app pad");

    // Ghost Pads

    let ghost_sink = gst::GhostPad::builder_with_target(&depay_sink_pad)
        .expect("failed to get ghost sink builder")
        .name("sink")
        .build();

    bin.add_pad(&ghost_sink).expect("failed to add ghost sink");

    let mkv_src = mkv_queue
        .static_pad("src")
        .expect("failed to get mkv parser src pad");

    let ghost_src = gst::GhostPad::builder_with_target(&mkv_src)
        .expect("Failed to get ghost src builder")
        .name("mkv_src")
        .build();

    ghost_src
        .set_active(true)
        .expect("failed to set ghost_src as active");

    bin.add_pad(&ghost_src).expect("failed to add ghost src");

    let app_src = app_queue
        .static_pad("src")
        .expect("failed to get app queue src pad");

    let app_ghost_src = gst::GhostPad::builder_with_target(&app_src)
        .expect("Failed to get app ghost src builder")
        .name("app_src")
        .build();

    app_ghost_src
        .set_active(true)
        .expect("failed to set app_ghost_src as active");

    bin.add_pad(&app_ghost_src)
        .expect("failed to add app ghost src");

    bin
}

fn handle_video_sample(
    data_weak: &Weak<Mutex<CustomData>>,
    appsink: &AppSink,
) -> Result<gst::FlowSuccess, gst::FlowError> {
    let Some(data) = data_weak.upgrade() else {
        error!("Failed to upgrade the weak reference");
        return Err(gst::FlowError::CustomError);
    };
    let mut d = data.lock().unwrap();
    d.last_frame_time = Instant::now();
    drop(d);

    let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
    let buffer = sample.buffer().expect("failed to get sample buffer");
    let map = buffer.map_readable().expect("failed to get buffer map");
    let data = map.as_slice();

    if buffer.pts().is_none() {
        error!("Buffer without pts!");
        return Ok(gst::FlowSuccess::Ok);
    }

    let pts_90khz = buffer
        .pts()
        .map(|pts| pts.nseconds() * 90_000 / 1_000_000_000)
        .expect("failed to get buffer pts");

    let mut nal_count = 0;
    let msg = CaptureMessage::Packet {
        data: data.to_owned(),
        timestamp: i64::try_from(pts_90khz).expect("i64 overflow"),
        duration: 100,
    };
    exserial::print_message(msg);
    nal_count += 1;

    debug!(
        "SAMPLE PTS: {} 90Hz pts: {:?} offset: {}, NAL COUNT: {}",
        buffer.pts().expect("failed to get pts"),
        pts_90khz,
        buffer.offset(),
        nal_count
    );

    Ok(gst::FlowSuccess::Ok)
}

fn handle_audio_sample(
    codec: models::AudioCodec,
    appsink: &AppSink,
) -> Result<gst::FlowSuccess, gst::FlowError> {
    let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
    let buffer = sample.buffer().expect("failed to get audio sample buffer");
    let map = buffer
        .map_readable()
        .expect("failed to get audio buffer map");
    let data = map.as_slice();

    let pts_90khz = buffer
        .pts()
        .map_or(0, |pts| pts.nseconds() * 90_000 / 1_000_000_000);

    let msg = CaptureMessage::AudioPacket {
        codec,
        data: data.to_owned(),
        timestamp: i64::try_from(pts_90khz).unwrap_or(0),
    };
    exserial::print_message(msg);

    Ok(gst::FlowSuccess::Ok)
}

fn handle_file_location_request(data_weak: &Weak<Mutex<CustomData>>) -> gstreamer::glib::Value {
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    let Some(data) = data_weak.upgrade() else {
        panic!("Failed to upgrade the weak reference");
    };
    let mut d = data.lock().unwrap();

    if let Some(current_filename) = &d.current_filename {
        let msg = CaptureMessage::EndFile {
            filename: current_filename.clone(),
            end_time: timestamp.clone(),
        };
        exserial::print_message(msg);
    }

    let id = Uuid::now_v7();
    let path = uuid_to_filename(&d.storage_path, id);
    let parent = path.parent().expect("failed to get new file parent");
    create_dir_all(parent).expect("failed to create parent directory");
    debug!("New file: {}", path.display());

    let msg = CaptureMessage::NewFile {
        filename: path.to_string_lossy().to_string(),
        begin_time: timestamp,
    };
    exserial::print_message(msg);
    d.current_filename = Some(path.to_string_lossy().to_string());
    drop(d);
    path.into()
}

fn handle_connect_pad_added(data_weak: Weak<Mutex<CustomData>>, src: &Element, src_pad: &Pad) {
    info!("Received new pad {} from {}", src_pad.name(), src.name());

    let Some(data) = data_weak.upgrade() else {
        error!("Failed to upgrade the weak reference");
        return;
    };
    let mut d = data.lock().unwrap();

    src.downcast_ref::<gstreamer::Bin>()
        .expect("src downcast failed.")
        .debug_to_dot_file_with_ts(gstreamer::DebugGraphDetails::all(), "pad-added");

    let new_pad_caps = src_pad
        .current_caps()
        .expect("Failed to get caps of new pad.");
    let new_pad_struct = new_pad_caps
        .structure(0)
        .expect("Failed to get first structure of caps.");
    let new_pad_type = new_pad_struct.name();

    let media = new_pad_struct.get::<&str>("media").unwrap_or_default();
    let encoding_name = new_pad_struct
        .get::<&str>("encoding-name")
        .unwrap_or_default();

    if media == "video" && d.video_appsink.is_some() {
        info!("Video is already linked. Ignoring.");
        return;
    }

    let (depayloader_name, parser_name) = match (&media, &encoding_name) {
        (&"video", &"H264") => (Some("rtph264depay"), Some("h264parse")),
        // (&"video", &"H265") => (Some("rtph265depay"), Some("h265parse")),
        (&"audio", &"OPUS") => (Some("rtpopusdepay"), Some("opusparse")),
        (&"audio", &"MPEG4-GENERIC" | &"AAC") => (Some("rtpmp4gdepay"), Some("aacparse")),
        (&"audio", &"PCMU") => (Some("rtppcmudepay"), None),
        (&"audio", &"PCMA") => (Some("rtppcmadepay"), None),
        _ => (None, None),
    };

    info!(
        "New pad type {}, {} {}",
        &new_pad_type, &media, &encoding_name
    );

    if let ("video", Some(depay_name), Some(parser_name), &None) =
        (media, depayloader_name, parser_name, &d.video_appsink)
    {
        let bin = create_video_branch(depay_name, parser_name);
        let pipeline = src
            .parent()
            .expect("Failed to get src parent")
            .downcast::<gst::Pipeline>()
            .expect("failed to get unwrap src parent");

        pipeline.add_many([&bin]).expect("Failed to add bin");

        let bin_sink_pad = bin.static_pad("sink").expect("failed to get bin sink pad");

        let bin_mkv_src_pad = bin
            .static_pad("mkv_src")
            .expect("failed to get bin src pad");

        src_pad
            .link(&bin_sink_pad)
            .expect("failed to link new_src_pad to bin_sink_pad");

        // the bin pipeline to appsink
        let appsink = AppSink::builder().name("video_appsink").sync(false).build();

        pipeline
            .add_many([appsink.upcast_ref::<gst::Element>()])
            .expect("failed to add video appsink to pipeline");

        appsink.set_callbacks(
            AppSinkCallbacks::builder()
                .new_sample(move |appsink: &AppSink| {
                    handle_video_sample(&data_weak.clone(), appsink)
                })
                .build(),
        );

        let bin_app_src_pad = bin
            .static_pad("app_src")
            .expect("failed to get bin app_src pad");
        let appsink_sink_pad = appsink
            .static_pad("sink")
            .expect("failed to get appsink src pad");
        bin_app_src_pad
            .link(&appsink_sink_pad)
            .expect("failed to link appsink_sink_pad");
        appsink
            .sync_state_with_parent()
            .expect("failed to sync video appsink state");
        d.video_appsink = Some(appsink);

        // link the bin pipeline to the mkv splitmuxsink
        bin_mkv_src_pad
            .link(&d.mkv_sink_pad.1)
            .expect("linking bin to video sink failed");

        info!("Link succeeded (type {new_pad_type}).");

        bin.sync_state_with_parent()
            .expect("failed to sync bin state");
    } else if let ("audio", Some(depay_name), &None) = (media, depayloader_name, &d.audio_appsink) {
        // Determine fi this is a WebRTC-compatible codec
        let audio_codec = match encoding_name {
            "OPUS" => Some(models::AudioCodec::Opus),
            "PCMA" => Some(models::AudioCodec::Pcma),
            "PCMU" => Some(models::AudioCodec::Pcmu),
            _ => None,
        };
        let bin = create_audio_branch(depay_name, parser_name);

        let pipeline = src
            .parent()
            .expect("Failed to get src parent")
            .downcast::<gst::Pipeline>()
            .expect("failed to get unwrap src parent");

        pipeline.add_many([&bin]).expect("Failed to add bin");

        let bin_sink_pad = bin.static_pad("sink").expect("failed to get bin sink pad");

        let bin_mkv_src_pad = bin
            .static_pad("mkv_src")
            .expect("failed to get bin src pad");

        src_pad
            .link(&bin_sink_pad)
            .expect("failed to link new_src_pad to bin_sink_pad");

        // the bin pipeline to appsink
        let appsink = AppSink::builder().name("audio_appsink").sync(false).build();

        pipeline
            .add_many([appsink.upcast_ref::<gst::Element>()])
            .expect("failed to add audio appsink to pipeline");

        if let Some(codec) = audio_codec {
            appsink.set_callbacks(
                AppSinkCallbacks::builder()
                    .new_sample(move |appsink: &AppSink| {
                        handle_audio_sample(codec.clone(), appsink)
                    })
                    .build(),
            );
        } else {
            // For non-webrtc compatible codecs, just draint the samples
            appsink.set_callbacks(
                AppSinkCallbacks::builder()
                    .new_sample(|appsink: &AppSink| {
                        let _ = appsink.pull_sample();
                        Ok(gst::FlowSuccess::Ok)
                    })
                    .build(),
            );
        }

        let bin_app_src_pad = bin
            .static_pad("app_src")
            .expect("failed to get bin app_src pad");
        let appsink_sink_pad = appsink
            .static_pad("sink")
            .expect("failed to get appsink src pad");
        bin_app_src_pad
            .link(&appsink_sink_pad)
            .expect("failed to link appsink_sink_pad");
        appsink
            .sync_state_with_parent()
            .expect("failed to sync video appsink state");
        d.audio_appsink = Some(appsink);

        let audio_sink_pad = d
            .mkv_sink_pad
            .0
            .request_pad_simple("audio_%u")
            .expect("Failed to get video sink pad from convert");
        drop(d);
        // link the bin pipeline to the mkv splitmuxsink
        bin_mkv_src_pad
            .link(&audio_sink_pad)
            .expect("linking bin to audio sink failed");

        info!("Link succeeded (type {new_pad_type}).");

        bin.sync_state_with_parent()
            .expect("failed to sync bin state");
    } else {
        error!("Unknown RTP encoding: {media} / {encoding_name}");
    }
}

fn main() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .expect("failed to initialize logger");

    // Initialize GStreamer
    gstreamer::init().expect("gstreamer::init failed");

    if std::env::args().len() < 3 {
        println!("USAGE: capture_worker <rtsp_url> <storage_path>");
        return;
    }

    let url = std::env::args().nth(1).expect("Failed to get url");
    let storage_path: std::path::PathBuf = std::env::args()
        .nth(2)
        .expect("Failed to get storage path")
        .into();
    let data = CustomData::new(storage_path);

    let source = gst::ElementFactory::make("rtspsrc")
        .name("source")
        .property_from_str("location", &url)
        .build()
        .expect("Could not create source element.");

    let pipeline = gst::Pipeline::with_name("capture-pipeline");

    pipeline
        .add_many([&source, &data.mkv_sink_pad.0])
        .expect("Failed to add elements to pipeline.");

    let data: Arc<Mutex<CustomData>> = Arc::new(Mutex::new(data));
    let data_weak = Arc::downgrade(&data);

    data.lock()
        .unwrap()
        .mkv_sink_pad
        .0
        .connect("format-location-full", false, {
            let data_weak = data_weak.clone();

            move |_el| -> Option<gstreamer::glib::Value> {
                Some(handle_file_location_request(&data_weak.clone()))
            }
        });

    // Connect the pad-added signal
    source.connect_pad_added({
        let data_weak = data_weak.clone();
        move |src, src_pad| {
            handle_connect_pad_added(data_weak.clone(), src, src_pad);
        }
    });

    // Start playing
    pipeline
        .set_state(gstreamer::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state");

    // Wait until error or EOS
    let bus = pipeline.bus().unwrap();
    loop {
        let data_weak = data_weak.clone();
        if let Some(msg) = bus.timed_pop(gstreamer::ClockTime::from_seconds(3)) {
            use gstreamer::MessageView;
            match msg.view() {
                MessageView::Error(err) => {
                    error!(
                        "Error received from element {:?} {}",
                        err.src().map(gstreamer::prelude::GstObjectExt::path_string),
                        err.error()
                    );
                    error!("Debugging information: {:?}", err.debug());
                    break;
                }
                MessageView::StateChanged(state_changed) => {
                    if state_changed.src().is_some_and(|s| s == &pipeline) {
                        debug!(
                            "Pipeline state changed from {:?} to {:?}",
                            state_changed.old(),
                            state_changed.current()
                        );
                    }
                }
                MessageView::Eos(..) => break,

                MessageView::Progress(_progress) => {}
                _ => {}
            }
        } else {
            // Timeout occurrred - check for hung stream
            let data = data_weak.upgrade().expect("failed to get data for timeout");

            let d = data.lock().unwrap();

            let last_frame_duration = Instant::now().duration_since(d.last_frame_time);
            drop(d);
            if last_frame_duration > TIMEOUT {
                error!(
                    "Haven't received frame for {} seconds. Which is longer than the configured timeout {} seconds. Exiting...",
                    last_frame_duration.as_secs(),
                    TIMEOUT.as_secs()
                );
                break;
            }
        }
    }

    pipeline
        .set_state(gstreamer::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");
}
