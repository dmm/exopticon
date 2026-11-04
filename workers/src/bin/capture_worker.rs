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
    collections::HashMap,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
    thread,
    time::{Duration, Instant},
};

use chrono::{DateTime, SecondsFormat, TimeDelta, Utc};
use exserial::{exlog::ExLog, models::CaptureMessage};
use gstreamer::{
    self as gst, Bin, Element, Pad, PadProbeId, PadProbeReturn, PadProbeType,
    glib::object::{Cast, ObjectExt},
    prelude::{
        ClockExt, ElementExt, ElementExtManual, GstBinExtManual, GstObjectExt, GstValueExt, PadExt,
        PadExtManual, TagSetterExtManual,
    },
};
use gstreamer_app::{AppSink, AppSinkCallbacks};
use log::{debug, error, info, trace};
use serde::Serialize;
use uuid::Uuid;

static LOGGER: ExLog = ExLog;
static TIMEOUT: Duration = Duration::from_secs(10);
static STARTUP_PREROLL_TIMEOUT: Duration = Duration::from_secs(1);
static MATROSKA_TIMECODE_SCALE: gst::ClockTime = gst::ClockTime::from_mseconds(1);
const SHUTDOWN_MESSAGE: &str = "capture-worker-shutdown";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ClockAnchorMetadata {
    pipeline_running_time_ns: u64,
    utc_time: String,
}

/// mkv container session data with session clock - utc wall time
/// anchor and session id
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureTimelineMetadata {
    capture_session_id: Uuid,
    // MKV timestamps are session running times (at the configured timecode
    // precision): UTC = anchor UTC + MKV timestamp - anchor running time.
    clock_anchor: ClockAnchorMetadata,
}

#[derive(Clone, Debug)]
struct CaptureClock {
    session_id: Uuid,
    anchor_running_time: gst::ClockTime,
    anchor_utc: DateTime<Utc>,
    last_reference_running_time: Option<gst::ClockTime>,
}

impl CaptureClock {
    fn anchored_to(element: &Element) -> Option<Self> {
        // Bracket the GStreamer clock read with UTC reads and use their midpoint.
        // This keeps the two clocks paired without treating a media buffer's PTS
        // as either running time or wall-clock time.
        let utc_before = Utc::now();
        let anchor_running_time = element.current_running_time()?;
        let utc_after = Utc::now();
        let half_sample_interval = utc_after
            .signed_duration_since(utc_before)
            .num_nanoseconds()
            .map_or(TimeDelta::zero(), |ns| TimeDelta::nanoseconds(ns / 2));

        Some(Self {
            session_id: Uuid::new_v4(),
            anchor_running_time,
            anchor_utc: utc_before + half_sample_interval,
            last_reference_running_time: None,
        })
    }

    fn metadata_for_reference(
        &mut self,
        element: &Element,
        reference_running_time: gst::ClockTime,
    ) -> Option<(Uuid, ClockAnchorMetadata)> {
        if self
            .last_reference_running_time
            .is_some_and(|last_origin| reference_running_time < last_origin)
        {
            info!(
                "Pipeline running time moved backwards from {:?} to {reference_running_time}; starting a new capture session",
                self.last_reference_running_time
            );
            *self = Self::anchored_to(element)?;
        }

        self.last_reference_running_time = Some(reference_running_time);
        Some((
            self.session_id,
            ClockAnchorMetadata {
                pipeline_running_time_ns: self.anchor_running_time.nseconds(),
                utc_time: self.anchor_utc.to_rfc3339_opts(SecondsFormat::Nanos, true),
            },
        ))
    }

    fn utc_at_running_time(&self, running_time: gst::ClockTime) -> Option<DateTime<Utc>> {
        let delta_ns = i128::from(running_time.nseconds())
            .checked_sub(i128::from(self.anchor_running_time.nseconds()))?;
        self.anchor_utc
            .checked_add_signed(TimeDelta::nanoseconds(i64::try_from(delta_ns).ok()?))
    }
}

#[derive(Clone, Debug)]
struct FragmentCapture {
    filename: String,
    begin_running_time: gst::ClockTime,
    capture_clock: CaptureClock,
}

#[derive(Debug)]
struct BlockedPrerollPad {
    pad: Pad,
    probe_id: PadProbeId,
}

#[derive(Debug, Default)]
struct StartupPreroll {
    complete: bool,
    timeout_scheduled: bool,
    blocked_pads: Vec<BlockedPrerollPad>,
}

#[derive(Debug)]
struct PrerollBlockResult {
    blocked: bool,
    start_timeout: bool,
}

impl StartupPreroll {
    fn block_pad(&mut self, pad: &Pad, stream_name: &'static str) -> PrerollBlockResult {
        if self.complete {
            return PrerollBlockResult {
                blocked: false,
                start_timeout: false,
            };
        }

        let start_timeout = !self.timeout_scheduled;
        self.timeout_scheduled = true;

        let probe_id = pad
            .add_probe(PadProbeType::BLOCK_DOWNSTREAM, move |_, _| {
                debug!("Holding {stream_name} mux branch during startup preroll");
                PadProbeReturn::Ok
            })
            .expect("failed to add startup preroll probe");

        self.blocked_pads.push(BlockedPrerollPad {
            pad: pad.clone(),
            probe_id,
        });

        PrerollBlockResult {
            blocked: true,
            start_timeout,
        }
    }

    fn finish(&mut self, reason: &str) {
        if self.complete {
            return;
        }

        self.complete = true;
        let blocked_count = self.blocked_pads.len();

        for blocked in self.blocked_pads.drain(..) {
            blocked.pad.remove_probe(blocked.probe_id);
        }

        debug!("Startup preroll complete via {reason}; released {blocked_count} mux branch(es)");
    }
}

/// Serializes branch attachment with startup preroll completion.
/// Recording callbacks must never acquire this state's lock.
#[derive(Debug, Default)]
struct StreamSetupState {
    /// prevents new branches from being attached once shutdown starts
    shutting_down: bool,
    /// set once the video branch is fully connected and synchronized
    video_attached: bool,
    /// set once the audio branch is fully connected, including drained late audio
    audio_attached: bool,
    /// blocks mux branches during the startup attachment window
    startup_preroll: StartupPreroll,
}

