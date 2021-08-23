/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
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

// clippy doesn't like the base64_serde_type macro
#![allow(clippy::empty_enum)]

use std::collections::HashMap;
use std::convert::TryInto;
use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use actix::fut::wrap_future;
use actix::prelude::*;
use base64::STANDARD_NO_PAD;
use bytes::BytesMut;
use chrono::Utc;
use futures::sink::SinkExt;
use log::Level;
use serde::Deserialize;
use tokio::process::Command;
use tokio_util::codec::length_delimited;
use uuid::Uuid;

use crate::fair_queue::FairQueue;
use crate::models::{
    AnalysisSubscriptionModel, CreateObservation, CreateObservationSnapshot, CreateObservations,
    DbExecutor, Observation, SubscriptionMask,
};
use crate::ws_camera_server::{
    CameraFrame, FrameResolution, FrameSource, SubscriptionSubject, WsCameraServer,
};
use crate::{
    analysis_supervisor::{AnalysisMetrics, StopAnalysisActor},
    models::CreateEvent,
};

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
        level: i32,
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
    Event(CreateEvent),
}

/// Represents message sent to worker
#[derive(Serialize, Deserialize)]
enum AnalysisWorkerCommand {
    /// A frame of video send for analysis
    Frame {
        /// Camera frame
        frame: CameraFrame,
        /// regions in frame to mask out
        masks: Vec<SubscriptionMask>,
    },
}

/// Analysis Actor context
pub struct AnalysisActor {
    /// id of analysis instance
    pub id: i32,
    /// executable to run
    pub executable_name: String,
    /// arguments to provide to executable
    pub arguments: Vec<String>,
    /// frame sources
    pub subscriptions: HashMap<SubscriptionSubject, AnalysisSubscriptionModel>,
    /// stdin of worker process
    pub worker_stdin: Option<
        tokio_util::codec::FramedWrite<
            tokio::process::ChildStdin,
            tokio_util::codec::LengthDelimitedCodec,
        >,
    >,
    /// number of frames requested by worker process
    pub frames_requested: u8,
    /// last frame sent to worker
    pub last_frame: Option<CameraFrame>,
    /// max frames-per-second to send to worker
    pub max_fps: i32,
    /// represents the previous time a frame was sent to worker
    pub last_frame_time: Option<Instant>,
    /// Queue of frames to feed analysis worker
    pub frame_queue: FairQueue<SubscriptionSubject, CameraFrame>,
    /// Address of database actor
    pub db_address: Addr<DbExecutor>,
    /// metrics
    pub metrics: AnalysisMetrics,
}

impl AnalysisActor {
    /// Returns initialized `AnalysisActor`
    pub fn new(
        id: i32,
        executable_name: String,
        arguments: Vec<String>,
        max_fps: i32,
        subscriptions: Vec<AnalysisSubscriptionModel>,
        db_address: Addr<DbExecutor>,
        metrics: AnalysisMetrics,
    ) -> Self {
        let mut sub_map = HashMap::new();
        for s in subscriptions {
            sub_map.insert(s.source.clone(), s);
        }

        Self {
            id,
            executable_name,
            arguments,
            subscriptions: sub_map,
            worker_stdin: None,
            frames_requested: 0,
            last_frame: None,
            max_fps,
            last_frame_time: None,
            frame_queue: FairQueue::new(),
            db_address,
            metrics,
        }
    }

    /// Processes the analysis worker message
    #[allow(clippy::too_many_lines)]
    fn message_to_action(&mut self, msg: AnalysisWorkerMessage, ctx: &mut Context<Self>) {
        match msg {
            AnalysisWorkerMessage::Log { level, message } => {
                let log_level = match level {
                    50 | 40 => Level::Error,
                    30 => Level::Warn,
                    20 => Level::Info,
                    10 => Level::Debug,
                    _ => Level::Trace,
                };
                log!(
                    log_level,
                    "Analysis worker {} log message: {}",
                    self.id,
                    message
                );
            }
            AnalysisWorkerMessage::FrameRequest(count) => {
                self.frames_requested = count;
                self.push_frame(ctx);
            }
            AnalysisWorkerMessage::Observation(observations) => {
                if let Some(mut frame) = self.last_frame.take() {
                    let new_obs: Vec<Observation> = observations
                        .iter()
                        .map(|o| Observation {
                            id: 0,
                            frame_offset: o.frame_offset,
                            tag: o.tag.clone(),
                            details: o.details.clone(),
                            score: o.score,
                            ul_x: o.ul_x,
                            ul_y: o.ul_y,
                            lr_x: o.lr_x,
                            lr_y: o.lr_y,
                            inserted_at: Utc::now(),
                            video_unit_id: o.video_unit_id,
                        })
                        .collect();

                    let offset = match frame.source {
                        FrameSource::Camera {
                            camera_id: _,
                            analysis_offset,
                        }
                        | FrameSource::AnalysisEngine {
                            analysis_engine_id: _,
                            analysis_offset,
                        } => analysis_offset,
                        FrameSource::Playback { id } => {
                            panic!("Playback is not a valid analysis source {}", id);
                        }
                    };
                    frame.source = FrameSource::AnalysisEngine {
                        analysis_engine_id: self.id,
                        analysis_offset: offset,
                    };

                    frame.observations = new_obs;
                    WsCameraServer::from_registry().do_send(frame);

                    let new_creates = observations
                        .into_iter()
                        .filter(|x| x.tag != "motion")
                        .collect();
                    let fut = self.db_address.send(CreateObservations {
                        observations: new_creates,
                    });
                    ctx.spawn(wrap_future(fut).map(|result, _actor: &mut Self, _ctx| {
                        if let Ok(Ok(_)) = result {
                        } else {
                            error!("Error inserting observations!")
                        }
                    }));
                }
            }
            AnalysisWorkerMessage::FrameReport { tag: _, jpeg } => {
                WsCameraServer::from_registry().do_send(CameraFrame {
                    camera_id: 0,
                    jpeg,
                    observations: Vec::new(),
                    resolution: FrameResolution::SD,
                    source: FrameSource::AnalysisEngine {
                        analysis_engine_id: self.id,
                        analysis_offset: Duration::from_secs(0),
                    },
                    video_unit_id: Uuid::nil(),
                    offset: -1,
                    unscaled_width: -1,
                    unscaled_height: -1,
                });
            }
            AnalysisWorkerMessage::TimingReport { tag, times } => {
                let (avg, min, max) = calculate_statistics(&times);
                debug!(
                    "Analysis Actor got {} time report! {:.2} avg, {:.2} min, {:.2} max",
                    tag,
                    avg / 1000,
                    min / 1000,
                    max / 1000
                )
            }
            AnalysisWorkerMessage::Event(event) => {
                let db = self.db_address.clone();
                let fut = async move {
                    let ob_id = event.display_observation_id;
                    match db.send(event).await {
                        Ok(Ok(event)) => {
                            debug!("Inserted event! {:?}", event);
                        }

                        Ok(Err(e)) => {
                            error!("Failed to add event: {}", e);
                        }
                        Err(e) => {
                            error!("Failed to add event: {}", e);
                        }
                    };

                    match db
                        .send(CreateObservationSnapshot {
                            observation_id: ob_id,
                        })
                        .await
                    {
                        Ok(Ok(event)) => {
                            debug!("Inserted observation snapshot! {:?}", event);
                        }

                        Ok(Err(e)) => {
                            error!("Failed to add observation snapshot: {}", e);
                        }
                        Err(e) => {
                            error!("Failed to add observation snapshot: {}", e);
                        }
                    };
                };

                ctx.spawn(wrap_future(fut));
            }
        }
    }

