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

use std::collections::HashMap;
use std::time::Duration;

use actix::fut::wrap_future;
use actix::prelude::*;

use crate::analysis_actor::AnalysisActor;
use crate::db_registry;
use crate::models::{AnalysisSubscriptionModel, FetchAllAnalysisModel};
use crate::ws_camera_server::{
    FrameResolution, FrameSource, Subscribe, SubscriptionSubject, WsCameraServer,
};

/// Message telling supervisor to start new analysis actor
#[derive(Serialize, Deserialize)]
pub struct StartAnalysisActor {
    /// id of analysis instance
    pub id: i32,
    /// name of executable implementing analysis worker
    pub executable_name: String,
    /// arguments to provide analysis worker on startup
    pub arguments: Vec<String>,
    /// max frames-per-second to send to worker
    pub max_fps: i32,
    /// frame source subscriptions for actor
    pub subscriptions: Vec<AnalysisSubscriptionModel>,
}

impl Message for StartAnalysisActor {
    type Result = i32;
}

/// Message telling supervisor to stop existing analysis actor
pub struct StopAnalysisActor {
    /// id of analysis worker to stop
    pub id: i32,
}

impl Message for StopAnalysisActor {
    type Result = ();
}

/// Message requesting an `AnalysisWorker` restart
pub struct RestartAnalysisActor {
    /// id of analysis instance
    pub id: i32,
    /// name of executable implementing analysis worker
    pub executable_name: String,
    /// arguments to provide analysis worker on startup
    pub arguments: Vec<String>,
    /// max frames-per-second to send to worker
    pub max_fps: i32,
    /// frame source subscriptions
    pub subscriptions: Vec<AnalysisSubscriptionModel>,
}

impl Message for RestartAnalysisActor {
    type Result = ();
}

/// A `Message` requesting the `AnalysisSupervisor` restart
/// `AnalysisActor`s to accomodate changesh
pub struct SyncAnalysisActors {}

impl Message for SyncAnalysisActors {
    type Result = ();
}

/// `AnalysisSupervisor` actor
pub struct AnalysisSupervisor {
    /// supervised actors
    actors: HashMap<i32, Addr<AnalysisActor>>,
}

impl Actor for AnalysisSupervisor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        ctx.notify(SyncAnalysisActors {});
    }
}

impl Default for AnalysisSupervisor {
    fn default() -> Self {
        Self {
            actors: HashMap::new(),
        }
    }
}

impl SystemService for AnalysisSupervisor {}
impl Supervised for AnalysisSupervisor {}

impl Handler<StartAnalysisActor> for AnalysisSupervisor {
    type Result = i32;

    fn handle(&mut self, msg: StartAnalysisActor, _ctx: &mut Context<Self>) -> Self::Result {
        info!("Starting analysis actor id: {}", msg.id);
        let actor = AnalysisActor::new(
            msg.id,
            msg.executable_name,
            msg.arguments,
            msg.max_fps,
            msg.subscriptions.clone(),
            db_registry::get_db(),
        );
        let address = actor.start();
        self.actors.insert(msg.id, address.clone());

        for sub in msg.subscriptions {
            // setup camera subscriptions
            let subject = match sub.source {
                FrameSource::Camera { camera_id } => {
                    SubscriptionSubject::Camera(camera_id, FrameResolution::SD)
                }
                FrameSource::AnalysisEngine {
                    analysis_engine_id, ..
                } => SubscriptionSubject::AnalysisEngine(analysis_engine_id),
                FrameSource::Playback { id } => SubscriptionSubject::Playback(id, 0, 0),
            };
            WsCameraServer::from_registry().do_send(Subscribe {
                subject,
                client: address.clone().recipient(),
            });
        }

        msg.id
    }
}

impl Handler<StopAnalysisActor> for AnalysisSupervisor {
    type Result = ();

    fn handle(&mut self, msg: StopAnalysisActor, _ctx: &mut Context<Self>) -> Self::Result {
        info!("Stopping analysis actor id: {}", &msg.id);
        self.actors.remove(&msg.id);
    }
}

impl Handler<RestartAnalysisActor> for AnalysisSupervisor {
    type Result = ();

    fn handle(&mut self, msg: RestartAnalysisActor, ctx: &mut Context<Self>) -> Self::Result {
        info!("Restarting analysis actor id: {}", msg.id);
        let fut = wrap_future(ctx.address().send(StopAnalysisActor { id: msg.id })).map(
            |_res, _act: &mut Self, ctx: &mut Context<Self>| {
                ctx.notify_later(
                    StartAnalysisActor {
                        id: msg.id,
                        executable_name: msg.executable_name,
                        arguments: msg.arguments,
                        max_fps: msg.max_fps,
                        subscriptions: msg.subscriptions,
                    },
                    Duration::new(5, 0),
                );
            },
        );
        ctx.spawn(fut);
    }
}

// Right now this handler only restarts actors, eventually it should
// restart them only when changed.
impl Handler<SyncAnalysisActors> for AnalysisSupervisor {
    type Result = ();

    fn handle(&mut self, _msg: SyncAnalysisActors, ctx: &mut Context<Self>) -> Self::Result {
        // fetch analysis engines
        let fut = wrap_future(db_registry::get_db().send(FetchAllAnalysisModel {})).map(
            |res, _act, inner_ctx: &mut Context<Self>| {
                if let Ok(Ok(res)) = res {
                    for engine in res {
                        for a in engine.1 {
                            inner_ctx.notify(RestartAnalysisActor {
                                id: a.id,
                                executable_name: engine.0.entry_point.clone(),
                                arguments: Vec::new(),
                                max_fps: a.max_fps,
                                subscriptions: a.subscriptions,
                            });
                        }
                    }
                }
            },
        );

        ctx.spawn(fut);
    }
}

impl AnalysisSupervisor {
    /// Returns new `AnalysisSupervisor`
    pub fn new() -> Self {
        Self {
            actors: HashMap::new(),
        }
    }
}