/// Recording bookkeeping shared by splitmux callbacks and the bus loop.
#[derive(Debug, Default)]
struct RecordingState {
    /// muxer created for the fragment whose location is requested next
    pending_muxer: Option<Element>,
    /// files currently being written or finalized, keyed by splitmux fragment ID
    fragments: HashMap<u32, FragmentCapture>,
    /// identity and UTC mapping for the current `GStreamer` running-time domain
    capture_clock: Option<CaptureClock>,
}

fn create_recording_sink() -> Element {
    let muxer_properties = gst::Structure::builder("properties")
        // Keep every fragment in the capture session's running-time domain.
        .field("offset-to-zero", false)
        .field(
            "timecodescale",
            i64::try_from(MATROSKA_TIMECODE_SCALE.nseconds())
                .expect("Matroska timecode scale exceeds i64"),
        )
        .field("writing-app", "EXOPTICON")
        .build();

    gst::ElementFactory::make("splitmuxsink")
        .name("sink")
        .property("async-finalize", true)
        .property("max-size-bytes", 15_000_000u64)
        .property_from_str("muxer-factory", "matroskamux")
        .property("muxer-properties", muxer_properties)
        .build()
        .expect("Could not create sink element.")
}

fn caps_are_compatible(old: &gst::CapsRef, new: &gst::CapsRef) -> bool {
    let (Some(old_s), Some(new_s)) = (old.structure(0), new.structure(0)) else {
        return false;
    };

    // Fields where presence/absence matters and values must match
    let significant_fields = [
        "width",
        "height",
        "profile",
        "level",
        "stream-format",
        "chroma-format",
        "bit-depth-luma",
        "bit-depth-chroma",
    ];

    for field in &significant_fields {
        let old_serialized = old_s.value(*field).ok().map(|v| v.serialize().ok());

        let new_serialized = new_s.value(*field).ok().map(|v| v.serialize().ok());

        match (&old_serialized, &new_serialized) {
            (Some(o), Some(n)) => {
                if o != n {
                    return false;
                }
            }
            (None, None) => {}
            _ => return false,
        }
    }

    true
}

fn add_ghost_pad(bin: &Bin, name: &str, target: &Pad) {
    let ghost_pad = gst::GhostPad::builder_with_target(target)
        .expect("failed to get ghost pad builder")
        .name(name)
        .build();

    // Source pads are activated explicitly; sink pads follow the bin's state.
    if target.direction() == gst::PadDirection::Src {
        ghost_pad
            .set_active(true)
            .expect("failed to activate ghost source pad");
    }

    bin.add_pad(&ghost_pad).expect("failed to add ghost pad");
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

    let tee_sink_pad = tee.static_pad("sink").expect("failed to get tee sink pad");
    tee_sink_pad.add_probe(PadProbeType::BUFFER, move |pad, info| {
        let Some(buffer) = info.buffer_mut() else {
            return PadProbeReturn::Ok;
        };
        if buffer.pts().is_none() {
            let pts = (|| {
                let element = pad.parent_element()?;
                let clock = element.clock()?;
                let base_time = element.base_time()?;
                Some(clock.time().saturating_sub(base_time))
            })();
            match pts {
                Some(rt) => {
                    debug!("Assigning running time PTS {rt} to PTS-less buffer");
                    buffer.make_mut().set_pts(rt);
                }
                None => return PadProbeReturn::Drop,
            }
        }
        PadProbeReturn::Ok
    });

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

    let mkv_capsfilter = gst::ElementFactory::make("capsfilter")
        .name("mkv_capsfilter")
        .property(
            "caps",
            gst::Caps::builder("video/x-h264")
                .field("stream-format", "avc")
                .field("alignment", "au")
                .build(),
        )
        .build()
        .expect("Failed to create mkv capsfilter");

    let appsink_parser = gst::ElementFactory::make(parser_name)
        .name("appsink_parser")
        .build()
        .expect("failed to build appsink parser");

    bin.add_many([&mkv_parser, &mkv_capsfilter, &appsink_parser])
        .expect("failed to add video bin parsers");

    appsink_parser.set_property("config-interval", -1i32);
    appsink_parser.set_property("disable-passthrough", true);

    mkv_parser
        .sync_state_with_parent()
        .expect("Failed to sync mkv_parser state with parent");

    mkv_capsfilter
        .sync_state_with_parent()
        .expect("Failed to sync mkv_capsfilter");

    appsink_parser
        .sync_state_with_parent()
        .expect("Failed to sync appsink_parser state with parent");

    let mkv_capsfilter_sink = mkv_capsfilter
        .static_pad("sink")
        .expect("failed to get capsfilter sink pad");

    let initial_caps: Arc<Mutex<Option<gst::Caps>>> = Arc::new(Mutex::new(None));

    mkv_capsfilter_sink.add_probe(PadProbeType::EVENT_DOWNSTREAM, move |pad, info| {
        let Some(event) = info.event() else {
            return PadProbeReturn::Ok;
        };
        let gst::EventView::Caps(caps_ev) = event.view() else {
            return PadProbeReturn::Ok;
        };

        let mut stored = initial_caps.lock().unwrap();
        let new_caps = caps_ev.caps();

        match stored.as_ref() {
            None => {
                // First caps — allow through
                *stored = Some(new_caps.to_owned());
                drop(stored);
                PadProbeReturn::Ok
            }
            Some(old_caps) if caps_are_compatible(old_caps, new_caps) => {
                // Harmless change (e.g., framerate drift) — suppress it
                log::info!("Suppressing compatible caps renegotiation");
                PadProbeReturn::Drop
            }
            Some(old_caps) => {
                // Significant change — post an error to trigger clean shutdown
                log::error!(
                    "Incompatible caps change detected.\n  Old: {old_caps}\n  New: {new_caps}"
                );

                if let Some(element) = pad.parent_element()
                    && let Err(e) = element.post_message(
                        gst::message::Error::builder(
                            gst::ResourceError::Failed,
                            "Video parameters changed, restarting capture",
                        )
                        .build(),
                    )
                {
                    panic!("Error posting Video parameters changed message. Time to die! {e}");
                }

                PadProbeReturn::Drop
            }
        }
    });

    // Link: mkv_queue → mkv_parser → mkv_capsfilter (→ ghost pad → splitmuxsink)
    gst::Element::link_many([&mkv_queue, &mkv_parser, &mkv_capsfilter])
        .expect("failed to link mkv branch");

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

    add_ghost_pad(&bin, "sink", &depay_sink_pad);

    let mkv_src = mkv_capsfilter
        .static_pad("src")
        .expect("failed to get mkv parser src pad");

    add_ghost_pad(&bin, "mkv_src", &mkv_src);

    let app_src = capsfilter
        .static_pad("src")
        .expect("failed to get app queue src pad");

    add_ghost_pad(&bin, "app_src", &app_src);

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

    add_ghost_pad(&bin, "sink", &depay_sink_pad);

    let mkv_src = mkv_queue
        .static_pad("src")
        .expect("failed to get mkv parser src pad");

    add_ghost_pad(&bin, "mkv_src", &mkv_src);

    let app_src = app_queue
        .static_pad("src")
        .expect("failed to get app queue src pad");

    add_ghost_pad(&bin, "app_src", &app_src);

    bin
}

