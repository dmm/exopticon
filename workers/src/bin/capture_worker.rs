use gstreamer::{
    self as gst,
    glib::{
        object::{Cast, ObjectExt},
        value::ToValue,
    },
    prelude::{ElementExt, ElementExtManual, GstBinExtManual, GstObjectExt, PadExt},
};
use uuid::Uuid;

fn main() {
    // Initialize GStreamer
    gstreamer::init().expect("gstreamer::init failed");

    if std::env::args().len() < 3 {
        println!("USAGE: capture_worker <rtsp_url> <storage_path>");
        return;
    }

    let url = std::env::args().nth(1).expect("Failed to get url");
    let storage_path = std::env::args().nth(2).expect("Failed to get storage path");

    let source = gst::ElementFactory::make("rtspsrc")
        .name("source")
        .property_from_str("location", &url)
        .build()
        .expect("Could not create source element.");

    let sink = gst::ElementFactory::make("splitmuxsink")
        .name("sink")
        .property("async-finalize", true)
        .property("max-size-bytes", 15000000u64)
        .property_from_str("muxer-factory", "matroskamux")
        .build()
        .expect("Could not create sink element.");

    let pipeline = gst::Pipeline::with_name("capture-pipeline");

    pipeline
        .add_many([&source, &sink])
        .expect("Failed to add elements to pipeline.");

    let video_sink_pad = sink
        .request_pad_simple("video")
        .expect("Failed to get video sink pad from convert");

    sink.connect("format-location-full", false, move |_el| {
        let id = Uuid::now_v7();
        Some(format!("{}/{}.mkv", storage_path, id).to_value())
    });

    // Connect the pad-added signal
    source.connect_pad_added(move |src, src_pad| {
        println!("Received new pad {} from {}", src_pad.name(), src.name());

        src.downcast_ref::<gstreamer::Bin>()
            .unwrap()
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
            println!("Video is already linked. Ignoring.");
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

        println!(
            "New pad type {}, {} {}",
            &new_pad_type, &media, &encoding_name
        );

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
                let res = new_src_pad.link(&video_sink_pad);
                if res.is_err() {
                    println!("Type is {new_pad_type} but link failed.");
                } else {
                    println!("Link succeeded (type {new_pad_type}).");
                }
            } else {
                let audio_sink = sink
                    .request_pad_simple("audio_%u")
                    .expect("Failed to get audio sink pad");
                let res = new_src_pad.link(&audio_sink);
                if res.is_err() {
                    println!("Type is {new_pad_type} but link failed.");
                } else {
                    println!("Link succeeded (type {new_pad_type}).");
                }
            }
        } else {
            eprintln!("Unknown RTP encoding: {} / {}", media, encoding_name);
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

        match msg.view() {
            MessageView::Error(err) => {
                eprintln!(
                    "Error received from element {:?} {}",
                    err.src().map(|s| s.path_string()),
                    err.error()
                );
                eprintln!("Debugging information: {:?}", err.debug());
                break;
            }
            MessageView::StateChanged(state_changed) => {
                if state_changed.src().map(|s| s == &pipeline).unwrap_or(false) {
                    println!(
                        "Pipeline state changed from {:?} to {:?}",
                        state_changed.old(),
                        state_changed.current()
                    );
                }
            }
            MessageView::Eos(..) => break,
            _ => (),
        }
    }

    pipeline
        .set_state(gstreamer::State::Null)
        .expect("Unable to set the pipeline to the `Null` state");
}
