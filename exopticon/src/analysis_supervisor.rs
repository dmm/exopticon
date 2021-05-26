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

use actix::prelude::*;
use actix_interop::{critical_section, with_ctx, FutureInterop};
use prometheus::{opts, IntCounterVec, Registry};

use crate::models::{AnalysisEngine, AnalysisSubscriptionModel, FetchAllAnalysisModel};
use crate::prom_registry;
use crate::ws_camera_server::{Subscribe, WsCameraServer};
use crate::{analysis_actor::AnalysisActor, models::AnalysisInstanceModel};
use crate::{db_registry, ws_camera_server::Unsubscribe};

#[derive(Clone)]
pub struct AnalysisMetrics {
    pub process_count: IntCounterVec,
    pub restart_count: IntCounterVec,
}

impl AnalysisMetrics {
    pub fn new() -> Self {
        Self {
            process_count: IntCounterVec::new(
                opts!(
                    "analysis_process_count",
                    "Number of frames processed by analysis instance"
                )
                .namespace("exopticon"),
                &["instance_id", "instance_name"],
            )
            .expect("Unable to create analysis metric"),

            restart_count: IntCounterVec::new(
                opts!(
                    "analysis_restart_count",
                    "Number of unplanned restarts by analysis instance"
                )
                .namespace("exopticon"),
                &["instance_id", "instance_name"],
            )
            .expect("Unable to create analysis metric"),
        }
    }

    pub fn register(&self, registry: &Registry) -> Result<(), ()> {
        registry
            .register(Box::new(self.process_count.clone()))
            .map_err(|_| ())?;
        registry
            .register(Box::new(self.restart_count.clone()))
            .map_err(|_| ())?;

        Ok(())
    }
}

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
pub struct RestartAnalysisActor {}

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
    actors: HashMap<
        i32,
        (
            AnalysisEngine,
            AnalysisInstanceModel,
            Option<Addr<AnalysisActor>>,
        ),
    >,
    metrics: AnalysisMetrics,
}

impl Actor for AnalysisSupervisor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        match self
            .metrics
            .register(&prom_registry::get_metrics().registry)
        {
            Ok(()) => (),
            Err(()) => error!("Failed to register analysis metrics!"),
        }
        ctx.notify(SyncAnalysisActors {});
        ctx.notify(RestartAnalysisActor {});
    }
}

impl Default for AnalysisSupervisor {
    fn default() -> Self {
        Self {
            actors: HashMap::new(),
            metrics: AnalysisMetrics::new(),
        }
    }
}

impl SystemService for AnalysisSupervisor {}
impl Supervised for AnalysisSupervisor {}

impl Handler<StopAnalysisActor> for AnalysisSupervisor {
    type Result = ();

    fn handle(&mut self, msg: StopAnalysisActor, _ctx: &mut Context<Self>) -> Self::Result {
        info!("Stopping analysis actor id: {}", &msg.id);
        self.actors.remove(&msg.id);
    }
}

impl Handler<RestartAnalysisActor> for AnalysisSupervisor {
    type Result = ();

    fn handle(&mut self, _msg: RestartAnalysisActor, ctx: &mut Context<Self>) -> Self::Result {
        let fut = async move {
            critical_section::<Self, _>(async move {
                with_ctx(|actor: &mut Self, ctx: &mut Context<Self>| {
                    for (engine, instance, addr) in actor.actors.values_mut() {
                        if addr.is_none() && instance.enabled {
                            info!("Restarting analysis actor id: {}", instance.id);
                            let new_addr =
                                Self::start_actor(engine, instance, actor.metrics.clone());
                            *addr = Some(new_addr);
                        }

                        if let Some(addr2) = addr {
                            if !addr2.connected() && instance.enabled {
                                info!("Restarting analysis actor id: {}", instance.id);
                                let new_addr =
                                    Self::start_actor(engine, instance, actor.metrics.clone());
                                *addr = Some(new_addr);
                            }
                        }
                    }
                    ctx.notify_later(RestartAnalysisActor {}, Duration::from_secs(10));
                });
            })
            .await;
        }
        .interop_actor(self);

        ctx.spawn(fut);
    }
}

// Right now this handler only restarts actors, eventually it should
// restart them only when changed.
impl Handler<SyncAnalysisActors> for AnalysisSupervisor {
    type Result = ();

    fn handle(&mut self, _msg: SyncAnalysisActors, ctx: &mut Context<Self>) -> Self::Result {
        // fetch analysis engines
        let fut = async move {
            critical_section::<Self, _>(async move {
                let analysis_instances =
                    match db_registry::get_db().send(FetchAllAnalysisModel {}).await {
                        Ok(Ok(n)) => n,
                        Ok(Err(_)) | Err(_) => {
                            error!("Failed to load analysis instances during sync!");
                            return;
                        }
                    };

                with_ctx(|actor: &mut Self, _| {
                    actor.clear_actors();
                    for engine in &analysis_instances {
                        for instance in &engine.1 {
                            actor
                                .actors
                                .insert(instance.id, (engine.0.clone(), instance.clone(), None));
                        }
                    }
                });
            })
            .await;
        }
        .interop_actor(self);
        ctx.spawn(fut);
    }
}

impl AnalysisSupervisor {
    pub fn start_actor(
        engine: &AnalysisEngine,
        new_actor: &AnalysisInstanceModel,
        metrics: AnalysisMetrics,
    ) -> Addr<AnalysisActor> {
        info!("Starting analysis actor id: {}", new_actor.id);
        let actor = AnalysisActor::new(
            new_actor.id,
            engine.entry_point.clone(),
            Vec::new(),
            new_actor.max_fps,
            new_actor.subscriptions.clone(),
            db_registry::get_db(),
            metrics,
        );
        let address = actor.start();

        for sub in &new_actor.subscriptions {
            // setup camera subscriptions
            WsCameraServer::from_registry().do_send(Subscribe {
                subject: sub.source.clone(),
                client: address.clone().recipient(),
            });
        }

        address
    }
    pub fn clear_actors(&mut self) {
        for (_, (_, instance, addr)) in self.actors.drain() {
            if let Some(addr) = addr {
                // actor is alive
                // remove frame subscriptions
                for sub in instance.subscriptions {
                    // Unsubscribe actor from frames
                    WsCameraServer::from_registry().do_send(Unsubscribe {
                        subject: sub.source.clone(),
                        client: addr.clone().recipient(),
                    });
                }
                // Ask actor to die
                addr.do_send(StopAnalysisActor { id: instance.id });
                // because we are 'drain'ing the HashMap the addr will
                // be dropped. This should be the last reference so
                // the analysis actor should be cleaned up after this.
            }
        }
    }
}