fn handle_video_sample(
    last_frame_time: &Mutex<Instant>,
    appsink: &AppSink,
) -> Result<gst::FlowSuccess, gst::FlowError> {
    *last_frame_time.lock().unwrap() = Instant::now();

    let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
    let buffer = sample.buffer().expect("failed to get sample buffer");
    let map = buffer.map_readable().expect("failed to get buffer map");
    let data = map.as_slice();

    if buffer.pts().is_none() {
        error!("Buffer without pts despite pad probe — dropping");
        return Ok(gst::FlowSuccess::Ok);
    }

    let pts_90khz = buffer
        .pts()
        .map(|pts| pts.nseconds() * 90_000 / 1_000_000_000)
        .expect("failed to get buffer pts");

    let mut nal_count = 0;
    let msg = CaptureMessage::Packet {
        encoding: exserial::models::PacketEncoding::H264,
        data: data.to_owned(),
        timestamp: i64::try_from(pts_90khz).expect("i64 overflow"),
        duration: 100,
    };
    exserial::print_message(msg);
    nal_count += 1;

    trace!(
        "SAMPLE PTS: {} 90Hz pts: {:?} offset: {}, NAL COUNT: {}",
        buffer.pts().expect("failed to get pts"),
        pts_90khz,
        buffer.offset(),
        nal_count
    );

    Ok(gst::FlowSuccess::Ok)
}

fn handle_audio_sample(appsink: &AppSink) -> Result<gst::FlowSuccess, gst::FlowError> {
    let sample = appsink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
    let sample_caps = sample.caps().expect("failed to get audio sample  caps");
    let sample_caps_struct = sample_caps
        .structure(0)
        .expect("failed to get audio sample caps structure");
    let sample_type = sample_caps_struct.name();

    // Only u-law PCM is supported right now
    if sample_type != "audio/x-mulaw" {
        trace!("Invalid sample encoding \"{sample_type}\"");
        return Ok(gst::FlowSuccess::Ok);
    }

    let buffer = sample.buffer().expect("failed to get sample buffer");
    let map = buffer.map_readable().expect("failed to get buffer map");
    let data = map.as_slice();

    if buffer.pts().is_none() {
        error!("Buffer without pts despite pad probe — dropping");
        return Ok(gst::FlowSuccess::Ok);
    }

    let pts_90khz = buffer
        .pts()
        .map(|pts| pts.nseconds() * 90_000 / 1_000_000_000)
        .expect("failed to get buffer pts");

    let msg = CaptureMessage::Packet {
        encoding: exserial::models::PacketEncoding::PCMU,
        data: data.to_owned(),
        timestamp: i64::try_from(pts_90khz).expect("i64 overflow"),
        duration: 100,
    };
    exserial::print_message(msg);

    Ok(gst::FlowSuccess::Ok)
}

fn sample_running_time(sample: &gst::Sample) -> Option<gst::ClockTime> {
    let pts = sample.buffer()?.pts()?;
    let segment = sample.segment()?.downcast_ref::<gst::ClockTime>()?;
    segment.to_running_time(pts)
}

fn write_capture_timeline_tag(muxer: Element, metadata: &CaptureTimelineMetadata) {
    let metadata_json = serde_json::to_string(metadata)
        .expect("capture timeline metadata contains only serializable values");
    let tag_setter = muxer
        .dynamic_cast::<gst::TagSetter>()
        .expect("matroskamux does not implement GstTagSetter");
    tag_setter.add_tag::<gst::tags::Comment>(&metadata_json.as_str(), gst::TagMergeMode::Replace);
    debug!("MKV capture timeline metadata: {metadata_json}");
}

fn handle_muxer_added(recording_weak: &Weak<Mutex<RecordingState>>, muxer: &Element) {
    let recording = recording_weak
        .upgrade()
        .expect("recording state dropped before muxer-added");
    recording.lock().unwrap().pending_muxer = Some(muxer.clone());
}

fn handle_file_location_request(
    splitmuxsink: &Element,
    fragment_id: u32,
    first_sample: &gst::Sample,
    storage_path: &Path,
    recording_weak: &Weak<Mutex<RecordingState>>,
) -> gstreamer::glib::Value {
    let reference_running_time = sample_running_time(first_sample)
        .expect("first fragment sample does not have a PTS in a TIME segment");
    let recording = recording_weak
        .upgrade()
        .expect("recording state dropped before format-location-full");
    let filename = format!("{}.mkv", Uuid::now_v7());
    let path = storage_path.join(&filename);
    debug!("New staged file: {}", path.display());

    let mut recording = recording.lock().unwrap();
    let capture_clock = recording.capture_clock.get_or_insert_with(|| {
        CaptureClock::anchored_to(splitmuxsink)
            .expect("splitmuxsink does not have a running-time clock")
    });
    let (capture_session_id, clock_anchor) = capture_clock
        .metadata_for_reference(splitmuxsink, reference_running_time)
        .expect("splitmuxsink does not have a running-time clock");
    debug!(
        "splitmuxsink fragment start: fragment_id={fragment_id} filename={filename} capture_session_id={capture_session_id} begin_running_time_ns={}",
        reference_running_time.nseconds(),
    );
    let fragment = FragmentCapture {
        filename,
        begin_running_time: reference_running_time,
        capture_clock: capture_clock.clone(),
    };
    let pending_muxer = recording
        .pending_muxer
        .take()
        .expect("format-location-full arrived before muxer-added");
    recording.fragments.insert(fragment_id, fragment);
    drop(recording);

    let metadata = CaptureTimelineMetadata {
        capture_session_id,
        clock_anchor,
    };
    write_capture_timeline_tag(pending_muxer, &metadata);

    path.into()
}

