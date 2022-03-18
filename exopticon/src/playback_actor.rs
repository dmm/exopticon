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

use std::env;
use std::path::PathBuf;
use std::process::Stdio;

use actix::{
    Actor, ActorContext, AsyncContext, Context, Handler, Message, Recipient, StreamHandler,
    SystemService,
};
use bytes::BytesMut;
use exserial::models::CaptureMessage;
use tokio::process::Command;
use tokio_util::codec::length_delimited;
use uuid::Uuid;

use crate::models::Observation;
use crate::playback_supervisor::{PlaybackSupervisor, StopPlayback};
use crate::ws_camera_server::{CameraFrame, FrameResolution, FrameSource};
use crate::ws_session::{RawCameraFrame, WsMessage};

/// struct representing playback frame with list of observations. It's
/// used to send frame and observations to client.
#[derive(Message, Serialize)]
#[rtype(result = "()")]
pub struct PlaybackFrame {
    /// frame for client
    pub frame: CameraFrame,
    /// list of observations associated with frame
    pub observations: Vec<Observation>,
}

/// struct representing playback actor state
pub struct PlaybackActor {
    /// client provided playback id
    pub id: u64,
    /// id of video unit to play
    pub video_unit_id: Uuid,
    /// initial frame offset to play
    pub initial_offset: i64,
    /// path to video file to play
    pub video_file_path: String,
    /// observations included in this video unit
    pub observations: Vec<Observation>,
    /// address to send playback frames
    pub target_address: Recipient<WsMessage>,
    /// current frame count offset, not really used yet
    pub offset: i64,
}

impl Actor for PlaybackActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        debug!("Starting Playback Actor: {}", self.id);
        ctx.address().do_send(StartWorker {});
    }

    fn stopping(&mut self, _ctx: &mut Self::Context) -> actix::Running {
        debug!("Stopping Playback Actor: {}", self.id);
        actix::Running::Stop
    }
}

impl PlaybackActor {
    /// Returns newly initialized playback actor
    pub const fn new(
        id: u64, // stream id provided by client
        video_unit_id: Uuid,
        initial_offset: i64,
        video_file_path: String,
        observations: Vec<Observation>,
        target_address: Recipient<WsMessage>,
    ) -> Self {
        Self {
            id,
            video_unit_id,
            initial_offset,
            video_file_path,
            observations,
            target_address,
            offset: 0,
        }
    }

    /// Gather observations associated with selected frame
    pub fn slurp_observations(&self, offset: i64) -> Vec<Observation> {
        let mut current_obs = Vec::new();
        let iter = self.observations.iter();

        for obs in iter {
            // if within 1 millisecond
            if (obs.frame_offset - offset).abs() < 1000 {
                current_obs.push((*obs).clone());
            }
        }

        if !current_obs.is_empty() {
            debug!("Found {} observations", current_obs.len());
        }

        current_obs
    }
}

impl StreamHandler<Result<BytesMut, std::io::Error>> for PlaybackActor {
    fn handle(&mut self, item: Result<bytes::BytesMut, std::io::Error>, ctx: &mut Context<Self>) {
        let item = match item {
            Ok(b) => b,
            Err(e) => {
                error!("PlaybackActor: stream handler error! {}", e);
                // Notify WsSession that we are done
                if self
                    .target_address
                    .try_send(WsMessage::PlaybackEnd { id: self.id })
                    .is_err()
                {
                    error!("Error sending PlaybackEnd");
                }
                // Notify supervisor that we are done.
                PlaybackSupervisor::from_registry().do_send(StopPlayback { id: self.id });

                ctx.stop();
                return;
            }
        };
        let frame: Result<CaptureMessage, bincode::Error> = bincode::deserialize(&item[..]);
        match frame {
            Ok(CaptureMessage::Frame {
                jpeg,
                offset,
                unscaled_width,
                unscaled_height,
            }) => {
                let frame = RawCameraFrame {
                    camera_id: 0,
                    resolution: FrameResolution::HD,
                    unscaled_width,
                    unscaled_height,
                    source: FrameSource::Playback { id: self.id },
                    video_unit_id: self.video_unit_id,
                    offset,
                    observations: self.slurp_observations(offset),
                    jpeg,
                };
                if self
                    .target_address
                    .do_send(WsMessage::Frame(frame))
                    .is_err()
                {
                    debug!(
                        "Playback Actor: {} Unable to send message to recipient, dying..",
                        self.id
                    );
                    // Notify WsSession that we are done
                    if self
                        .target_address
                        .try_send(WsMessage::PlaybackEnd { id: self.id })
                        .is_err()
                    {
                        error!("Error sending PlaybackEnd");
                    }
                    // Notify supervisor that we are done.
                    PlaybackSupervisor::from_registry().do_send(StopPlayback { id: self.id });
                }
                self.offset += 1;
            }

            Ok(CaptureMessage::EndFile {
                filename: _,
                end_time: _,
            }) => {
                // playback worker signaled end of file
                debug!("Playback Worker for {} died!", self.video_file_path);

                // Notify WsSession that we are done
                if self
                    .target_address
                    .try_send(WsMessage::PlaybackEnd { id: self.id })
                    .is_err()
                {
                    error!("Error sending PlaybackEnd");
                }

                // Notify supervisor that we are done.
                PlaybackSupervisor::from_registry().do_send(StopPlayback { id: self.id });
            }

            Ok(_) => {
                error!("playback worker sent invalid message type");
            }

            Err(e) => error!("Error deserializing frame! {}", e),
        }
    }

    fn finished(&mut self, _ctx: &mut Self::Context) {}
}

/// Message to actor to begin playback
#[derive(Message)]
#[rtype(result = "()")]
struct StartWorker;

impl Handler<StartWorker> for PlaybackActor {
    type Result = ();

    fn handle(&mut self, _msg: StartWorker, ctx: &mut Context<Self>) -> Self::Result {
        debug!(
            "Launching playback worker for file: {}",
            self.video_file_path
        );
        let worker_path = env::var("EXOPTICONWORKERS").unwrap_or_else(|_| "/".to_string());
        let executable_path: PathBuf = [worker_path, "cworkers/playbackworker".to_string()]
            .iter()
            .collect();

        let mut cmd = Command::new(executable_path);
        cmd.arg(&self.video_file_path);
        cmd.arg(self.initial_offset.to_string());
        cmd.stdout(Stdio::piped());

        let mut child = cmd.spawn().expect("Failed to spawn playback worker.");
        let stdout = child
            .stdout
            .take()
            .expect("Failed to open stdout of playback worker.");
        let framed_stream = length_delimited::Builder::new().new_read(stdout);
        Self::add_stream(framed_stream, ctx);
        // let fut = wrap_future(child.wait()).map(|_status, actor: &mut Self, _ctx| {
        //     debug!("Playback Worker for {} died!", actor.video_file_path);

        //     // Notify WsSession that we are done
        //     if actor
        //         .target_address
        //         .try_send(WsMessage::PlaybackEnd { id: actor.id })
        //         .is_err()
        //     {
        //         error!("Error sending PlaybackEnd");
        //     }

        //     // Notify supervisor that we are done.
        //     PlaybackSupervisor::from_registry().do_send(StopPlayback { id: actor.id });
        // });

        //        ctx.spawn(fut);
    }
}

impl Handler<StopPlayback> for PlaybackActor {
    type Result = ();

    fn handle(&mut self, _msg: StopPlayback, ctx: &mut Context<Self>) -> Self::Result {
        debug!("Stopping playback for: {}", self.id);
        ctx.stop();
    }
}
