// This file is a part of Exopticon, a free video surveillance tool. Visit
// https://exopticon.org for more information.
//
// Copyright (C) 2019 David Matthew Mattli
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
use std::process::{Command, Stdio};

use actix::fut::wrap_future;
use actix::{
    Actor, ActorContext, ActorFuture, AsyncContext, Context, Handler, Message, Recipient,
    StreamHandler, SystemService,
};
use bytes::BytesMut;
use exserial::models::CaptureMessage;
use rmp_serde::Deserializer;
use serde::Deserialize;
use tokio::codec::length_delimited;
use tokio_process::CommandExt;

use crate::models::Observation;
use crate::playback_supervisor::{PlaybackSupervisor, StopPlayback};
use crate::ws_camera_server::{CameraFrame, FrameResolution, FrameSource};

/// struct representing playback frame with list of observations. It's
/// used to send frame and observations to client.
#[derive(Message, Serialize)]
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
    pub video_unit_id: i32,
    /// initial frame offset to play
    pub initial_offset: i32,
    /// path to video file to play
    pub video_file_path: String,
    /// observations included in this video unit
    pub observations: Vec<Observation>,
    /// address to send playback frames
    pub target_address: Recipient<PlaybackFrame>,
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
        video_unit_id: i32,
        initial_offset: i32,
        video_file_path: String,
        observations: Vec<Observation>,
        target_address: Recipient<PlaybackFrame>,
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
            if i64::from(obs.frame_offset) == offset {
                current_obs.push((*obs).clone());
            }
        }

        if !current_obs.is_empty() {
            debug!("Found {} observations", current_obs.len());
        }

        current_obs
    }
}

impl StreamHandler<BytesMut, std::io::Error> for PlaybackActor {
    fn handle(&mut self, item: BytesMut, _ctx: &mut Context<Self>) {
        let mut de = Deserializer::new(&item[..]);

        let frame: Result<CaptureMessage, rmp_serde::decode::Error> =
            Deserialize::deserialize(&mut de);

        match frame {
            Ok(CaptureMessage::Frame {
                jpeg,
                offset,
                unscaled_width,
                unscaled_height,
            }) => {
                if self
                    .target_address
                    .do_send(PlaybackFrame {
                        frame: CameraFrame {
                            camera_id: 0,
                            jpeg,
                            resolution: FrameResolution::HD,
                            source: FrameSource::Playback { id: self.id },
                            video_unit_id: self.video_unit_id,
                            offset,
                            unscaled_width,
                            unscaled_height,
                        },
                        observations: self.slurp_observations(offset),
                    })
                    .is_err()
                {
                    debug!(
                        "Playback Actor: {} Unable to send message to recipient, dying..",
                        self.id
                    );
                    // Notify supervisor that we are done.
                    PlaybackSupervisor::from_registry().do_send(StopPlayback { id: self.id });
                }
                self.offset += 1;
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
struct StartWorker;

impl Message for StartWorker {
    type Result = ();
}

impl Handler<StartWorker> for PlaybackActor {
    type Result = ();

    fn handle(&mut self, _msg: StartWorker, ctx: &mut Context<Self>) -> Self::Result {
        debug!(
            "Launching playback worker for file: {}",
            self.video_file_path
        );
        let mut cmd = Command::new("src/cworkers/playbackworker");
        cmd.arg(&self.video_file_path);
        cmd.arg(self.initial_offset.to_string());
        cmd.stdout(Stdio::piped());

        let mut child = cmd.spawn_async().expect("Failed to spawn playback worker.");
        let stdout = child
            .stdout()
            .take()
            .expect("Failed to open stdout of playback worker.");
        let framed_stream = length_delimited::Builder::new().new_read(stdout);
        Self::add_stream(framed_stream, ctx);
        let fut = wrap_future(child)
            .map(|_status, actor: &mut Self, _ctx| {
                debug!("Playback Worker for {} died!", actor.video_file_path);

                // Notify supervisor that we are done.
                PlaybackSupervisor::from_registry().do_send(StopPlayback { id: actor.id });
            })
            .map_err(|err, act, _ctx| {
                error!(
                    "Playback Worker: {} error launching child process {}",
                    act.video_file_path, err
                );
            });

        ctx.spawn(fut);
    }
}

impl Handler<StopPlayback> for PlaybackActor {
    type Result = ();

    fn handle(&mut self, _msg: StopPlayback, ctx: &mut Context<Self>) -> Self::Result {
        debug!("Stopping playback for: {}", self.id);
        ctx.stop();
    }
}