fn closed_fragment_message(
    structure: &gst::StructureRef,
    recording_weak: &Weak<Mutex<RecordingState>>,
) -> CaptureMessage {
    let fragment_id = structure
        .get::<u32>("fragment-id")
        .expect("fragment-closed message has no fragment ID");
    let duration = structure
        .get::<gst::ClockTime>("fragment-duration")
        .expect("fragment-closed message has no duration");
    let reference_running_time = structure
        .get::<gst::ClockTime>("running-time")
        .expect("fragment-closed message has no reference running time");
    let recording = recording_weak
        .upgrade()
        .expect("recording state dropped before fragment-closed");
    let fragment = recording
        .lock()
        .unwrap()
        .fragments
        .remove(&fragment_id)
        .expect("fragment-closed arrived for an unknown fragment");
    let duration_end_running_time = fragment
        .begin_running_time
        .checked_add(duration)
        .expect("fragment running-time end overflowed");
    // On rotation, running-time is the next reference buffer's timestamp.
    // Without buffer durations, fragment-duration stops at the final frame's
    // PTS and omits its display interval. At EOS, running-time is the final
    // timestamp instead, so retain the duration-based end when it is later.
    // Both values belong to this fragment, even with async finalization.
    let end_running_time = duration_end_running_time.max(reference_running_time);
    let end_time = fragment
        .capture_clock
        .utc_at_running_time(end_running_time)
        .expect("fragment end is too far from its UTC anchor")
        .to_rfc3339_opts(SecondsFormat::Nanos, true);

    let begin_time = fragment
        .capture_clock
        .utc_at_running_time(fragment.begin_running_time)
        .expect("fragment start is too far from its UTC anchor")
        .to_rfc3339_opts(SecondsFormat::Nanos, true);

    debug!(
        "splitmuxsink fragment closed: fragment_id={fragment_id} filename={} capture_session_id={} begin_running_time_ns={} fragment_duration_ns={} reference_running_time_ns={} end_running_time_ns={} begin_time={begin_time} end_time={end_time}",
        fragment.filename,
        fragment.capture_clock.session_id,
        fragment.begin_running_time.nseconds(),
        duration.nseconds(),
        reference_running_time.nseconds(),
        end_running_time.nseconds(),
    );

    // The muxer has closed the file. The worker never touches it after handoff.
    CaptureMessage::EndFile {
        filename: fragment.filename,
        begin_time,
        end_time,
    }
}

fn schedule_startup_preroll_timeout(setup_weak: Weak<Mutex<StreamSetupState>>) {
    thread::Builder::new()
        .name("startup-preroll-timeout".to_string())
        .spawn(move || {
            thread::sleep(STARTUP_PREROLL_TIMEOUT);

            let Some(setup) = setup_weak.upgrade() else {
                return;
            };

            setup.lock().unwrap().startup_preroll.finish("timeout");
        })
        .expect("failed to spawn startup preroll timeout");
}

fn drain_unmuxed_branch(pipeline: &gst::Pipeline, src_pad: &Pad, sink_name: &str) {
    let fakesink = gst::ElementFactory::make("fakesink")
        .name(sink_name)
        .property("sync", false)
        .build()
        .expect("failed to build fakesink");

    pipeline
        .add_many([&fakesink])
        .expect("failed to add fakesink");

    let fakesink_pad = fakesink
        .static_pad("sink")
        .expect("failed to get fakesink sink pad");

    src_pad
        .link(&fakesink_pad)
        .expect("failed to link late mux branch to fakesink");

    fakesink
        .sync_state_with_parent()
        .expect("failed to sync fakesink state");
}

#[derive(Clone, Copy)]
struct VideoCodec {
    depayloader: &'static str,
    parser: &'static str,
}

#[derive(Clone, Copy)]
struct AudioCodec {
    depayloader: &'static str,
    parser: Option<&'static str>,
}

enum RtpCodec {
    Video(VideoCodec),
    Audio(AudioCodec),
}

fn select_rtp_codec(media: &str, encoding: &str) -> Option<RtpCodec> {
    match (media, encoding) {
        ("video", "H264") => Some(RtpCodec::Video(VideoCodec {
            depayloader: "rtph264depay",
            parser: "h264parse",
        })),
        ("audio", "OPUS") => Some(RtpCodec::Audio(AudioCodec {
            depayloader: "rtpopusdepay",
            parser: Some("opusparse"),
        })),
        ("audio", "MPEG4-GENERIC" | "AAC") => Some(RtpCodec::Audio(AudioCodec {
            depayloader: "rtpmp4gdepay",
            parser: Some("aacparse"),
        })),
        ("audio", "PCMU") => Some(RtpCodec::Audio(AudioCodec {
            depayloader: "rtppcmudepay",
            parser: None,
        })),
        ("audio", "PCMA") => Some(RtpCodec::Audio(AudioCodec {
            depayloader: "rtppcmadepay",
            parser: None,
        })),
        _ => None,
    }
}

fn attach_appsink(pipeline: &gst::Pipeline, bin: &Bin, name: &str, callbacks: AppSinkCallbacks) {
    let appsink = AppSink::builder().name(name).sync(false).build();
    pipeline
        .add_many([appsink.upcast_ref::<gst::Element>()])
        .expect("failed to add appsink to pipeline");
    appsink.set_callbacks(callbacks);

    let app_src_pad = bin
        .static_pad("app_src")
        .expect("failed to get bin app_src pad");
    let sink_pad = appsink
        .static_pad("sink")
        .expect("failed to get appsink sink pad");
    app_src_pad
        .link(&sink_pad)
        .expect("failed to link branch to appsink");
    appsink
        .sync_state_with_parent()
        .expect("failed to sync appsink state");
}

