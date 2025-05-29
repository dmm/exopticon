use std::{
    fs::create_dir_all,
    sync::{Arc, Mutex},
};

use exserial::{exlog::ExLog, models::CaptureMessage};
use gstreamer::{
    self as gst,
    glib::object::{Cast, ObjectExt},
    prelude::{ElementExt, ElementExtManual, GstBinExtManual, GstObjectExt, PadExt},
    Bin,
};
use gstreamer_app::{AppSink, AppSinkCallbacks};
use log::{debug, error, info};
use uuid::Uuid;

static LOGGER: ExLog = ExLog;

#[derive(Debug)]
struct CustomData {
    video_appsink: Option<AppSink>,
    audio_appsink: Option<AppSink>,
}

impl CustomData {
    pub fn new() -> Self {
        Self {
            video_appsink: None,
            audio_appsink: None,
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

    parent_path.join(&path).into()
}

fn create_video_branch() -> Bin {
    let bin = gst::Bin::with_name("video_sink_bin");

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
    let tee_sink = tee.static_pad("sink").expect("failed to get tee sink");

    let ghost_sink = gst::GhostPad::builder_with_target(&tee_sink)
        .expect("failed to get ghost sink builder")
        .name("sink")
        .build();

    bin.add_pad(&ghost_sink).expect("failed to add ghost sink");

    let mkv_src = mkv_queue
        .static_pad("src")
        .expect("failed to get mkv queue src pad");

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

    let source = gst::ElementFactory::make("rtspsrc")
        .name("source")
        .property_from_str("location", &url)
        .build()
        .expect("Could not create source element.");

    let mkv_sink = gst::ElementFactory::make("splitmuxsink")
        .name("sink")
        .property("async-finalize", true)
        .property("max-size-bytes", 15000000u64)
        .property_from_str("muxer-factory", "matroskamux")
        .build()
        .expect("Could not create sink element.");

    let pipeline = gst::Pipeline::with_name("capture-pipeline");

    pipeline
        .add_many([&source, &mkv_sink])
        .expect("Failed to add elements to pipeline.");

    let video_sink_pad = mkv_sink
        .request_pad_simple("video")
        .expect("Failed to get video sink pad from convert");

    mkv_sink.connect("format-location-full", false, move |_el| {
        let id = Uuid::now_v7();
        let path = uuid_to_filename(&storage_path, id);
        let parent = path.parent().expect("failed to get new file parent");
        create_dir_all(parent).expect("failed to create parent directory");
        info!("New file: {}", path.display());
        Some(path.into())
    });

    let data: Arc<Mutex<CustomData>> = Arc::new(Mutex::new(CustomData::new()));
    let data_weak = Arc::downgrade(&data);
    let data_weak2 = Arc::downgrade(&data);

    // Connect the pad-added signal
    source.connect_pad_added(move |src, src_pad| {
        info!("Received new pad {} from {}", src_pad.name(), src.name());

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

        if media == "video" && video_sink_pad.is_linked() {
            info!("Video is already linked. Ignoring.");
            return;
        }

        let (depayloader_name, parser_name) = match (&media, &encoding_name) {
            (&"video", &"H264") => (Some("rtph264depay"), Some("h264parse")),
            (&"video", &"H265") => (Some("rtph265depay"), Some("h265parse")),
            (&"audio", &"OPUS") => (Some("rtpopusdepay"), Some("opusparse")),
            (&"audio", &"MPEG4-GENERIC") | (&"audio", &"AAC") => {
                (Some("rtpmp4gdepay"), Some("aacparse"))
            }
            (&"audio", &"PCMU") => (Some("rtppcmudepay"), None),
            (&"audio", &"PCMA") => (Some("rtppcmadepay"), None),
            _ => (None, None),
        };

        info!(
            "New pad type {}, {} {}",
            &new_pad_type, &media, &encoding_name
        );

        let Some(data) = data_weak.upgrade() else {
            return;
        };
        let mut d = data.lock().unwrap();

        if let Some(depay_name) = depayloader_name {
            let depay = gst::ElementFactory::make(&depay_name)
                .name(depay_name)
                .build()
                .expect("Failed to build depay element.");
            let pipeline = src
                .parent()
                .expect("Failed to get src parent")
                .downcast::<gst::Pipeline>()
                .unwrap();
            pipeline.add_many(&[&depay]).expect("Failed to add depay");
            depay
                .sync_state_with_parent()
                .expect("Failed to sync depay state with parent");
            let depay_sink_pad = depay
                .static_pad("sink")
                .expect("Failed to get depay sink pad.");
            src_pad
                .link(&depay_sink_pad)
                .expect("Failed to link new src and depay");

            let new_src_pad = if let Some(parser_name) = parser_name {
                let parser = gst::ElementFactory::make(&parser_name)
                    .name(parser_name)
                    .build()
                    .expect("Failed to build parser");
                pipeline
                    .add_many(&[&parser])
                    .expect("Failed to add depay & parser to pipeline");
                parser
                    .sync_state_with_parent()
                    .expect("parser failed sync state with parent");
                depay.link(&parser).expect("Failed to link depay to parser");
                parser
                    .static_pad("src")
                    .expect("Failed to get parser src pad.")
            } else {
                // we don't need a parser, for pcm-a or pcm-u
                depay.static_pad("src").expect("Failed to get depay src")
            };

            if media.starts_with("video") {
                let bin = create_video_branch();
                pipeline.add_many(&[&bin]).expect("Failed to add bin");

                let bin_sink_pad = bin.static_pad("sink").expect("failed to get bin sink pad");

                let bin_mkv_src_pad = bin
                    .static_pad("mkv_src")
                    .expect("failed to get bin src pad");

                new_src_pad
                    .link(&bin_sink_pad)
                    .expect("failed to link new_src_pad to bin_sink_pad");

                // the bin pipeline to appsink
                let appsink = AppSink::builder()
                    //                    .caps(&new_pad_caps)
                    .name("video_appsink")
                    .build();

                pipeline
                    .add_many(&[appsink.upcast_ref::<gst::Element>()])
                    .expect("failed to add video appsink to pipeline");

                let value = data_weak2.clone();
                appsink.set_callbacks(
                    AppSinkCallbacks::builder()
                        .new_sample(move |_| {
                            let Some(data) = value.upgrade() else {
                                return Ok(gst::FlowSuccess::Ok);
                            };

                            let appsink = {
                                let data = data.lock().unwrap();
                                data.video_appsink.clone()
                            };

                            if let Some(appsink) = appsink {
                                if let Ok(sample) = appsink.pull_sample() {
                                    let buffer =
                                        sample.buffer().expect("failed to get sample buffer");
                                    let caps = sample.segment().expect("failed to get sample caps");
                                    //                                    let nal = buffer.map().unwrap().as_slice();
                                    let nal =
                                        buffer.map_readable().expect("failed to get buffer map");
                                    let pts_90khz = buffer
                                        .pts()
                                        .map(|pts| pts.nseconds() * 90_000 / 1_000_000_000)
                                        .expect("failed to get buffer pts");
                                    let msg = CaptureMessage::Packet {
                                        data: nal.to_vec(),
                                        timestamp: i64::try_from(pts_90khz).expect("i64 overflow"),
                                        duration: 0,
                                    };
                                    exserial::print_message(msg);
                                    debug!(
                                        "SAMPLE PTS: {} 90Hz pts: {:?} offset: {}",
                                        buffer.pts().expect("failed to get pts"),
                                        pts_90khz,
                                        buffer.offset()
                                    );
                                }
                            } else {
                                error!("APPSINK IS NONE");
                            }

                            Ok(gst::FlowSuccess::Ok)
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
                let res = bin_mkv_src_pad.link(&video_sink_pad);
                if res.is_err() {
                    info!("Type is {new_pad_type} but link failed.");
                } else {
                    info!("Link succeeded (type {new_pad_type}).");
                }

                bin.sync_state_with_parent()
                    .expect("failed to sync bin state");
            } else {

                // let audio_tee_sink = audio_tee
                //     .request_pad_simple("sink")
                //     .expect("Failed to get audio tee sink pad");
                // let res = new_src_pad.link(&audio_tee_sink);
                // audio_tee
                //     .link_pads(Some("src_%u"), &mkv_sink, Some("audio_%u"))
                //     .expect("Failed to link audio_tee pads");
                // if res.is_err() {
                //     println!("Type is {new_pad_type} but link failed.");
                // } else {
                //     println!("Link succeeded (type {new_pad_type}).");
                // }
            }
        } else {
            error!("Unknown RTP encoding: {} / {}", media, encoding_name);
        }
    });

    // Start playing
    pipeline
        .set_state(gstreamer::State::Playing)
        .expect("Unable to set the pipeline to the `Playing` state");

    // Wait until error or EOS
    let bus = pipeline.bus().unwrap();
    for msg in bus.iter_timed(gstreamer::ClockTime::NONE) {
        use gstreamer::MessageView;
        info!("LOOP!!!! {:?}", msg.type_());
        match msg.view() {
            MessageView::Error(err) => {
                error!(
                    "Error received from element {:?} {}",
                    err.src().map(|s| s.path_string()),
                    err.error()
                );
                error!("Debugging information: {:?}", err.debug());
                break;
            }
            MessageView::StateChanged(state_changed) => {
                if state_changed.src().map(|s| s == &pipeline).unwrap_or(false) {
                    info!(
                        "Pipeline state changed from {:?} to {:?}",
                        state_changed.old(),
                        state_changed.current()
                    );
                }
            }
            MessageView::Eos(..) => break,

            MessageView::Progress(_progress) => {
                info!("PROGRESS");
            }
            _ => {}
        }
    }

    pipeline
        .set_state(gstreamer::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");
}