    /// Push a frame to the worker if the worker is ready to receive a frame
    fn push_frame(&mut self, ctx: &mut Context<Self>) {
        // If no frames requested or no frames to send, do nothing.
        if self.frames_requested == 0 || self.frame_queue.len() == 0 {
            return;
        }

        if self.max_fps != 0 {
            if let Some(last_frame_time) = self.last_frame_time {
                let interval_millis: u64 = (1000 / self.max_fps).try_into().unwrap_or(0);
                let frame_interval = Duration::from_millis(interval_millis);
                let time_since_last_frame = Instant::now().duration_since(last_frame_time);
                if time_since_last_frame < frame_interval {
                    return;
                }
            }
        }

        self.metrics
            .process_count
            .with_label_values(&[&self.id.to_string(), ""])
            .inc_by(1);

        // Take ownership of the worker's stdin. If any of the below
        // conditionals fail this will be dropped, closing stdin. That
        // will make the worker fail so it shouldn't happen.
        if let Some(mut framed_stdin) = self.worker_stdin.take() {
            // we know there is a frame to send because of the initial
            // check. Is there a better way?
            if let Some(frame) = self.frame_queue.pop_front() {
                let masks = match self.subscriptions.get(&SubscriptionSubject::from(&frame)) {
                    None => Vec::new(),
                    Some(source) => source.masks.clone(),
                };
                let worker_message = AnalysisWorkerCommand::Frame {
                    frame: frame.clone(),
                    masks,
                };

                if let Ok(serialized) = serde_json::to_string(&worker_message) {
                    self.frames_requested -= 1;
                    self.last_frame = Some(frame);

                    let task = async move {
                        if let Err(err) = framed_stdin.send(bytes::Bytes::from(serialized)).await {
                            error!("Analysis Worker: Failed to write frame: {}", err)
                        };
                        framed_stdin
                    };
                    let fut = wrap_future(task).map(|sink, actor: &mut Self, _ctx| {
                        actor.worker_stdin = Some(sink);
                        actor.last_frame_time = Some(Instant::now());
                    });
                    ctx.spawn(fut);
                }
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

        let worker_path = env::var("EXOPTICONWORKERS").unwrap_or_else(|_| "/".to_string());
        let executable_path: PathBuf = [worker_path, self.executable_name.clone()].iter().collect();
        let mut cmd = Command::new(executable_path);
        for c in &self.arguments {
            cmd.arg(c);
        }
        cmd.stdout(Stdio::piped());
        cmd.stdin(Stdio::piped());
        //        cmd.stderr(Stdio::null());
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

            actor
                .metrics
                .restart_count
                .with_label_values(&[&actor.id.to_string(), ""])
                .inc_by(1);

            // Restart worker in five seconds
            ctx.notify_later(StartWorker {}, Duration::new(5, 0));
        });

        ctx.spawn(fut);
    }
}

impl Handler<CameraFrame> for AnalysisActor {
    type Result = ();

    fn handle(&mut self, msg: CameraFrame, ctx: &mut Context<Self>) -> Self::Result {
        // Enqueue received frame
        self.frame_queue
            .push_back(SubscriptionSubject::from(&msg), msg);

        self.push_frame(ctx);
    }
}

impl Handler<StopAnalysisActor> for AnalysisActor {
    type Result = ();

    fn handle(&mut self, _msg: StopAnalysisActor, ctx: &mut Context<Self>) -> Self::Result {
        self.worker_stdin = None;
        ctx.stop();
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
