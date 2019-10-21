use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use actix::{
    Actor, ActorFuture, Addr, AsyncContext, Context, Handler, Message, StreamHandler, SystemService,
};
use actix_web::actix::fut::wrap_future;
use bytes::BytesMut;
use chrono::{DateTime, Utc};
use rmp_serde::Deserializer;
use serde::Deserialize;

use tokio::codec::length_delimited;
use tokio_process::CommandExt;

use crate::models::{CreateVideoUnitFile, DbExecutor, UpdateVideoUnitFile};
use crate::ws_camera_server::{CameraFrame, FrameResolution, FrameSource, WsCameraServer};

/// Holds messages from capture worker
#[derive(Default, Debug, PartialEq, Deserialize, Serialize)]
pub struct CaptureMessage {
    /// type of worker message
    #[serde(rename = "type")]
    #[serde(default)]
    pub message_type: String,

    /// if message is a log, the log level
    #[serde(default)]
    pub level: String,

    /// if the message is a log, the log message
    #[serde(default)]
    pub message: String,

    /// if the message is a frame, the jpeg frame
    #[serde(rename = "jpegFrame")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    pub jpeg: Vec<u8>,

    /// if the message is a frame, the sd scaled jpeg frame
    #[serde(rename = "jpegFrameScaled")]
    #[serde(default)]
    #[serde(with = "serde_bytes")]
    pub scaled_jpeg: Vec<u8>,

    /// if the message is a frame the original width of the image
    #[serde(rename = "unscaledWidth")]
    #[serde(default)]
    pub unscaled_width: i32,

    /// if the message is a frame the original height of the image
    #[serde(rename = "unscaledHeight")]
    #[serde(default)]
    pub unscaled_height: i32,

    /// if message is a frame, the offset from the beginning of file
    #[serde(default)]
    pub offset: i64,

    ///
    #[serde(default)]
    pub height: i32,

    /// if message is a new file, the created file name
    #[serde(default)]
    pub filename: String,

    /// if message is a new file, the file creation time
    #[serde(rename = "beginTime")]
    #[serde(default)]
    pub begin_time: String,

    /// if the message is a closed file, the file end time
    #[serde(rename = "endTime")]
    #[serde(default)]
    pub end_time: String,
}

/// Holds state of capture actor
pub struct CaptureActor {
    /// id of camera actor is capturing video for
    pub camera_id: i32,
    /// url of video stream
    pub stream_url: String,
    /// absolute path to video storage
    pub storage_path: String,
    /// address of database worker
    pub db_addr: Addr<DbExecutor>,
    /// id of currently open video unit
    pub video_unit_id: Option<i32>,
    /// id of currently open video file
    pub video_file_id: Option<i32>,
    /// frame offset from beginning of the current video unit
    pub offset: i64,
    /// filename currently being captured
    pub filename: Option<String>,
}

impl CaptureActor {
    /// Returns new initialized CaptureActor
    pub const fn new(
        db_addr: Addr<DbExecutor>,
        camera_id: i32,
        stream_url: String,
        storage_path: String,
    ) -> Self {
        Self {
            camera_id,
            stream_url,
            storage_path,
            db_addr,
            video_unit_id: None,
            video_file_id: None,
            offset: 0,
            filename: None,
        }
    }

    /// Called when the underlying capture worker signals the file as
    /// closed. The database record is updated
    #[allow(clippy::cast_possible_truncation)]
    fn close_file(&self, ctx: &mut Context<Self>, filename: &str, end_time: DateTime<Utc>) {
        if let (Some(video_unit_id), Some(video_file_id), Ok(metadata)) = (
            self.video_unit_id,
            self.video_file_id,
            fs::metadata(filename),
        ) {
            let fut = self.db_addr.send(UpdateVideoUnitFile {
                video_unit_id,
                end_time: end_time.naive_utc(),
                video_file_id,
                size: metadata.len() as i32,
            });
            ctx.spawn(
                wrap_future::<_, Self>(fut)
                    .map(|result, _actor, _ctx| match result {
                        Ok((_video_unit, _video_file)) => {}
                        Err(e) => panic!("CaptureWorker: Error updating video unit: {}", e),
                    })
                    .map_err(|_e, _actor, _ctx| {
                        error!("CaptureWorker: Error calling UpdateVideoUnitFile");
                    }),
            );
        } else {
            error!("Error closing file!");
        }
    }

