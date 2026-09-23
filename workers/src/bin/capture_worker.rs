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
    io,
    path::PathBuf,
    sync::{
        Arc, Mutex, Weak,
        mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel},
    },
    thread,
    time::{Duration, Instant},
};

use chrono::{DateTime, SecondsFormat, TimeDelta, Utc};
use exserial::{
    exlog::ExLog,
    models::{CaptureCommand, CaptureMessage},
};
use gstreamer::{
    self as gst, Bin, Element, Pad, PadProbeId, PadProbeReturn, PadProbeType,
    glib::{
        self,
        object::{Cast, ObjectExt},
    },
    prelude::{
        ClockExt, ElementExt, ElementExtManual, GstBinExtManual, GstObjectExt, GstValueExt, PadExt,
        PadExtManual, TagSetterExtManual,
    },
};
use gstreamer_app::{AppSink, AppSinkCallbacks};
use log::{debug, error, info};
use serde::Serialize;
use uuid::Uuid;

static LOGGER: ExLog = ExLog;
static TIMEOUT: Duration = Duration::from_secs(10);
static STARTUP_PREROLL_TIMEOUT: Duration = Duration::from_secs(1);
static MATROSKA_TIMECODE_SCALE: gst::ClockTime = gst::ClockTime::from_mseconds(1);

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
struct FileReservation {
    video_unit_id: i64,
    video_file_id: i64,
    filename: String,
}

#[derive(Clone, Debug)]
struct FragmentCapture {
    video_unit_id: i64,
    video_file_id: i64,
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

        info!("Startup preroll complete via {reason}; released {blocked_count} mux branch(es)");
    }
}

#[derive(Debug)]
struct ReservationClient {
    receiver: Mutex<Receiver<Result<FileReservation, String>>>,
    timeout: Duration,
}

impl ReservationClient {
    fn start(timeout: Duration) -> Arc<Self> {
        let (sender, receiver) = sync_channel(1);
        let client = Arc::new(Self {
            receiver: Mutex::new(receiver),
            timeout,
        });

        thread::Builder::new()
            .name("capture-command-reader".to_string())
            .spawn(move || Self::read_commands(&sender))
            .expect("failed to spawn capture command reader");

        client
    }

    fn request_next() {
        exserial::print_message(CaptureMessage::ReserveFile);
    }

