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

use std::time::Duration;

use actix::prelude::*;
use actix_interop::{critical_section, with_ctx, FutureInterop};
use prometheus::{opts, IntCounterVec, Registry};

use crate::capture_actor::CaptureActor;
use crate::db_registry;
use crate::models::{Camera, FetchAllCameraGroupAndCameras};
use crate::prom_registry;

#[derive(Clone)]
pub struct CaptureMetrics {
    pub restart_count: IntCounterVec,
}

impl CaptureMetrics {
    pub fn new() -> Self {
        Self {
            restart_count: IntCounterVec::new(
                opts!(
                    "capture_restart_count",
                    "Number of unplanned restarts of capture worker"
                )
                .namespace("exopticon"),
                &["camera_id", "camera_name"],
            )
            .expect("Unable to create capture metric"),
        }
    }

    pub fn register(&self, registry: &Registry) -> Result<(), ()> {
        registry
            .register(Box::new(self.restart_count.clone()))
            .map_err(|_| ())?;

        Ok(())
    }
}

/// Message instructing `CaptureSupervisor` to stop the specified worker
pub struct StopCaptureWorker {
    /// stop the capture actor associated with this camera id
    pub id: i32,
}

impl Message for StopCaptureWorker {
    type Result = ();
}

pub struct SyncCaptureActors {}

impl Message for SyncCaptureActors {
    type Result = ();
}

pub struct RestartCaptureActors {}

impl Message for RestartCaptureActors {
    type Result = ();
}

/// holds state of `CaptureSupervisor` actor
pub struct CaptureSupervisor {
    /// Child workers
    workers: Vec<(String, Camera, Option<Addr<CaptureActor>>)>,
    metrics: CaptureMetrics,
}

impl Actor for CaptureSupervisor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        match self
            .metrics
            .register(&prom_registry::get_metrics().registry)
        {
            Ok(()) => (),
            Err(()) => error!("Failed to register capture metrics!"),
        }

        ctx.notify(SyncCaptureActors {});
        ctx.notify_later(RestartCaptureActors {}, Duration::from_secs(2));
    }
}

impl Default for CaptureSupervisor {
    fn default() -> Self {
        Self {
            workers: Vec::new(),
            metrics: CaptureMetrics::new(),
        }
    }
}

impl Supervised for CaptureSupervisor {}

impl SystemService for CaptureSupervisor {}

impl Handler<RestartCaptureActors> for CaptureSupervisor {
    type Result = ();

    fn handle(&mut self, _msg: RestartCaptureActors, ctx: &mut Context<Self>) -> Self::Result {
        let fut = async {
            critical_section::<Self, _>(async {
                with_ctx(|actor: &mut Self, ctx: &mut Context<Self>| {
                    for (storage_path, camera, addr) in &mut actor.workers {
                        if addr.is_none() && camera.enabled {
                            info!("Restarting capture actor id: {}", camera.id);
                            let new_addr = CaptureActor::new(
                                camera.id,
                                camera.rtsp_url.clone(),
                                (*storage_path).to_string(),
                            )
                            .start();
                            *addr = Some(new_addr);
                        }

                        if let Some(addr2) = addr {
                            if !addr2.connected() && camera.enabled {
                                info!("Restarting capture actor id: {}", camera.id);
                                let new_addr = CaptureActor::new(
                                    camera.id,
                                    camera.rtsp_url.clone(),
                                    (*storage_path).to_string(),
                                )
                                .start();
                                *addr = Some(new_addr);
                                actor
                                    .metrics
                                    .restart_count
                                    .with_label_values(&[&camera.id.to_string(), &camera.name])
                                    .inc_by(1);
                            }
                        }
                    }
                    ctx.notify_later(RestartCaptureActors {}, Duration::from_secs(10));
                });
            })
            .await;
        }
        .interop_actor(self);

        ctx.spawn(fut);
    }
}
impl Handler<SyncCaptureActors> for CaptureSupervisor {
    type Result = ();

    fn handle(&mut self, _msg: SyncCaptureActors, ctx: &mut Context<Self>) -> Self::Result {
        let fut = async {
            critical_section::<Self, _>(async {
                with_ctx(|actor: &mut Self, _| {
                    for (_, camera, addr) in &actor.workers {
                        if let Some(addr) = addr {
                            addr.do_send(StopCaptureWorker { id: camera.id });
                        }
                    }
                    actor.workers.clear();
                });

                // Fetch cameras
                let groups = match db_registry::get_db()
                    .send(FetchAllCameraGroupAndCameras {})
                    .await
                {
                    Ok(Ok(c)) => c,
                    Ok(Err(_)) | Err(_) => return,
                };
                debug!("Syncing capture actors!");
                with_ctx(|actor: &mut Self, _| {
                    for g in groups {
                        for c in g.1 {
                            let storage_path = g.0.storage_path.clone();
                            actor.workers.push((storage_path, c, None));
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
