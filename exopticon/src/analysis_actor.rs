use std::convert::TryInto;
use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use actix::fut::wrap_future;
use actix::prelude::*;
use base64::STANDARD_NO_PAD;
use bytes::BytesMut;
use futures::sink::SinkExt;
use log::Level;
use serde::Deserialize;
use serde_json;
use tokio::process::Command;
use tokio_util::codec::length_delimited;

use crate::analysis_supervisor::{AnalysisSupervisor, RestartAnalysisActor};
use crate::models::{CreateObservation, CreateObservations, DbExecutor};
use crate::ws_camera_server::{CameraFrame, FrameResolution, FrameSource, WsCameraServer};

base64_serde_type!(Base64Standard, STANDARD_NO_PAD);

/// Represents logging levels from ffmpeg
#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum LogLevel {
    /// Critical log
    Critical,
    /// Error log
    Error,
    /// Warning log
    Warning,
    /// Info log
    Info,
    /// Debug log
    Debug,
    /// Not set? Interpreted as debug log
    NotSet,
}

impl From<Level> for LogLevel {
    #[must_use]
    fn from(item: Level) -> Self {
        match item {
            Level::Error => Self::Error,
            Level::Warn => Self::Warning,
            Level::Info => Self::Info,
            Level::Debug | Level::Trace => Self::Debug,
        }
    }
}

impl From<LogLevel> for Level {
    #[must_use]
    fn from(item: LogLevel) -> Self {
        match item {
            LogLevel::Critical | LogLevel::Error => Self::Error,
            LogLevel::Warning => Self::Warn,
            LogLevel::Info => Self::Info,
            LogLevel::Debug | LogLevel::NotSet => Self::Debug,
        }
    }
}

/// Represents message received from worker
#[derive(Serialize, Deserialize)]
enum AnalysisWorkerMessage {
    /// Log message from worker
    Log {
        /// log level index
        level: LogLevel,
        /// Worker log message
        message: String,
    },
    /// A request to send more frames
    FrameRequest(u8),
    /// Observation message from worker
    Observation(Vec<CreateObservation>),
    /// Processed frame from worker
    FrameReport {
        /// tag identifying frame report
        tag: String,
        /// image data
        #[serde(with = "Base64Standard")]
        jpeg: Vec<u8>,
    },
    /// Timing report
    TimingReport {
        /// tag identifying timing type
        tag: String,
        /// timing values
        times: Vec<u64>,
    },
}

/// Represents message sent to worker
#[derive(Serialize, Deserialize)]
enum AnalysisWorkerCommand {
    /// A frame of video send for analysis
    Frame(CameraFrame),
}

/// Analysis Actor context
pub struct AnalysisActor {
    /// actor id
    pub id: i32,
    /// executable to run
    pub executable_name: String,
    /// arguments to provide to executable
    pub arguments: Vec<String>,
    /// ids of subscribed cameras
    pub subscribed_camera_ids: Vec<i32>,
    /// stdin of worker process
    pub worker_stdin: Option<
        tokio_util::codec::FramedWrite<
            tokio::process::ChildStdin,
            tokio_util::codec::LengthDelimitedCodec,
        >,
    >,
    /// number of frames requested by worker process
    pub frames_requested: u8,
    /// represents the previous time a frame was sent to worker
    pub last_frame_time: Option<Instant>,
    /// Address of database actor
    pub db_address: Addr<DbExecutor>,
}

impl AnalysisActor {
    /// Returns initialized `AnalysisActor`
    pub const fn new(
        id: i32,
        executable_name: String,
        arguments: Vec<String>,
        subscribed_camera_ids: Vec<i32>,
        db_address: Addr<DbExecutor>,
    ) -> Self {
        Self {
            id,
            executable_name,
            arguments,
            subscribed_camera_ids,
            worker_stdin: None,
            frames_requested: 0,
            last_frame_time: None,
            db_address,
        }
    }

    /// Processes the analysis worker message
    fn message_to_action(&mut self, msg: AnalysisWorkerMessage, ctx: &mut Context<Self>) {
        match msg {
            AnalysisWorkerMessage::Log { level, message } => {
                log!(level.into(), "Analysis Worker log: {}", message)
            }
            AnalysisWorkerMessage::FrameRequest(count) => {
                self.frames_requested = count;
            }
            AnalysisWorkerMessage::Observation(observations) => {
                debug!("Analysis observations: {}", observations.len());
                let fut = self.db_address.send(CreateObservations { observations });
                ctx.spawn(wrap_future(fut).map(|result, _actor, _ctx| {
                    if let Ok(Ok(count)) = result {
                        debug!("Inserted {} observations.", count)
                    } else {
                        error!("Error inserting observations!")
                    }
                }));
            }
            AnalysisWorkerMessage::FrameReport { tag, jpeg } => {
                WsCameraServer::from_registry().do_send(CameraFrame {
                    camera_id: 0,
                    jpeg,
                    resolution: FrameResolution::SD,
                    source: FrameSource::AnalysisEngine {
                        analysis_engine_id: self.id,
                        tag,
                    },
                    video_unit_id: -1,
                    offset: -1,
                    unscaled_width: -1,
                    unscaled_height: -1,
                });
            }
            AnalysisWorkerMessage::TimingReport { tag, times } => {
                let (avg, min, max) = calculate_statistics(&times);
                info!(
                    "Analysis Actor got {} time report! {:.2} avg, {:.2} min, {:.2} max",
                    tag,
                    avg / 1000,
                    min / 1000,
                    max / 1000
                )
            }
        }
    }
}

