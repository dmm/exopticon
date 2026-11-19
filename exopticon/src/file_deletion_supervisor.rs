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

use futures::{StreamExt, stream::FuturesUnordered};
use tokio::task::spawn_blocking;

use crate::file_deletion_actor::FileDeletionActor;

pub struct FileDeletionSupervisor {
    db: crate::db::Service,
}

impl FileDeletionSupervisor {
    pub const fn new(db: crate::db::Service) -> Self {
        Self { db }
    }

    pub async fn supervise(self) {
        loop {
            let db = self.db.clone();
            match spawn_blocking(move || db.fetch_all_storage_groups()).await {
                Ok(Ok(groups)) => {
                    let mut passes = FuturesUnordered::new();
                    for group in groups {
                        let actor = FileDeletionActor::new(group.metadata.name, self.db.clone());
                        passes.push(async move { actor.work().await });
                    }
                    while let Some(result) = passes.next().await {
                        if let Err(err) = result {
                            error!("Retention pass failed: {err}");
                        }
                    }
                }
                Ok(Err(err)) => error!("Could not refresh retention groups: {err}"),
                Err(err) => error!("Retention group refresh task failed: {err}"),
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    pub async fn purge_cameras(self) {
        loop {
            let db = self.db.clone();
            match spawn_blocking(move || db.purge_deleted_cameras()).await {
                Ok(Ok(())) => {}
                Ok(Err(err)) => error!("Could not purge cameras: {err}"),
                Err(err) => error!("Camera purge task failed: {err}"),
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}
