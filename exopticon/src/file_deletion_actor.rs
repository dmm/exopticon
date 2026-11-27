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

use tokio::task::spawn_blocking;

use crate::{
    db::storage_groups::StorageGroupOldFiles,
    metrics::{StorageGroupUsage, record_retention_error},
};

pub struct FileDeletionActor {
    storage_group_name: String,
    db: crate::db::Service,
}

impl FileDeletionActor {
    pub const fn new(storage_group_name: String, db: crate::db::Service) -> Self {
        Self {
            storage_group_name,
            db,
        }
    }

    async fn handle_files(&self, files: StorageGroupOldFiles) -> anyhow::Result<StorageGroupUsage> {
        let max_size_bytes = files.storage_group_capacity.saturating_mul(1024 * 1024);
        let mut delete_amount: i64 = files.storage_group_size.saturating_sub(max_size_bytes);
        let mut files_to_delete = Vec::new();

        debug!(
            "FileDeletionActor {}: Handling {} files, max_size: {}MiB, current_size: {}MiB, \
             delete amount: {}MiB",
            self.storage_group_name,
            files.video_units.len(),
            max_size_bytes / 1024 / 1024,
            files.storage_group_size / 1024 / 1024,
            delete_amount / 1024 / 1024
        );

        for (video_unit_size, _, video_file) in files.video_units {
            if delete_amount <= 0 {
                break;
            }
            delete_amount -= video_unit_size;
            files_to_delete.push((video_unit_size, video_file));
        }

        let mut deleted_bytes: i64 = 0;
        for (unit_size, file) in files_to_delete {
            let file_id = file.id;
            let db = self.db.clone();
            let group = self.storage_group_name.clone();
            debug!("Deleting recording file {} {}", file_id, file.filename);
            match spawn_blocking(move || db.delete_video_unit_for_file(file_id, &group)).await? {
                Ok(()) => {
                    deleted_bytes = deleted_bytes.saturating_add(unit_size);
                }
                Err(err) => {
                    record_retention_error();
                    error!(
                        "Error deleting recording file {} {}: {}",
                        file_id, file.filename, err
                    );
                }
            }
        }

        let remaining_bytes = files.storage_group_size.saturating_sub(deleted_bytes);
        Ok(StorageGroupUsage {
            used_bytes: u64::try_from(remaining_bytes).unwrap_or(0),
            max_bytes: u64::try_from(max_size_bytes).unwrap_or(0),
        })
    }

    pub async fn work(&self) -> anyhow::Result<StorageGroupUsage> {
        let db = self.db.clone();
        let storage_group_name = self.storage_group_name.clone();
        let files =
            spawn_blocking(move || db.fetch_storage_group_old_units(&storage_group_name, 100))
                .await??;
        self.handle_files(files).await
    }
}
