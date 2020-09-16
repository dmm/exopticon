// This is mostly taken from
// https://raw.githubusercontent.com/sdroege/gstreamer-rs/master/examples/src/bin/thumbnail.rs
// so the license is MIT or Apache 2.0 until it's sufficiently rewritten.

/// error handling
mod error;

use std::io::Write;

extern crate gstreamer as gst;
use gst::gst_element_error;
use gst::prelude::*;
use gst::ClockTime;
extern crate gstreamer_app as gst_app;
extern crate gstreamer_video as gst_video;

extern crate image;
#[macro_use]
extern crate log;
extern crate env_logger;

use anyhow::Error;
use byte_slice_cast::*;
use derive_more::{Display, Error};

use crate::error::SnapshotError;

#[derive(Debug, Display, Error)]
#[display(fmt = "Missing element {}", _0)]
struct MissingElement(#[error(not(source))] &'static str);

#[derive(Debug, Display, Error)]
#[display(fmt = "Received error from {}: {} (debug: {:?})", src, error, debug)]
struct ErrorMessage {
    src: String,
    error: String,
    debug: Option<String>,
    source: glib::Error,
}

fn create_pipeline(uri: String) -> Result<gst::Pipeline, Error> {
    gst::init()?;

    // Create our pipeline from a pipeline description string.
    let pipeline = gst::parse_launch(&format!(
        "uridecodebin uri={} ! videoconvert ! jpegenc ! appsink name=sink",
        uri
    ))?
    .downcast::<gst::Pipeline>()
    .expect("Expected a gst::Pipeline");

    // Get access to the appsink element.
    let appsink = pipeline
        .get_by_name("sink")
        .expect("Sink element not found")
        .downcast::<gst_app::AppSink>()
        .expect("Sink element is expected to be an appsink!");

    // Don't synchronize on the clock, we only want a snapshot asap.
    appsink.set_property("sync", &false).unwrap();

    let mut got_snapshot = false;

    // Getting data out of the appsink is done by setting callbacks on it.
    // The appsink will then call those handlers, as soon as data is available.
    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            // Add a handler to the "new-sample" signal.
            .new_sample(move |appsink| {
                debug!("Starting appsink new_sample callback");
                // Pull the sample in question out of the appsink's buffer.
                let sample = appsink.pull_sample().map_err(|_| {
                    error!("Failed to get the sample!");
                    gst::FlowError::Eos
                })?;
                let buffer = sample.get_buffer().ok_or_else(|| {
                    gst_element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to get buffer from appsink")
                    );

                    gst::FlowError::Error
                })?;

                //                let caps = sample.get_caps().expect("Sample without caps");
                //                let info = gst_video::VideoInfo::from_caps(&caps).expect("Failed to parse caps");

                // Make sure that we only get a single buffer
                if got_snapshot {
                    return Err(gst::FlowError::Eos);
                }
                got_snapshot = true;

                // At this point, buffer is only a reference to an existing memory region somewhere.
                // When we want to access its content, we have to map it while requesting the required
                // mode of access (read, read/write).
                // This type of abstraction is necessary, because the buffer in question might not be
                // on the machine's main memory itself, but rather in the GPU's memory.
                // So mapping the buffer makes the underlying memory region accessible to us.
                // See: https://gstreamer.freedesktop.org/documentation/plugin-development/advanced/allocation.html
                let map = buffer.map_readable().map_err(|_| {
                    gst_element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to map buffer readable")
                    );

                    gst::FlowError::Error
                })?;
                let samples = map.as_slice_of::<u8>().map_err(|_| {
                    gst_element_error!(
                        appsink,
                        gst::ResourceError::Failed,
                        ("Failed to interprete buffer as S16 PCM")
                    );

                    gst::FlowError::Error
                })?;

                // We only want to have a single buffer and then have the pipeline terminate
                info!("Have video frame");

                let stdout = std::io::stdout();
                let mut handle = stdout.lock();
                handle.write_all(samples).expect("stdout write failed!");

                Err(gst::FlowError::Eos)
            })
            .build(),
    );

    Ok(pipeline)
}

fn main_loop(pipeline: gst::Pipeline, position: u64) -> Result<(), Error> {
    pipeline.set_state(gst::State::Paused)?;

    let bus = pipeline
        .get_bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    let mut seeked = false;

    for msg in bus.iter_timed(gst::CLOCK_TIME_NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::AsyncDone(msg) => {
                debug!("Async done, running time: {}", msg.get_running_time());
                if !seeked {
                    // AsyncDone means that the pipeline has started now and that we can seek
                    info!(
                        "Got AsyncDone message, seeking to {} microseconds",
                        position
                    );

                    if pipeline
                        .seek_simple(
                            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                            ClockTime::from_useconds(position),
                        )
                        .is_err()
                    {
                        info!("Failed to seek, taking first frame");
                    }

                    pipeline.set_state(gst::State::Playing)?;
                    seeked = true;
                } else {
                    info!("Got second AsyncDone message, seek finished");
                }
            }
            MessageView::Eos(..) => {
                // The End-of-stream message is posted when the stream is done, which in our case
                // happens immediately after creating the thumbnail because we return
                // gst::FlowError::Eos then.
                info!("Got Eos message, done");
                break;
            }
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null)?;
                return Err(ErrorMessage {
                    src: msg
                        .get_src()
                        .map(|s| String::from(s.get_path_string()))
                        .unwrap_or_else(|| String::from("None")),
                    error: err.get_error().to_string(),
                    debug: err.get_debug(),
                    source: err.get_error(),
                }
                .into());
            }
            _ => (),
        }
    }

    pipeline.set_state(gst::State::Null)?;

    Ok(())
}

pub fn get_snapshot(uri: String, microsecond_offset: u64) -> Result<Vec<u8>, SnapshotError> {
    match create_pipeline(uri).and_then(|pipeline| main_loop(pipeline, microsecond_offset)) {
        Ok(_r) => debug!("IT WORKED!"),

        Err(e) => error!("Error! {}", e),
    }

    Err(SnapshotError::Invalid)
}

fn main() {
    env_logger::init();
    use std::env;

    let mut args = env::args();
    // Parse commandline arguments: input URI, position in seconds, output path
    let _arg0 = args.next().unwrap();
    let uri = args
        .next()
        .expect("No input URI provided on the commandline");
    let position = args
        .next()
        .expect("No position in second on the commandline");
    let position = position
        .parse::<u64>()
        .expect("Failed to parse position as integer");

    match create_pipeline(uri).and_then(|pipeline| main_loop(pipeline, position)) {
        Ok(r) => r,
        Err(e) => error!("Error! {}", e),
    }
}
