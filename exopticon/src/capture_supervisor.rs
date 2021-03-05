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

use crate::capture_actor::CaptureActor;
use crate::db_registry;
use crate::models::{Camera, FetchAllCameraGroupAndCameras};

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
}

impl Actor for CaptureSupervisor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        ctx.notify(SyncCaptureActors {});
        ctx.notify_later(RestartCaptureActors {}, Duration::from_secs(2));
    }
}

impl Default for CaptureSupervisor {
    fn default() -> Self {
        Self {
            workers: Vec::new(),
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