    /// Processes a `CaptureMessage` from the capture worker,
    /// performing the appropriate action.
    fn message_to_action(&mut self, msg: CaptureMessage, ctx: &mut Context<Self>) {
        // Check if log
        match msg.message_type.as_str() {
            "log" => debug!(
                "Capture worker {} log message: {}",
                self.camera_id, msg.message
            ),
            "frame" => {
                if self.video_unit_id.is_none() {
                    error!("Video Unit id not set!");
                }
                WsCameraServer::from_registry().do_send(CameraFrame {
                    camera_id: self.camera_id,
                    jpeg: msg.jpeg,
                    resolution: FrameResolution::HD,
                    source: FrameSource::Camera {
                        camera_id: self.camera_id,
                    },
                    video_unit_id: self.video_unit_id.unwrap_or(-1),
                    offset: msg.offset,
                    unscaled_width: msg.unscaled_width,
                    unscaled_height: msg.unscaled_height,
                });
                self.offset += 1;
            }
            "frameScaled" => {
                WsCameraServer::from_registry().do_send(CameraFrame {
                    camera_id: self.camera_id,
                    jpeg: msg.scaled_jpeg,
                    resolution: FrameResolution::SD,
                    source: FrameSource::Camera {
                        camera_id: self.camera_id,
                    },
                    video_unit_id: self.video_unit_id.unwrap_or(-1),
                    offset: msg.offset,
                    unscaled_width: msg.unscaled_width,
                    unscaled_height: msg.unscaled_height,
                });
                self.offset += 1;
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
                                Err(e) => panic!("Error inserting video unit: {}", e),
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
                self.offset = 0;
            }
            "endFile" => {
                if let Ok(end_time) = msg.end_time.parse::<DateTime<Utc>>() {
                    self.close_file(ctx, &msg.filename, end_time);
                    self.video_unit_id = None;
                    self.video_file_id = None;
                    self.filename = None;
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
    fn handle(&mut self, item: BytesMut, ctx: &mut Context<Self>) {
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

/// Message for capture actor to start worker
struct StartWorker;

impl Message for StartWorker {
    type Result = ();
}

impl Handler<StartWorker> for CaptureActor {
    type Result = ();

    fn handle(&mut self, _msg: StartWorker, ctx: &mut Context<Self>) -> Self::Result {
        debug!("Launching worker for stream: {}", self.stream_url);
        let storage_path = Path::new(&self.storage_path).join(self.camera_id.to_string());
        if std::fs::create_dir(&storage_path).is_err() {
            // The error returned by create_dir has no information so
            // we can't really distinguish between failure
            // scenarios. If the directory already exists everything
            // is fine, otherwise we fail later.
        }
        let mut cmd = Command::new("src/cworkers/captureworker");
        cmd.arg(&self.stream_url);
        cmd.arg("0");
        cmd.arg(&storage_path);
        cmd.arg("/dev/null");
        cmd.stdout(Stdio::piped());

        let mut child = cmd.spawn_async().expect("Failed to launch");
        let stdout = child
            .stdout()
            .take()
            .expect("Failed to open stdout on worker child");
        let framed_stream = length_delimited::Builder::new().new_read(stdout);
        Self::add_stream(framed_stream, ctx);
        let fut = wrap_future::<_, Self>(child)
            .map(|_status, actor, ctx| {
                // Change this to an error when we can't distinguish
                // between intentional and unintentional exits.
                debug!("CaptureWorker {}: capture process died...", actor.camera_id);

                // Close file if open
                if let Some(filename) = &actor.filename {
                    debug!(
                        "CaptureActor {}: capture process died, closing file: {}",
                        actor.camera_id, &filename
                    );
                    actor.close_file(ctx, filename, Utc::now());
                    actor.filename = None;
                }

                ctx.notify_later(StartWorker {}, Duration::new(5, 0));
            })
            .map_err(|err, act, _ctx| {
                error!(
                    "CaptureActor {}: Error launching child process: {}",
                    act.camera_id, err
                )
            }); // Do something on error?
        ctx.spawn(fut);
    }
}

impl Actor for CaptureActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.address().do_send(StartWorker {});
    }
}
