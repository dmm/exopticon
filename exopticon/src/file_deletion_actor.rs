/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2023 David Matthew Mattli <dmm@mattli.us>
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

use tokio::task::spawn_blocking;

use crate::db::storage_groups::StorageGroupOldFiles;

pub struct FileDeletionActor {
    storage_group_id: i32,
    db: crate::db::Service,
}

impl FileDeletionActor {
    pub const fn new(storage_group_id: i32, db: crate::db::Service) -> Self {
        Self {
            storage_group_id,
            db,
        }
    }

    async fn handle_files(&self, files: StorageGroupOldFiles) -> anyhow::Result<()> {
        let max_size_bytes = files.storage_group_capacity * 1024 * 1024;
        let mut delete_amount: i64 = files.storage_group_size - max_size_bytes;
        let mut video_unit_ids = Vec::new();

        debug!(
            "FileDeletionActor {}: Handling {} files, max_size: {}MiB, current_size: {}MiB, \
             delete amount: {}MiB",
            self.storage_group_id,
            files.video_units.len(),
            max_size_bytes / 1024 / 1024,
            files.storage_group_size / 1024 / 1024,
            delete_amount / 1024 / 1024
        );

        for (video_unit_size, video_unit, video_file) in files.video_units {
            if delete_amount <= 0 {
                break;
            }
            delete_amount -= video_unit_size;
            video_unit_ids.push((video_unit.id, video_file.filename));
        }

        for (video_unit_id, filename) in video_unit_ids {
            // delete video unit
            let vu_id = video_unit_id;
            let db = self.db.clone();
            debug!("Deleting {} {}", video_unit_id, filename);
            if let Err(e) = spawn_blocking(move || db.delete_video_unit(vu_id)).await? {
                error!(
                    "error deleting VideoUnit {}, filename {}: {}",
                    &video_unit_id, &filename, e
                );
            }
        }

        Ok(())
    }

    async fn work(&mut self) -> anyhow::Result<()> {
        let db = self.db.clone();
        let storage_group_id = self.storage_group_id;
        let files = spawn_blocking(move || db.fetch_storage_group_old_units(storage_group_id, 100))
            .await??;
        self.handle_files(files).await?;

        Ok(())
    }

    pub async fn run(mut self) {
        loop {
            if let Err(e) = self.work().await {
                error!("Error deleting files! {}", e);
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}