    fn wait_for_reservation(&self) -> Result<FileReservation, String> {
        let receiver = self
            .receiver
            .lock()
            .expect("reservation receiver lock poisoned");
        match receiver.recv_timeout(self.timeout) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => Err(format!(
                "timed out waiting {} seconds for file reservation",
                self.timeout.as_secs()
            )),
            Err(RecvTimeoutError::Disconnected) => Err(
                "capture actor command stream closed before file reservation arrived".to_string(),
            ),
        }
    }

    fn read_commands(sender: &SyncSender<Result<FileReservation, String>>) {
        let stdin = io::stdin();
        let mut stdin = stdin.lock();

        loop {
            match exserial::read_framed::<_, CaptureCommand>(&mut stdin) {
                Ok(command) => {
                    let reservation = match command {
                        CaptureCommand::FileReserved {
                            video_unit_id,
                            video_file_id,
                            filename,
                        } => Ok(FileReservation {
                            video_unit_id,
                            video_file_id,
                            filename,
                        }),
                        CaptureCommand::FileReservationFailed { message } => Err(message),
                    };

                    if sender.send(reservation).is_err() {
                        break;
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => {
                    let _ = sender.try_send(Err("capture actor command stream closed".to_string()));
                    break;
                }
                Err(err) => {
                    let _ = sender
                        .try_send(Err(format!("failed to read capture actor command: {err}")));
                    break;
                }
            }
        }
    }
}

/// `CaptureWorker` state
#[derive(Debug)]
struct CustomData {
    /// DB-backed file reservations
    reservations: Arc<ReservationClient>,
    /// prefetched reservation for the first output file
    next_file: Option<FileReservation>,
    /// splitmuxsink and associated video sink pad, used by gstreamer
    /// callbacks
    mkv_sink_pad: (Element, Option<Pad>),
    /// set once video pad is connected
    video_appsink: Option<AppSink>,
    /// set if audio stream is provided
    audio_appsink: Option<AppSink>,
    /// blocks mux branches until `rtspsrc` has exposed all startup pads
    startup_preroll: StartupPreroll,
    /// muxer created for the fragment whose location is requested next
    pending_muxer: Option<Element>,
    /// files currently being written or finalized, keyed by splitmux fragment ID
    fragments: HashMap<u32, FragmentCapture>,
    /// identity and UTC mapping for the current `GStreamer` running-time domain
    capture_clock: Option<CaptureClock>,
    /// `last_frame_time` is set to detect a hung process not
    /// producing frames
    last_frame_time: Instant,
}

impl CustomData {
    pub fn new(reservations: Arc<ReservationClient>, next_file: FileReservation) -> Self {
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

        let mkv_sink = gst::ElementFactory::make("splitmuxsink")
            .name("sink")
            .property("async-finalize", true)
            .property("max-size-bytes", 15_000_000u64)
            .property_from_str("muxer-factory", "matroskamux")
            .property("muxer-properties", muxer_properties)
            .build()
            .expect("Could not create sink element.");

        Self {
            reservations,
            next_file: Some(next_file),
            mkv_sink_pad: (mkv_sink, None),
            video_appsink: None,
            audio_appsink: None,
            startup_preroll: StartupPreroll::default(),
            pending_muxer: None,
            fragments: HashMap::new(),
            capture_clock: None,
            last_frame_time: Instant::now(),
        }
    }
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
        let old_serialized = old_s
            .value_by_quark(glib::Quark::from_str(*field))
            .ok()
            .map(|v| v.serialize().ok());

        let new_serialized = new_s
            .value_by_quark(glib::Quark::from_str(*field))
            .ok()
            .map(|v| v.serialize().ok());

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
                Some(clock.time()?.saturating_sub(base_time))
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

    let ghost_sink = gst::GhostPad::builder_with_target(&depay_sink_pad)
        .expect("failed to get ghost sink builder")
        .name("sink")
        .build();

    bin.add_pad(&ghost_sink).expect("failed to add ghost sink");

    let mkv_src = mkv_capsfilter
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

    debug!(
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
        debug!("Invalid sample encoding \"{sample_type}\"");
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

fn handle_muxer_added(data_weak: &Weak<Mutex<CustomData>>, muxer: &Element) {
    let data = data_weak
        .upgrade()
        .expect("capture data dropped before muxer-added");
    data.lock().unwrap().pending_muxer = Some(muxer.clone());
}

fn handle_file_location_request(
    splitmuxsink: &Element,
    fragment_id: u32,
    first_sample: &gst::Sample,
    data_weak: &Weak<Mutex<CustomData>>,
) -> gstreamer::glib::Value {
    let reference_running_time = sample_running_time(first_sample)
        .expect("first fragment sample does not have a PTS in a TIME segment");
    let Some(data) = data_weak.upgrade() else {
        panic!("Failed to upgrade the weak reference");
    };
    let (reservations, next_file, capture_clock) = {
        let mut d = data.lock().unwrap();
        let mut capture_clock = d
            .capture_clock
            .take()
            .or_else(|| CaptureClock::anchored_to(splitmuxsink))
            .expect("splitmuxsink does not have a running-time clock");
        let (capture_session_id, clock_anchor) = capture_clock
            .metadata_for_reference(splitmuxsink, reference_running_time)
            .expect("splitmuxsink does not have a running-time clock");
        d.capture_clock = Some(capture_clock);
        let pending_muxer = d
            .pending_muxer
            .take()
            .expect("format-location-full arrived before muxer-added");
        write_capture_timeline_tag(
            pending_muxer,
            &CaptureTimelineMetadata {
                capture_session_id,
                clock_anchor,
            },
        );

        (
            Arc::clone(&d.reservations),
            d.next_file.take(),
            d.capture_clock
                .clone()
                .expect("capture clock was just stored"),
        )
    };

    let reservation = next_file.unwrap_or_else(|| {
        reservations.wait_for_reservation().unwrap_or_else(|err| {
            error!("Failed to get file reservation: {err}");
            panic!("failed to get file reservation: {err}");
        })
    });
    let filename = reservation.filename.clone();
    debug!("New reserved file: {filename}");

    {
        let mut d = data.lock().unwrap();
        d.fragments.insert(
            fragment_id,
            FragmentCapture {
                video_unit_id: reservation.video_unit_id,
                video_file_id: reservation.video_file_id,
                filename: filename.clone(),
                begin_running_time: reference_running_time,
                capture_clock,
            },
        );
    }

    ReservationClient::request_next();
    PathBuf::from(filename).into()
}

fn handle_fragment_opened(structure: &gst::StructureRef, data_weak: &Weak<Mutex<CustomData>>) {
    let fragment_id = structure
        .get::<u32>("fragment-id")
        .expect("fragment-opened message has no fragment ID");
    let data = data_weak
        .upgrade()
        .expect("capture data dropped before fragment-opened");
    let d = data.lock().unwrap();
    let fragment = d
        .fragments
        .get(&fragment_id)
        .expect("fragment-opened arrived before format-location-full");
    // File bookkeeping uses the reference stream's fragment boundary. Sample
    // playback uses the session timestamps stored directly in the MKV.
    let begin_time = fragment
        .capture_clock
        .utc_at_running_time(fragment.begin_running_time)
        .expect("fragment start is too far from its UTC anchor")
        .to_rfc3339_opts(SecondsFormat::Nanos, true);
    let message = CaptureMessage::FileOpened {
        video_unit_id: fragment.video_unit_id,
        video_file_id: fragment.video_file_id,
        filename: fragment.filename.clone(),
        begin_time,
    };
    drop(d);
    exserial::print_message(message);
}

fn handle_fragment_closed(structure: &gst::StructureRef, data_weak: &Weak<Mutex<CustomData>>) {
    let fragment_id = structure
        .get::<u32>("fragment-id")
        .expect("fragment-closed message has no fragment ID");
    let duration = structure
        .get::<gst::ClockTime>("fragment-duration")
        .expect("fragment-closed message has no duration");
    let data = data_weak
        .upgrade()
        .expect("capture data dropped before fragment-closed");
    let fragment = data
        .lock()
        .unwrap()
        .fragments
        .remove(&fragment_id)
        .expect("fragment-closed arrived for an unknown fragment");
    let end_running_time = fragment
        .begin_running_time
        .checked_add(duration)
        .expect("fragment running-time end overflowed");
    let end_time = fragment
        .capture_clock
        .utc_at_running_time(end_running_time)
        .expect("fragment end is too far from its UTC anchor")
        .to_rfc3339_opts(SecondsFormat::Nanos, true);

    exserial::print_message(CaptureMessage::EndFile {
        video_unit_id: fragment.video_unit_id,
        video_file_id: fragment.video_file_id,
        filename: fragment.filename,
        end_time,
    });
}

fn schedule_startup_preroll_timeout(data_weak: Weak<Mutex<CustomData>>) {
    thread::Builder::new()
        .name("startup-preroll-timeout".to_string())
        .spawn(move || {
            thread::sleep(STARTUP_PREROLL_TIMEOUT);

            let Some(data) = data_weak.upgrade() else {
                return;
            };

            data.lock().unwrap().startup_preroll.finish("timeout");
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

    info!("New pad type {new_pad_type}, {media} {encoding_name}");

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
        let preroll_block = d.startup_preroll.block_pad(&bin_mkv_src_pad, "video");
        if preroll_block.start_timeout {
            schedule_startup_preroll_timeout(data_weak.clone());
        }

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
            .link(
                d.mkv_sink_pad
                    .1
                    .as_ref()
                    .expect("splitmuxsink video pad was not requested"),
            )
            .expect("linking bin to video sink failed");

        info!("Link succeeded (type {new_pad_type}).");

        bin.sync_state_with_parent()
            .expect("failed to sync bin state");
    } else if let ("audio", Some(depay_name), &None) = (media, depayloader_name, &d.audio_appsink) {
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
        let preroll_block = d.startup_preroll.block_pad(&bin_mkv_src_pad, "audio");
        if preroll_block.start_timeout {
            schedule_startup_preroll_timeout(data_weak);
        }

        src_pad
            .link(&bin_sink_pad)
            .expect("failed to link new_src_pad to bin_sink_pad");

        // the bin pipeline to appsink
        let appsink = AppSink::builder().name("audio_appsink").sync(false).build();

        pipeline
            .add_many([appsink.upcast_ref::<gst::Element>()])
            .expect("failed to add audio appsink to pipeline");

        appsink.set_callbacks(
            AppSinkCallbacks::builder()
                .new_sample(handle_audio_sample)
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
        d.audio_appsink = Some(appsink);

        let request_audio_pad = preroll_block.blocked;
        let mkv_sink = d.mkv_sink_pad.0.clone();
        drop(d);

        // Requesting a pad synchronously emits muxer-added, whose handler also
        // accesses CustomData, so the CustomData lock must not be held here.
        let audio_sink_pad = request_audio_pad.then(|| {
            mkv_sink
                .request_pad_simple("audio_%u")
                .expect("Failed to get audio sink pad from splitmuxsink")
        });

        if let Some(audio_sink_pad) = audio_sink_pad {
            // link the bin pipeline to the mkv splitmuxsink
            bin_mkv_src_pad
                .link(&audio_sink_pad)
                .expect("linking bin to audio sink failed");
        } else {
            info!("Audio arrived after startup preroll; draining it outside of the MKV muxer");
            drain_unmuxed_branch(&pipeline, &bin_mkv_src_pad, "late_audio_fakesink");
        }

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
    let _storage_path: PathBuf = std::env::args()
        .nth(2)
        .expect("Failed to get storage path")
        .into();
    let reservations = ReservationClient::start(TIMEOUT);
    ReservationClient::request_next();
    let initial_reservation = reservations
        .wait_for_reservation()
        .expect("failed to reserve initial file");
    let data = CustomData::new(reservations, initial_reservation);

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

    let mkv_sink = data.lock().unwrap().mkv_sink_pad.0.clone();
    mkv_sink.connect("muxer-added", false, {
        let data_weak = data_weak.clone();

        move |values| {
            let muxer = values[1]
                .get::<Element>()
                .expect("muxer-added did not provide a GstElement");
            handle_muxer_added(&data_weak, &muxer);
            None
        }
    });

    mkv_sink.connect("format-location-full", false, {
        let data_weak = data_weak.clone();

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
                &data_weak,
            ))
        }
    });

    // Request the primary video pad only after muxer-added is connected. The
    // request creates the initial muxer and emits muxer-added synchronously.
    let video_sink_pad = mkv_sink
        .request_pad_simple("video")
        .expect("Failed to get video sink pad from splitmuxsink");
    data.lock().unwrap().mkv_sink_pad.1 = Some(video_sink_pad);

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

                MessageView::Element(element) => {
                    let Some(structure) = element.structure() else {
                        continue;
                    };
                    match structure.name().as_str() {
                        "splitmuxsink-fragment-opened" => {
                            handle_fragment_opened(structure, &data_weak);
                        }
                        "splitmuxsink-fragment-closed" => {
                            handle_fragment_closed(structure, &data_weak);
                        }
                        _ => {}
                    }
                }

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