impl Actor for AnalysisActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        debug!("Analysis actor starting!");
        ctx.address().do_send(StartWorker {});
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("Analysis actor stopping!");
    }
}

impl StreamHandler<Result<BytesMut, std::io::Error>> for AnalysisActor {
    fn handle(&mut self, item: Result<BytesMut, std::io::Error>, ctx: &mut Context<Self>) {
        let item = match item {
            Ok(b) => b,
            Err(err) => {
                error!("Caught error! {}", err);
                AnalysisSupervisor::from_registry().do_send(RestartAnalysisActor {
                    id: self.id,
                    executable_name: self.executable_name.clone(),
                    arguments: self.arguments.clone(),
                    subscribed_camera_ids: self.subscribed_camera_ids.clone(),
                });
                ctx.terminate();
                return;
            }
        };
        let frame: Result<AnalysisWorkerMessage, serde_json::error::Error> =
            serde_json::from_slice(&item);

        match frame {
            Ok(f) => self.message_to_action(f, ctx),
            Err(e) => error!("Error deserializing worker message! {:?}", e),
        }
    }

    fn finished(&mut self, _ctx: &mut Self::Context) {}
}

/// Message actor sends self to start the analysis worker
#[derive(Message)]
#[rtype(result = "()")]
struct StartWorker;

impl Handler<StartWorker> for AnalysisActor {
    type Result = ();

    fn handle(&mut self, _msg: StartWorker, ctx: &mut Context<Self>) -> Self::Result {
        info!("Launching worker for actor: {}", self.id);

        // Initialize frames request to zero
        self.frames_requested = 0;

        let worker_path = env::var("EXOPTICONWORKERS").unwrap_or("/".to_string());
        let executable_path: PathBuf = [worker_path, self.executable_name.clone()].iter().collect();
        let mut cmd = Command::new(executable_path);
        for c in &self.arguments {
            cmd.arg(c);
        }
        cmd.stdout(Stdio::piped());
        cmd.stdin(Stdio::piped());
        let mut worker = match cmd.spawn() {
            Ok(w) => w,
            Err(err) => {
                error!("Error starting analysis worker: {}", err);
                ctx.notify_later(StartWorker, Duration::new(5, 0));
                return;
            }
        };

        let stdout = worker
            .stdout
            .take()
            .expect("Failed to open stdout on worker child.");
        let framed_stream = length_delimited::Builder::new().new_read(stdout);
        Self::add_stream(framed_stream, ctx);

        // Frame worker stdin
        let stdin = worker
            .stdin
            .take()
            .expect("Failed to open stdin on worker child.");
        let framed_stdin = length_delimited::Builder::new().new_write(stdin);
        self.worker_stdin = Some(framed_stdin);
        let fut = wrap_future::<_, Self>(worker).map(|_status, actor, ctx| {
            info!("Analysis actor {}: analysis worker died...", actor.id);
            // Restart worker in five seconds
            ctx.notify_later(StartWorker {}, Duration::new(5, 0));
        });

        ctx.spawn(fut);
    }
}

impl Handler<CameraFrame> for AnalysisActor {
    type Result = ();

    fn handle(&mut self, msg: CameraFrame, ctx: &mut Context<Self>) -> Self::Result {
        let rate_ready = match self.last_frame_time {
            None => true,
            Some(last_frame_time) => {
                let now = Instant::now();
                let min_duration = Duration::from_millis(0); // FIXME
                now.duration_since(last_frame_time) > min_duration
            }
        };
        if self.frames_requested == 0 && rate_ready {
            return;
        }

        let worker_message = AnalysisWorkerCommand::Frame(msg);
        if let Some(mut framed_stdin) = self.worker_stdin.take() {
            if let Ok(serialized) = serde_json::to_string(&worker_message) {
                self.frames_requested -= 1;

                let task = async move {
                    if let Err(err) = framed_stdin.send(bytes::Bytes::from(serialized)).await {
                        error!("Analysis Worker: Failed to write frame: {}", err)
                    };
                    framed_stdin
                };
                let fut = wrap_future(task).map(|sink, actor: &mut Self, _ctx| {
                    actor.worker_stdin = Some(sink);
                });
                ctx.spawn(fut);
            }
        }
    }
}

/// Calculate average, min, and max, all times in microseconds
fn calculate_statistics(timings: &[u64]) -> (u64, u64, u64) {
    let mut min: u64 = u64::max_value();
    let mut max: u64 = u64::min_value();
    let mut avg: u64 = 0;

    for t in timings {
        if *t < min {
            min = *t;
        }
        if *t > max {
            max = *t;
        }
        // add to average and convert to microseconds
        avg += *t;
    }

    let len: u64 = timings
        .len()
        .try_into()
        .expect("u64 overflow in calculate_timings!");

    avg /= len;
    (avg, min, max)
}