// The lock on StreamSetupState is held by the caller for branch
// attachment, so startup preroll can't finish before linking and
// state synch is complete.
fn attach_video_branch(
    preroll: &mut StartupPreroll,
    setup_weak: &Weak<Mutex<StreamSetupState>>,
    pipeline: &gst::Pipeline,
    src_pad: &Pad,
    codec: VideoCodec,
    video_sink_pad: &Pad,
    last_frame_time: &Arc<Mutex<Instant>>,
) {
    let bin = create_video_branch(codec.depayloader, codec.parser);
    pipeline.add_many([&bin]).expect("Failed to add bin");

    let bin_sink_pad = bin.static_pad("sink").expect("failed to get bin sink pad");
    let bin_mkv_src_pad = bin
        .static_pad("mkv_src")
        .expect("failed to get bin src pad");
    let preroll_block = preroll.block_pad(&bin_mkv_src_pad, "video");
    if preroll_block.start_timeout {
        schedule_startup_preroll_timeout(setup_weak.clone());
    }

    src_pad
        .link(&bin_sink_pad)
        .expect("failed to link new_src_pad to bin_sink_pad");

    let last_frame_time = Arc::clone(last_frame_time);
    attach_appsink(
        pipeline,
        &bin,
        "video_appsink",
        AppSinkCallbacks::builder()
            .new_sample(move |appsink: &AppSink| handle_video_sample(&last_frame_time, appsink))
            .build(),
    );

    bin_mkv_src_pad
        .link(video_sink_pad)
        .expect("linking bin to video sink failed");
    bin.sync_state_with_parent()
        .expect("failed to sync bin state");
}

fn attach_audio_branch(
    preroll: &mut StartupPreroll,
    setup_weak: &Weak<Mutex<StreamSetupState>>,
    pipeline: &gst::Pipeline,
    src_pad: &Pad,
    codec: AudioCodec,
    mkv_sink: &Element,
) {
    let bin = create_audio_branch(codec.depayloader, codec.parser);
    pipeline.add_many([&bin]).expect("Failed to add bin");

    let bin_sink_pad = bin.static_pad("sink").expect("failed to get bin sink pad");
    let bin_mkv_src_pad = bin
        .static_pad("mkv_src")
        .expect("failed to get bin src pad");
    let preroll_block = preroll.block_pad(&bin_mkv_src_pad, "audio");
    if preroll_block.start_timeout {
        schedule_startup_preroll_timeout(setup_weak.clone());
    }

    src_pad
        .link(&bin_sink_pad)
        .expect("failed to link new_src_pad to bin_sink_pad");

    attach_appsink(
        pipeline,
        &bin,
        "audio_appsink",
        AppSinkCallbacks::builder()
            .new_sample(handle_audio_sample)
            .build(),
    );

    // Requesting a pad can synchronously emit muxer-added, which only
    // locks RecordingState. The caller keeps setup locked through
    // attachment.
    if preroll_block.blocked {
        let audio_sink_pad = mkv_sink
            .request_pad_simple("audio_%u")
            .expect("Failed to get audio sink pad from splitmuxsink");
        bin_mkv_src_pad
            .link(&audio_sink_pad)
            .expect("linking bin to audio sink failed");
    } else {
        info!("Audio arrived after startup preroll; draining it outside of the MKV muxer");
        drain_unmuxed_branch(pipeline, &bin_mkv_src_pad, "late_audio_fakesink");
    }

    bin.sync_state_with_parent()
        .expect("failed to sync bin state");
}

fn handle_connect_pad_added(
    setup_weak: &Weak<Mutex<StreamSetupState>>,
    src: &Element,
    src_pad: &Pad,
    mkv_sink: &Element,
    video_sink_pad: &Pad,
    last_frame_time: &Arc<Mutex<Instant>>,
) {
    debug!("Received new pad {} from {}", src_pad.name(), src.name());

    let Some(setup) = setup_weak.upgrade() else {
        error!("Stream setup state dropped before pad-added");
        return;
    };
    // Serialize duplicate detection, attachment, and preroll
    // completion. In particular, recording callbacks invoked during
    // attachment must not lock setup.
    let mut setup = setup.lock().unwrap();
    if setup.shutting_down {
        return;
    }

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

    if (media == "video" && setup.video_attached) || (media == "audio" && setup.audio_attached) {
        info!("{media} is already linked. Ignoring.");
        return;
    }

    debug!("New pad type {new_pad_type}, {media} {encoding_name}");
    let Some(codec) = select_rtp_codec(media, encoding_name) else {
        error!("Unknown RTP encoding: {media} / {encoding_name}");
        return;
    };
    let pipeline = src
        .parent()
        .expect("Failed to get src parent")
        .downcast::<gst::Pipeline>()
        .expect("failed to get unwrap src parent");

    match codec {
        RtpCodec::Video(codec) => {
            attach_video_branch(
                &mut setup.startup_preroll,
                setup_weak,
                &pipeline,
                src_pad,
                codec,
                video_sink_pad,
                last_frame_time,
            );
            setup.video_attached = true;
        }
        RtpCodec::Audio(codec) => {
            attach_audio_branch(
                &mut setup.startup_preroll,
                setup_weak,
                &pipeline,
                src_pad,
                codec,
                mkv_sink,
            );
            setup.audio_attached = true;
        }
    }
    debug!("Link succeeded (type {new_pad_type}).");
    drop(setup);
}

