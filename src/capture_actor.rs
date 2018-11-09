use std::process::{Command, Stdio};
use std::time::Duration;

use actix::*;
use actix_web::actix::fut::wrap_future;
use bytes::BytesMut;
use rmp_serde::Deserializer;
use serde::Deserialize;
use tokio_io::codec::length_delimited;
use tokio_process::CommandExt;

use ws_camera_server::{CameraFrame, FrameResolution, WsCameraServer};

#[derive(Default, Debug, PartialEq, Deserialize, Serialize)]
struct CaptureMessage {
    #[serde(rename = "type")]
    #[serde(default)]
    pub message_type: String,

    #[serde(default)]
    pub level: String,

    #[serde(default)]
    pub message: String,

    #[serde(rename = "jpegFrame")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    pub jpeg: Vec<u8>,

    #[serde(rename = "jpegFrameScaled")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    pub scaled_jpeg: Vec<u8>,

    #[serde(default)]
    pub offset: i64,

    #[serde(default)]
    pub height: i32,

    #[serde(default)]
    pub filename: String,

    #[serde(rename = "beginTime")]
    #[serde(default)]
    pub begin_time: String,

    #[serde(rename = "endTime")]
    #[serde(default)]
    pub end_time: String,
}

#[derive(Debug)]
pub struct CaptureActor {
    pub camera_id: i32,
    pub stream_url: String,
    pub storage_path: String,
}

impl CaptureActor {
    fn message_to_action(&self, msg: CaptureMessage) {
        // Check if log
        if msg.message_type == "log" {
            debug!("Worker log message: {}", msg.message);
        }
        // Check if full frame
        else if msg.jpeg.len() > 0 {
            WsCameraServer::from_registry().do_send(CameraFrame {
                camera_id: self.camera_id,
                jpeg: msg.jpeg,
                resolution: FrameResolution::HD,
            });
        }
        // Check if scaled frame
        else if msg.scaled_jpeg.len() > 0 {
            WsCameraServer::from_registry().do_send(CameraFrame {
                camera_id: self.camera_id,
                jpeg: msg.scaled_jpeg,
                resolution: FrameResolution::SD,
            });
        }
        // Check if new file
        else if msg.begin_time != "" {
            // worker has created a new file. Write video_unit and
            // file to database.

        }
        // Check if end file
        else if msg.end_time != "" {

        }
    }
}

impl StreamHandler<BytesMut, std::io::Error> for CaptureActor {
    fn handle(&mut self, item: BytesMut, _ctx: &mut Context<CaptureActor>) {
        let mut de = Deserializer::new(&item[..]);

        let frame: Result<CaptureMessage, rmp_serde::decode::Error> =
            Deserialize::deserialize(&mut de);

        match frame {
            Ok(f) => self.message_to_action(f),
            Err(e) => error!("Error deserializing frame! {}", e),
        }
    }

    fn finished(&mut self, _ctx: &mut Self::Context) {
        println!("finished");
    }
}

struct StartWorker;

impl Message for StartWorker {
    type Result = ();
}

impl Handler<StartWorker> for CaptureActor {
    type Result = ();

    fn handle(&mut self, _msg: StartWorker, ctx: &mut Context<Self>) -> Self::Result {
        info!("Launching worker for stream: {}", self.stream_url);
        let mut cmd = Command::new("src/cworkers/captureworker");
        cmd.arg(self.stream_url.clone());
        cmd.arg("0");
        cmd.arg(self.storage_path.clone());
        cmd.arg("/dev/null");
        cmd.stdout(Stdio::piped());

        let mut child = cmd.spawn_async().expect("Failed to launch");
        let stdout = child.stdout().take().unwrap();
        let framed_stream = length_delimited::FramedRead::new(stdout);
        Self::add_stream(framed_stream, ctx);
        let fut = wrap_future::<_, Self>(child)
            .map(|_status, _actor, ctx| {
                ctx.notify_later(StartWorker {}, Duration::new(5, 0));
            }).map_err(|_e, _actor, _ctx| {}); // Do something on error?
        ctx.spawn(fut);
    }
}

impl Actor for CaptureActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.address().do_send(StartWorker {});
    }
}

impl CaptureActor {
    pub fn new(camera_id: i32, stream_url: String, storage_path: String) -> CaptureActor {
        CaptureActor {
            camera_id,
            stream_url,
            storage_path,
        }
    }
}
