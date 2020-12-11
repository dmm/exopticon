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

//! Supervisor for playback actors

use std::collections::HashMap;

use actix::{Actor, Addr, Context, Handler, Message, Supervised, SystemService};
use uuid::Uuid;

use crate::models::Observation;
use crate::playback_actor::PlaybackActor;
use crate::ws_session::WsSession;

/// Start playback message
#[derive(Message)]
#[rtype(result = "()")]
pub struct StartPlayback {
    /// id of playback session
    pub id: u64,
    /// `Addr` of target ws_session
    pub address: Addr<WsSession>,
    /// initial video unit id
    pub video_unit_id: Uuid,
    /// initial offset
    pub offset: i64,
    /// filename
    pub video_filename: String,
    /// observations
    pub observations: Vec<Observation>,
}

/// Stop playback message
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub struct StopPlayback {
    /// id of playback session
    pub id: u64,
}

/// Represents playback supervisor
pub struct PlaybackSupervisor {
    /// hash map of playback actors
    actors: HashMap<u64, Addr<PlaybackActor>>,
}

impl Actor for PlaybackSupervisor {
    type Context = Context<Self>;
}

impl Default for PlaybackSupervisor {
    fn default() -> Self {
        Self {
            actors: HashMap::new(),
        }
    }
}

impl SystemService for PlaybackSupervisor {}
impl Supervised for PlaybackSupervisor {}

impl Handler<StartPlayback> for PlaybackSupervisor {
    type Result = ();

    fn handle(&mut self, msg: StartPlayback, _ctx: &mut Self::Context) {
        // check if existing actor uses requested id
        self.actors.remove(&msg.id);

        let address = PlaybackActor::new(
            msg.id,
            msg.video_unit_id,
            msg.offset,
            msg.video_filename.clone(),
            msg.observations,
            msg.address.clone().recipient(),
        )
        .start();

        self.actors.insert(msg.id, address);
        debug!(
            "Created playback actor: {}, actors len: {}",
            msg.id,
            self.actors.len()
        )
    }
}

impl Handler<StopPlayback> for PlaybackSupervisor {
    type Result = ();

    fn handle(&mut self, msg: StopPlayback, _ctx: &mut Self::Context) {
        debug!(
            "Got request to kill playback actor: {}, actors len: {}",
            &msg.id,
            self.actors.len()
        );

        if let Some(address) = self.actors.get(&msg.id) {
            address.do_send(StopPlayback { id: msg.id });
            self.actors.remove(&msg.id);
        }
    }
}
