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

use futures::{stream::FuturesUnordered, StreamExt};
use tokio::task::{spawn_blocking, JoinHandle};

use crate::file_deletion_actor;

pub struct FileDeletionSupervisor {
    db: crate::db::Service,
    delete_handles: FuturesUnordered<JoinHandle<()>>,
}

impl FileDeletionSupervisor {
    pub fn new(db: crate::db::Service) -> Self {
        Self {
            db,
            delete_handles: FuturesUnordered::new(),
        }
    }

    async fn start_deletors(&mut self) -> anyhow::Result<()> {
        let db = self.db.clone();
        let storage_groups: Vec<crate::api::storage_groups::StorageGroup> =
            spawn_blocking(move || db.fetch_all_storage_groups()).await??;
        // start deletion actors
        for s in storage_groups {
            debug!("Starting deletion actor for storage id {}", s.id);
            let actor = file_deletion_actor::FileDeletionActor::new(s.id, self.db.clone());

            let fut = tokio::spawn(actor.run());
            self.delete_handles.push(fut);
        }

        Ok(())
    }

    pub async fn supervise(mut self) -> anyhow::Result<()> {
        self.start_deletors().await?;
        if let Some(_storage_group_id) = self.delete_handles.next().await {
            error!("Deletion Actor died!");
            return Err(anyhow::anyhow!("Deletion Actor died!"));
        }

        Ok(())
    }
}