fn connect_recording_sink(
    mkv_sink: &Element,
    storage_path: PathBuf,
    recording: &Arc<Mutex<RecordingState>>,
) -> Pad {
    let recording_weak = Arc::downgrade(recording);
    mkv_sink.connect("muxer-added", false, {
        let recording_weak = recording_weak.clone();

        move |values| {
            let muxer = values[1]
                .get::<Element>()
                .expect("muxer-added did not provide a GstElement");
            handle_muxer_added(&recording_weak, &muxer);
            None
        }
    });

    mkv_sink.connect("format-location-full", false, {
        move |values| -> Option<gstreamer::glib::Value> {
            let splitmuxsink = values[0]
                .get::<Element>()
                .expect("format-location-full did not provide splitmuxsink");
            let fragment_id = values[1]
                .get::<u32>()
                .expect("format-location-full did not provide a fragment ID");
            let first_sample = values[2]
                .get::<gst::Sample>()
                .expect("format-location-full did not provide the first sample");
            Some(handle_file_location_request(
                &splitmuxsink,
                fragment_id,
                &first_sample,
                &storage_path,
                &recording_weak,
            ))
        }
    });

    // Request the primary video pad only after muxer-added is connected. The
    // request creates the initial muxer and emits muxer-added synchronously.
    mkv_sink
        .request_pad_simple("video")
        .expect("Failed to get video sink pad from splitmuxsink")
}

/// Only the bus loop changes the pipeline; the input thread just wakes it.
fn monitor_shutdown(mut input: impl Read, bus: &gst::Bus) {
    if let Err(err) = io::copy(&mut input, &mut io::sink()) {
        error!("Error reading capture worker stdin: {err}");
    }
    // A flushing bus means the pipeline has already stopped.
    let _ = bus.post(gst::message::Application::new(gst::Structure::new_empty(
        SHUTDOWN_MESSAGE,
    )));
}

fn main() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Debug))
        .expect("failed to initialize logger");

    // Initialize GStreamer
    gstreamer::init().expect("gstreamer::init failed");

    if std::env::args().len() < 3 {
        println!("USAGE: capture_worker <rtsp_url> <staging_path>");
        return;
    }

    let url = std::env::args().nth(1).expect("Failed to get url");
    let storage_path: PathBuf = std::env::args()
        .nth(2)
        .expect("Failed to get storage path")
        .into();
    std::fs::create_dir_all(&storage_path).expect("failed to create camera staging directory");
    let setup = Arc::new(Mutex::new(StreamSetupState::default()));
    let recording = Arc::new(Mutex::new(RecordingState::default()));
    let last_frame_time = Arc::new(Mutex::new(Instant::now()));
    let mkv_sink = create_recording_sink();

    let source = gst::ElementFactory::make("rtspsrc")
        .name("source")
        .property_from_str("location", &url)
        .build()
        .expect("Could not create source element.");

    let pipeline = gst::Pipeline::with_name("capture-pipeline");

    pipeline
        .add_many([&source, &mkv_sink])
        .expect("Failed to add elements to pipeline.");

    let video_sink_pad = connect_recording_sink(&mkv_sink, storage_path, &recording);

    // Connect the pad-added signal
    source.connect_pad_added({
        let setup_weak = Arc::downgrade(&setup);
        let last_frame_time = Arc::clone(&last_frame_time);
        move |src, src_pad| {
            handle_connect_pad_added(
                &setup_weak,
                src,
                src_pad,
                &mkv_sink,
                &video_sink_pad,
                &last_frame_time,
            );
        }
    });

    // Start playing
    pipeline
        .set_state(gstreamer::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state");

    let bus = pipeline.bus().unwrap();
    let stdin_bus = bus.clone();
    thread::Builder::new()
        .name("capture-stdin".to_string())
        .spawn(move || monitor_shutdown(io::stdin(), &stdin_bus))
        .expect("failed to spawn capture stdin monitor");

    let recording_weak = Arc::downgrade(&recording);
    let mut shutting_down = false;
    let mut saw_eos = false;
    loop {
        // Check even when unrelated bus messages keep arriving.
        let last_frame_duration = last_frame_time.lock().unwrap().elapsed();
        if !shutting_down && last_frame_duration > TIMEOUT {
            error!(
                "Haven't received frame for {} seconds. Which is longer than the configured timeout {} seconds. Exiting...",
                last_frame_duration.as_secs(),
                TIMEOUT.as_secs()
            );
            break;
        }

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
                MessageView::Application(application)
                    if application
                        .structure()
                        .is_some_and(|structure| structure.name() == SHUTDOWN_MESSAGE) =>
                {
                    if shutting_down {
                        continue;
                    }
                    info!("Stdin closed; draining capture pipeline");
                    shutting_down = true;
                    let video_attached = {
                        let mut setup = setup.lock().unwrap();
                        setup.shutting_down = true;
                        setup.startup_preroll.finish("shutdown");
                        setup.video_attached
                    };
                    // An unconnected splitmux video pad cannot reach EOS. No
                    // video branch means there is no recording to finalize.
                    if !video_attached {
                        break;
                    }
                    if !pipeline.send_event(gst::event::Eos::new()) {
                        error!("Capture pipeline rejected shutdown EOS");
                        break;
                    }
                }
                MessageView::Eos(..) => {
                    saw_eos = true;
                    shutting_down = true;
                }

                MessageView::Element(element) => {
                    let Some(structure) = element.structure() else {
                        continue;
                    };
                    if structure.name() == "splitmuxsink-fragment-closed" {
                        exserial::print_message(closed_fragment_message(
                            structure,
                            &recording_weak,
                        ));
                    }
                }

                MessageView::Progress(_progress) => {}
                _ => {}
            }
        }
        // Async finalizers may still have fragment-closed messages to deliver
        // after pipeline EOS. Keep the bus alive until every handoff is sent.
        if saw_eos && recording.lock().unwrap().fragments.is_empty() {
            break;
        }
    }

    pipeline
        .set_state(gstreamer::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");
}

#[cfg(test)]
mod tests {
    use gst::prelude::GstBinExt;

    use super::*;

    struct StreamSetupFixture {
        pipeline: gst::Pipeline,
        source: Bin,
        sink: Element,
        video_sink_pad: Pad,
        setup: Arc<Mutex<StreamSetupState>>,
        recording: Arc<Mutex<RecordingState>>,
    }

