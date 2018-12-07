#[allow(deprecated)]
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::actix::prelude::*;
use actix_web::actix::fut::wrap_future;
use bytes::BytesMut;
use chrono::{DateTime, Utc};
use rmp_serde::Deserializer;
use serde::Deserialize;
use tokio_io::codec::length_delimited;
use tokio_process::CommandExt;

use crate::models::{CreateVideoUnitFile, DbExecutor, UpdateVideoUnitFile};
use crate::ws_camera_server::{CameraFrame, FrameResolution, WsCameraServer};

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

pub struct CaptureActor {
    pub camera_id: i32,
    pub stream_url: String,
    pub storage_path: String,
    pub db_addr: Addr<DbExecutor>,
    pub video_unit_id: Option<i32>,
    pub video_file_id: Option<i32>,
    pub filename: Option<String>,
}

impl CaptureActor {
    fn close_file(
        &self,
        ctx: &mut Context<CaptureActor>,
        filename: String,
        end_time: DateTime<Utc>,
    ) {
        if let (Some(video_unit_id), Some(video_file_id), Ok(metadata)) = (
            self.video_unit_id,
            self.video_file_id,
            fs::metadata(filename),
        ) {
            let fut = self.db_addr.send(UpdateVideoUnitFile {
                video_unit_id: video_unit_id,
                end_time: end_time.naive_utc(),
                video_file_id: video_file_id,
                size: metadata.len() as i32,
            });
            ctx.spawn(
                wrap_future::<_, Self>(fut)
                    .map(|result, _actor, _ctx| match result {
                        Ok((_video_unit, _video_file)) => {}
                        Err(e) => error!("CaptureWorker: Error updating video unit: {}", e),
                    })
                    .map_err(|_e, _actor, _ctx| {
                        error!("CaptureWorker: Error calling UpdateVideoUnitFile");
                    }),
            );
        } else {
            error!("Error closing file!");
        }
    }

    fn message_to_action(&self, msg: CaptureMessage, ctx: &mut Context<CaptureActor>) {
        // Check if log
        match msg.message_type.as_str() {
            "log" => debug!("Worker log message: {}", msg.message),
            "frame" => {
                WsCameraServer::from_registry().do_send(CameraFrame {
                    camera_id: self.camera_id,
                    jpeg: msg.jpeg,
                    resolution: FrameResolution::HD,
                });
            }
            "frameScaled" => {
                WsCameraServer::from_registry().do_send(CameraFrame {
                    camera_id: self.camera_id,
                    jpeg: msg.scaled_jpeg,
                    resolution: FrameResolution::SD,
                });
            }
            "newFile" => {
                // worker has created a new file. Write video_unit and
                // file to database.
                if let Ok(date) = msg.begin_time.parse::<DateTime<Utc>>() {
                    let filename = msg.filename.clone();
                    let fut = self.db_addr.send(CreateVideoUnitFile {
                        camera_id: self.camera_id,
                        monotonic_index: 0,
                        begin_time: date.naive_utc(),
                        filename: msg.filename,
                    });

                    ctx.spawn(
                        wrap_future::<_, Self>(fut)
                            .map(|result, actor, _ctx| match result {
                                Ok((video_unit, video_file)) => {
                                    actor.video_unit_id = Some(video_unit.id);
                                    actor.video_file_id = Some(video_file.id);
                                    actor.filename = Some(filename)
                                }
                                Err(e) => error!("Error! {}", e),
                            })
                            .map_err(|e, _actor, _ctx| {
                                error!("Captureworker: Error sending new file message: {}", e);
                            }),
                    );
                } else {
                    error!(
                        "CaptureWorker: unable to parse begin time: {}",
                        msg.begin_time
                    );
                }
            }
            "endFile" => {
                if let Ok(end_time) = msg.end_time.parse::<DateTime<Utc>>() {
                    self.close_file(ctx, msg.filename, end_time)
                } else {
                    error!("CaptureActor: Error handling close file message.");
                }
            }
            _ => error!(
                "CaptureActor {}: Invalid capture message type: {}",
                self.camera_id, msg.message_type
            ),
        }
    }
}

impl StreamHandler<BytesMut, std::io::Error> for CaptureActor {
    fn handle(&mut self, item: BytesMut, ctx: &mut Context<CaptureActor>) {
        let mut de = Deserializer::new(&item[..]);

        let frame: Result<CaptureMessage, rmp_serde::decode::Error> =
            Deserialize::deserialize(&mut de);

        match frame {
            Ok(f) => self.message_to_action(f, ctx),
            Err(e) => error!("Error deserializing frame! {}", e),
        }
    }

    fn finished(&mut self, _ctx: &mut Self::Context) {}
}

struct StartWorker;

impl Message for StartWorker {
    type Result = ();
}

impl Handler<StartWorker> for CaptureActor {
    type Result = ();

    fn handle(&mut self, _msg: StartWorker, ctx: &mut Context<Self>) -> Self::Result {
        info!("Launching worker for stream: {}", self.stream_url);
        let storage_path = Path::new(&self.storage_path).join(self.camera_id.to_string());
        std::fs::create_dir(&storage_path);
        let mut cmd = Command::new("src/cworkers/captureworker");
        cmd.arg(&self.stream_url);
        cmd.arg("0");
        cmd.arg(&storage_path);
        cmd.arg("/dev/null");
        cmd.stdout(Stdio::piped());

        let mut child = cmd.spawn_async().expect("Failed to launch");
        let stdout = child.stdout().take().unwrap();
        let framed_stream = length_delimited::FramedRead::new(stdout);
        Self::add_stream(framed_stream, ctx);
        let fut = wrap_future::<_, Self>(child)
            .map(|_status, actor, ctx| {
                error!("CaptureWorker {}: capture process died...", actor.camera_id);
                ctx.notify_later(StartWorker {}, Duration::new(5, 0));
            })
            .map_err(|_e, _actor, _ctx| {}); // Do something on error?
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
    pub fn new(
        db_addr: Addr<DbExecutor>,
        camera_id: i32,
        stream_url: String,
        storage_path: String,
    ) -> CaptureActor {
        CaptureActor {
            camera_id,
            stream_url,
            storage_path,
            db_addr,
            video_unit_id: None,
            video_file_id: None,
            filename: None,
        }
    }
}
