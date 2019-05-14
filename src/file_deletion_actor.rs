use std::time::Duration;

use crate::actix::prelude::*;
use actix_web::actix::fut::wrap_future;

use crate::models::{
    Camera, DbExecutor, DeleteVideoUnitFiles, FetchCameraGroupFiles, VideoFile, VideoUnit,
};

/// A video unit/video file pair with the corresponding camera
type VideoUnitPair = (Camera, (VideoUnit, VideoFile));

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
    fn handle_files(
        &self,
        (max_size, current_size, files): (i64, i64, Vec<VideoUnitPair>),
        ctx: &mut Context<Self>,
    ) {
        let max_size_bytes = max_size * 1024 * 1024;
        let mut delete_amount: i64 = current_size - max_size_bytes;
        let mut video_unit_ids = Vec::new();
        let mut video_file_ids = Vec::new();

        debug!(
            "FileDeletionActor {}: Handling {} files, max_size: {}, current_size: {}, \
             delete amount: {}",
            self.camera_group_id,
            files.len(),
            max_size_bytes,
            current_size,
            delete_amount
        );

        for (_camera, (video_unit, video_file)) in files {
            if delete_amount <= 0 {
                break;
            }

            delete_amount -= i64::from(video_file.size);
            video_unit_ids.push(video_unit.id);
            video_file_ids.push(video_file.id);
            debug!(
                "file_deletion_actor({}): removing file: {}, size: {}",
                self.camera_group_id, video_file.filename, video_file.size
            );
            match std::fs::remove_file(video_file.filename) {
                serde::export::Ok(_) | serde::export::Err(_) => {}
            };
        }

        let fut = self.db_addr.send(DeleteVideoUnitFiles {
            video_unit_ids,
            video_file_ids,
        });

        ctx.spawn(
            wrap_future(fut)
                .map(|_result, _actor, _ctx| {})
                .map_err(|_e, _actor, _ctx| {}),
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
                if let Ok(r) = result {
                    actor.handle_files(r, ctx);
                } else {
                    error!("Error fetching camera group files.");
                }
                ctx.notify_later(StartWork {}, Duration::from_millis(5000));
            })
            .map_err(|_e, actor, ctx| {
                error!(
                    "Error fetching camera group files for id: {}",
                    actor.camera_group_id
                );
                ctx.notify_later(StartWork {}, Duration::from_millis(5000));
            });
        ctx.spawn(fut);
    }
}