    impl StreamSetupFixture {
        fn new() -> Self {
            gst::init().unwrap();
            let pipeline = gst::Pipeline::new();
            let source = Bin::new();
            let sink = create_recording_sink();
            pipeline.add_many([source.upcast_ref(), &sink]).unwrap();
            let recording = Arc::new(Mutex::new(RecordingState::default()));
            let video_sink_pad = connect_recording_sink(&sink, PathBuf::new(), &recording);
            let setup = Arc::new(Mutex::new(StreamSetupState {
                // Tests finish preroll explicitly instead of racing a real timer.
                startup_preroll: StartupPreroll {
                    timeout_scheduled: true,
                    ..StartupPreroll::default()
                },
                ..StreamSetupState::default()
            }));
            Self {
                pipeline,
                source,
                sink,
                video_sink_pad,
                setup,
                recording,
            }
        }

        fn attach(&self, media: &str, encoding: &str) -> Pad {
            let pad = Pad::new(gst::PadDirection::Src);
            self.source.add_pad(&pad).unwrap();
            pad.set_active(true).unwrap();
            let caps = gst::Caps::builder("application/x-rtp")
                .field("media", media)
                .field("encoding-name", encoding)
                .field(
                    "clock-rate",
                    match (media, encoding) {
                        ("video", _) => 90_000i32,
                        ("audio", "OPUS" | "MPEG4-GENERIC") => 48_000,
                        _ => 8_000,
                    },
                )
                .build();
            pad.store_sticky_event(&gst::event::Caps::new(&caps))
                .unwrap();
            handle_connect_pad_added(
                &Arc::downgrade(&self.setup),
                self.source.upcast_ref(),
                &pad,
                &self.sink,
                &self.video_sink_pad,
                &Arc::new(Mutex::new(Instant::now())),
            );
            pad
        }
    }

    #[test]
    fn startup_audio_attachment_keeps_preroll_serialized_without_locking_recording() {
        use std::sync::{
            TryLockError,
            atomic::{AtomicBool, Ordering},
        };

        let fixture = StreamSetupFixture::new();
        fixture.attach("video", "H264");
        let attachment_was_serialized = Arc::new(AtomicBool::new(false));
        fixture.pipeline.connect_element_added({
            let setup_weak = Arc::downgrade(&fixture.setup);
            let recording_weak = Arc::downgrade(&fixture.recording);
            let attachment_was_serialized = Arc::clone(&attachment_was_serialized);
            move |_, element| {
                if element.name() != "audio_sink_bin" {
                    return;
                }
                element.static_pad("mkv_src").unwrap().connect_linked({
                    let setup_weak = setup_weak.clone();
                    let recording_weak = recording_weak.clone();
                    let attachment_was_serialized = Arc::clone(&attachment_was_serialized);
                    move |_, _| {
                        let setup = setup_weak.upgrade().unwrap();
                        let recording = recording_weak.upgrade().unwrap();
                        attachment_was_serialized.store(
                            matches!(setup.try_lock(), Err(TryLockError::WouldBlock))
                                && recording.try_lock().is_ok(),
                            Ordering::SeqCst,
                        );
                    }
                });
            }
        });

        fixture.attach("audio", "PCMU");

        assert!(attachment_was_serialized.load(Ordering::SeqCst));
        let audio_pad = fixture.sink.static_pad("audio_0").unwrap();
        assert!(audio_pad.is_linked());
        assert!(fixture.pipeline.by_name("late_audio_fakesink").is_none());
        let mut setup = fixture.setup.lock().unwrap();
        assert!(setup.video_attached && setup.audio_attached);
        assert_eq!(setup.startup_preroll.blocked_pads.len(), 2);
        setup.startup_preroll.finish("test timeout");
        assert!(setup.startup_preroll.blocked_pads.is_empty());
        drop(setup);
    }

    #[test]
    fn audio_after_preroll_is_attached_and_drained_outside_the_recording() {
        let fixture = StreamSetupFixture::new();
        fixture.attach("video", "H264");
        fixture
            .setup
            .lock()
            .unwrap()
            .startup_preroll
            .finish("test timeout");

        fixture.attach("audio", "PCMU");

        assert!(fixture.setup.lock().unwrap().audio_attached);
        assert!(fixture.sink.static_pad("audio_0").is_none());
        let drain = fixture.pipeline.by_name("late_audio_fakesink").unwrap();
        let audio = fixture.pipeline.by_name("audio_sink_bin").unwrap();
        assert_eq!(
            audio.static_pad("mkv_src").unwrap().peer(),
            drain.static_pad("sink")
        );
        assert!(
            fixture
                .pipeline
                .by_name("audio_appsink")
                .unwrap()
                .static_pad("sink")
                .unwrap()
                .is_linked()
        );
    }

    #[test]
    fn supported_audio_codecs_attach_to_both_outputs() {
        for encoding in ["OPUS", "MPEG4-GENERIC", "PCMU", "PCMA"] {
            let fixture = StreamSetupFixture::new();
            let source_pad = fixture.attach("audio", encoding);

            assert!(source_pad.is_linked(), "{encoding}");
            assert!(fixture.setup.lock().unwrap().audio_attached, "{encoding}");
            assert!(fixture.sink.static_pad("audio_0").unwrap().is_linked());
            assert!(
                fixture
                    .pipeline
                    .by_name("audio_appsink")
                    .unwrap()
                    .static_pad("sink")
                    .unwrap()
                    .is_linked()
            );
            fixture
                .setup
                .lock()
                .unwrap()
                .startup_preroll
                .finish("test timeout");
        }
    }

    #[test]
    fn unsupported_codecs_leave_setup_available_for_supported_streams() {
        let fixture = StreamSetupFixture::new();
        let unsupported_video = fixture.attach("video", "H265");
        let unsupported_audio = fixture.attach("audio", "unknown");

        assert!(!unsupported_video.is_linked());
        assert!(!unsupported_audio.is_linked());
        {
            let setup = fixture.setup.lock().unwrap();
            assert!(!setup.video_attached && !setup.audio_attached);
            assert!(setup.startup_preroll.blocked_pads.is_empty());
            drop(setup);
        }
        assert!(fixture.pipeline.by_name("video_sink_bin").is_none());
        assert!(fixture.pipeline.by_name("audio_sink_bin").is_none());

        assert!(fixture.attach("video", "H264").is_linked());
        assert!(fixture.attach("audio", "PCMU").is_linked());
        let mut setup = fixture.setup.lock().unwrap();
        assert!(setup.video_attached && setup.audio_attached);
        setup.startup_preroll.finish("test timeout");
    }

