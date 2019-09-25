use std::error::Error;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use actix::fut::wrap_future;
use actix::prelude::*;

use bytes::{Bytes, BytesMut};
use log::Level;
use rmp_serde::to_vec_named;
use rmp_serde::Deserializer;
use serde::Deserialize;
use tokio::codec::length_delimited;
use tokio::prelude::Sink;
use tokio_process::CommandExt;

use crate::models::{CreateObservation, CreateObservations, DbExecutor};
use crate::ws_camera_server::{CameraFrame, FrameResolution, FrameSource, WsCameraServer};

/// Worker Log Levels
#[derive(Serialize, Deserialize)]
enum LogLevel {
    /// Error Log
    Error,
    /// Warning Log
    Warn,
    /// Info Log
    Info,
    /// Debug Log
    Debug,
    /// Trace Log
    Trace,
}

impl From<Level> for LogLevel {
    fn from(level: Level) -> Self {
        match level {
            Level::Error => LogLevel::Error,
            Level::Warn => LogLevel::Warn,
            Level::Info => LogLevel::Info,
            Level::Debug => LogLevel::Debug,
            Level::Trace => LogLevel::Trace,
        }
    }
}

/// Represents message received from worker
#[derive(Serialize, Deserialize)]
enum AnalysisWorkerMessage {
    /// Log message from worker
    Log {
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
        #[serde(with = "serde_bytes")]
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
    /// stdin of worker process
    pub worker_stdin: Option<
        tokio_codec::FramedWrite<tokio_process::ChildStdin, tokio::codec::LengthDelimitedCodec>,
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
        db_address: Addr<DbExecutor>,
    ) -> Self {
        Self {
            id,
            executable_name,
            arguments,
            worker_stdin: None,
            frames_requested: 0,
            last_frame_time: None,
            db_address,
        }
    }

    /// Processes the analysis worker message
    fn message_to_action(&mut self, msg: AnalysisWorkerMessage, ctx: &mut Context<Self>) {
        match msg {
            AnalysisWorkerMessage::Log { message } => {
                info!("Analysis Worker log: {}", message);
            }
            AnalysisWorkerMessage::FrameRequest(count) => {
                self.frames_requested = count;
            }
            AnalysisWorkerMessage::Observation(observations) => {
                debug!("Analysis observations: {}", observations.len());
                let fut = self.db_address.send(CreateObservations { observations });
                ctx.spawn(wrap_future(fut).map(|result, _actor, _ctx| {
                    match result {
                        Ok(count) => debug!("Inserted {} observations.", count),
                        Err(err) => error!("Error inserting observations: {}", err)
                    }
                }).map_err(
                    |_e, _actor, _ctx| {
                        error!("AnalysisActor: Unable to save aobservation.");
                    },
                ));
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
                    video_unit_id: 0,
                    offset: 0,
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

impl StreamHandler<BytesMut, std::io::Error> for AnalysisActor {
    fn handle(&mut self, item: BytesMut, ctx: &mut Context<Self>) {
        let mut de = Deserializer::new(&item[..]);

        let frame: Result<AnalysisWorkerMessage, rmp_serde::decode::Error> =
            Deserialize::deserialize(&mut de);

        match frame {
            Ok(f) => self.message_to_action(f, ctx),
            Err(e) => error!("Error deserializing worker message! {:?}", e.source()),
        }
    }

    fn finished(&mut self, _ctx: &mut Self::Context) {}
}

/// Message actor sends self to start the analysis worker
struct StartWorker;

impl Message for StartWorker {
    type Result = ();
}

impl Handler<StartWorker> for AnalysisActor {
    type Result = ();

    fn handle(&mut self, _msg: StartWorker, ctx: &mut Context<Self>) -> Self::Result {
        info!("Launching worker for actor: {}", self.id);

        // Initialize frames request to zero
        self.frames_requested = 0;

        let mut cmd = Command::new(self.executable_name.clone());
        for c in &self.arguments {
            cmd.arg(c);
        }
        cmd.stdout(Stdio::piped());
        cmd.stdin(Stdio::piped());
        let mut worker = match cmd.spawn_async() {
            Ok(w) => w,
            Err(err) => {
                error!("Error starting analysis worker: {}", err);
                ctx.notify_later(StartWorker, Duration::new(5, 0));
                return;
            }
        };

        let stdout = worker
            .stdout()
            .take()
            .expect("Failed to open stdout on worker child.");
        let framed_stream = length_delimited::Builder::new().new_read(stdout);
        Self::add_stream(framed_stream, ctx);

        // Frame worker stdin
        let stdin = worker
            .stdin()
            .take()
            .expect("Failed to open stdin on worker child.");
        let framed_stdin = length_delimited::Builder::new().new_write(stdin);
        self.worker_stdin = Some(framed_stdin);
        let fut = wrap_future::<_, Self>(worker)
            .map(|_status, actor, ctx| {
                info!("Analysis actor {}: analysis worker died...", actor.id);
                // Restart worker in five seconds
                ctx.notify_later(StartWorker {}, Duration::new(5, 0));
            })
            .map_err(|err, act, _ctx| {
                error!(
                    "Anaysis actor {}: Error launching child process: {}",
                    act.id, err
                )
            }); // Do something on error?

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
                let min_duration = Duration::from_millis(200);
                now.duration_since(last_frame_time) > min_duration
            }
        };
        if self.frames_requested == 0 && rate_ready {
            return;
        }

        if let Some(framed_stdin) = self.worker_stdin.take() {
            let worker_message = AnalysisWorkerCommand::Frame(msg);
            if let Ok(serialized) = to_vec_named(&worker_message) {
                self.frames_requested -= 1;
                let fut = wrap_future(framed_stdin.send(Bytes::from(serialized)))
                    .map(|sink, actor: &mut Self, _ctx| {
                        actor.worker_stdin = Some(sink);
                    })
                    .map_err(|err, _actor, _ctx| {
                        error!("Error sending on sink {}", err);
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

    avg /= timings.len() as u64;

    (avg, min, max)
}
