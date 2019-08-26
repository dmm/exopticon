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
use rmp_serde::Deserializer;
use serde::Deserialize;
use tokio::codec::length_delimited;
use tokio_process::CommandExt;

use crate::capture_actor::CaptureMessage;
use crate::playback_supervisor::{PlaybackSupervisor, StopPlayback};
use crate::ws_camera_server::{CameraFrame, FrameResolution, FrameSource};

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
    /// address to send playback frames
    pub target_address: Recipient<CameraFrame>,
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
        target_address: Recipient<CameraFrame>,
    ) -> Self {
        Self {
            id,
            video_unit_id,
            initial_offset,
            video_file_path,
            target_address,
        }
    }
}

impl StreamHandler<BytesMut, std::io::Error> for PlaybackActor {
    fn handle(&mut self, item: BytesMut, ctx: &mut Context<Self>) {
        let mut de = Deserializer::new(&item[..]);

        let frame: Result<CaptureMessage, rmp_serde::decode::Error> =
            Deserialize::deserialize(&mut de);

        match frame {
            Ok(f) => {
                if f.message_type == "frame"
                    && self
                        .target_address
                        .do_send(CameraFrame {
                            camera_id: 0,
                            jpeg: f.jpeg,
                            resolution: FrameResolution::HD,
                            source: FrameSource::Playback { id: self.id },
                            video_unit_id: self.video_unit_id,
                            offset: f.offset,
                        })
                        .is_err()
                {
                    debug!(
                        "Playback Actor: {} Unable to send message to recipient, dying..",
                        self.id
                    );
                    ctx.stop();
                }
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