    #[test]
    fn duplicate_streams_leave_the_original_branches_attached() {
        let fixture = StreamSetupFixture::new();
        let video = fixture.attach("video", "H264");
        let audio = fixture.attach("audio", "PCMU");
        let video_peer = video.peer();
        let audio_peer = audio.peer();

        let duplicate_video = fixture.attach("video", "H264");
        let duplicate_audio = fixture.attach("audio", "PCMU");

        assert!(!duplicate_video.is_linked());
        assert!(!duplicate_audio.is_linked());
        assert!(video_peer.is_some() && audio_peer.is_some());
        assert_eq!(video.peer(), video_peer);
        assert_eq!(audio.peer(), audio_peer);
        let mut setup = fixture.setup.lock().unwrap();
        assert_eq!(setup.startup_preroll.blocked_pads.len(), 2);
        setup.startup_preroll.finish("test timeout");
    }

    #[test]
    fn rotation_creates_closed_staged_recordings_without_a_parent() {
        gst::init().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let staging_path = directory.path().join(".staging/front");
        std::fs::create_dir_all(&staging_path).unwrap();
        let abandoned = staging_path.join(format!("{}.mkv", Uuid::now_v7()));
        std::fs::write(&abandoned, b"interrupted recording").unwrap();
        let recording = Arc::new(Mutex::new(RecordingState::default()));
        let sink = create_recording_sink();
        sink.set_property("max-size-bytes", 30_000u64);
        let pipeline = gst::Pipeline::new();
        let source = gst::parse::bin_from_description(
            "videotestsrc is-live=true num-buffers=120 pattern=snow ! \
             video/x-raw,width=160,height=120,framerate=30/1 ! \
             x264enc tune=zerolatency speed-preset=ultrafast key-int-max=15 bitrate=500 ! \
             h264parse",
            true,
        )
        .expect("rotation test needs videotestsrc, x264enc, and h264parse");
        pipeline.add_many([source.upcast_ref(), &sink]).unwrap();
        let video_sink_pad = connect_recording_sink(&sink, staging_path.clone(), &recording);
        source
            .static_pad("src")
            .unwrap()
            .link(&video_sink_pad)
            .unwrap();
        pipeline.set_state(gst::State::Playing).unwrap();

        let bus = pipeline.bus().unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        let weak = Arc::downgrade(&recording);
        let mut recordings = Vec::new();
        let mut saw_eos = false;
        while Instant::now() < deadline {
            let Some(message) = bus.timed_pop(gst::ClockTime::from_seconds(1)) else {
                continue;
            };
            match message.view() {
                gst::MessageView::Element(element) => {
                    let Some(structure) = element.structure() else {
                        continue;
                    };
                    if structure.name() == "splitmuxsink-fragment-closed" {
                        recordings.push(closed_fragment_message(structure, &weak));
                    }
                }
                gst::MessageView::Eos(_) => {
                    saw_eos = true;
                    break;
                }
                gst::MessageView::Error(err) => {
                    pipeline.set_state(gst::State::Null).unwrap();
                    panic!("capture failed: {} {:?}", err.error(), err.debug());
                }
                _ => {}
            }
        }
        pipeline.set_state(gst::State::Null).unwrap();
        assert!(saw_eos, "rotation test timed out");
        assert!(
            recordings.len() >= 3,
            "expected multiple completed rotations"
        );
        assert!(recording.lock().unwrap().fragments.is_empty());
        let capture_clock = recording.lock().unwrap().capture_clock.clone().unwrap();
        let mut boundaries = Vec::new();
        let mut filenames = std::collections::HashSet::new();
        for message in recordings {
            let CaptureMessage::EndFile {
                filename,
                begin_time,
                end_time,
            } = message
            else {
                panic!("unexpected completion message");
            };
            let uuid: Uuid = filename.strip_suffix(".mkv").unwrap().parse().unwrap();
            assert_eq!(uuid.get_version_num(), 7);
            assert_eq!(filename, format!("{uuid}.mkv"));
            assert!(filenames.insert(filename.clone()));
            let begin: DateTime<Utc> = begin_time.parse().unwrap();
            let end: DateTime<Utc> = end_time.parse().unwrap();
            assert!(end > begin);
            boundaries.push((begin, end));
            assert_eq!(begin_time.split('.').nth(1).unwrap().len(), 10);
            assert_eq!(end_time.split('.').nth(1).unwrap().len(), 10);
            let bytes = std::fs::read(staging_path.join(&filename)).unwrap();
            assert_eq!(&bytes[..4], &[0x1a, 0x45, 0xdf, 0xa3], "MKV header");
            // matroskamux stores the Comment as a quoted GstStructure string.
            let content = String::from_utf8_lossy(&bytes).replace('\\', "");
            let start = content.find("{\"captureSessionId\"").unwrap();
            let metadata = serde_json::Deserializer::from_str(&content[start..])
                .into_iter::<serde_json::Value>()
                .next()
                .unwrap()
                .unwrap();
            assert_eq!(
                metadata["captureSessionId"],
                capture_clock.session_id.to_string()
            );
            assert_ne!(uuid, capture_clock.session_id);
            assert_eq!(
                metadata["clockAnchor"]["pipelineRunningTimeNs"],
                capture_clock.anchor_running_time.nseconds()
            );
            assert_eq!(
                metadata["clockAnchor"]["utcTime"],
                capture_clock
                    .anchor_utc
                    .to_rfc3339_opts(SecondsFormat::Nanos, true)
            );
        }
        // Async finalizers may report completions out of order. At 30 fps,
        // integer nanosecond buffer durations can leave a one-nanosecond gap.
        boundaries.sort_unstable();
        for pair in boundaries.windows(2) {
            let gap = pair[1].0 - pair[0].1;
            assert!(gap.abs() <= TimeDelta::nanoseconds(1));
        }
        assert_eq!(std::fs::read(abandoned).unwrap(), b"interrupted recording");
    }
}
