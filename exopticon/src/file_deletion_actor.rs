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

use actix::fut::wrap_future;
use actix::{Actor, ActorFuture, Addr, AsyncContext, Context, Handler, Message, WrapFuture};

use crate::models::{DbExecutor, DeleteVideoUnits, FetchCameraGroupFiles, VideoUnit};

/// A video unit/video file pair with the corresponding camera
type VideoUnitPair = (VideoUnit, i64);

/// File deletion actor state
pub struct FileDeletionActor {
    /// id of camera group this actor will deletion excess files for
    camera_group_id: i32,
    /// Address of database worker
    db_addr: Addr<DbExecutor>,
}

impl Actor for FileDeletionActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        debug!(
            "FileDeletionActor: Starting for camera group: {}",
            self.camera_group_id
        );
        ctx.notify_later(StartWork {}, Duration::from_millis(100));
    }
}

impl FileDeletionActor {
    /// Returns newly initialized `FileDeletionActor`
    pub const fn new(camera_group_id: i32, db_addr: Addr<DbExecutor>) -> Self {
        Self {
            camera_group_id,
            db_addr,
        }
    }

    /// Processes a group of files and deletes excess
    #[allow(clippy::panic)]
    fn handle_files(
        &self,
        (max_size, current_size, files): (i64, i64, Vec<VideoUnitPair>),
        ctx: &mut Context<Self>,
    ) {
        let max_size_bytes = max_size * 1024 * 1024;
        let mut delete_amount: i64 = current_size - max_size_bytes;
        let mut video_unit_ids = Vec::new();

        debug!(
            "FileDeletionActor {}: Handling {} files, max_size: {}MiB, current_size: {}MiB, \
             delete amount: {}MiB",
            self.camera_group_id,
            files.len(),
            max_size_bytes / 1024 / 1024,
            current_size / 1024 / 1024,
            delete_amount / 1024 / 1024
        );

        for (video_unit, video_unit_size) in files {
            if delete_amount <= 0 {
                break;
            }
            delete_amount -= video_unit_size;
            video_unit_ids.push(video_unit.id);
        }

        let fut = self.db_addr.send(DeleteVideoUnits { video_unit_ids });
        ctx.spawn(
            wrap_future(fut).map(move |res, _actor: &mut Self, ctx: &mut Context<Self>| {
                match res {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => {
                        error!("Error deleting video units! {}", err);
                    }
                    Err(err) => {
                        error!("Error deleting video units! {}", err);
                        panic!("Failed to delete!");
                    }
                }

                ctx.notify_later(StartWork {}, Duration::from_millis(5000));
            }),
        );
    }
}

/// Message indicating `FileDeletionActor` should begin work
struct StartWork;

impl Message for StartWork {
    type Result = ();
}

impl Handler<StartWork> for FileDeletionActor {
    type Result = ();

    fn handle(&mut self, _msg: StartWork, ctx: &mut Context<Self>) -> Self::Result {
        debug!("FileDeletionActor: Starting work!");
        let fut = self
            .db_addr
            .send(FetchCameraGroupFiles {
                camera_group_id: self.camera_group_id,
                count: 100,
            })
            .into_actor(self)
            .map(|result, actor: &mut Self, ctx| {
                if let Ok(Ok(r)) = result {
                    actor.handle_files(r, ctx);
                } else {
                    error!(
                        "Error fetching camera group files for id: {}.",
                        actor.camera_group_id
                    );
                }
            });

        ctx.spawn(fut);
    }
}
