/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2023-2026 David Matthew Mattli <dmm@mattli.us>
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

use std::{
    collections::HashSet,
    time::{Duration, SystemTime},
};

use futures::{StreamExt, stream::FuturesUnordered};
use tokio::task::spawn_blocking;

use crate::{
    file_deletion_actor::FileDeletionActor,
    metrics::{StorageGroupUsageRegistry, record_retention_error, record_retention_run_at},
};

pub struct FileDeletionSupervisor {
    db: crate::db::Service,
    storage_usage: StorageGroupUsageRegistry,
}

impl FileDeletionSupervisor {
    pub const fn new(db: crate::db::Service, storage_usage: StorageGroupUsageRegistry) -> Self {
        Self { db, storage_usage }
    }

    pub async fn run_retention_pass(&self) {
        let db = self.db.clone();
        match spawn_blocking(move || db.fetch_all_storage_groups()).await {
            Ok(Ok(groups)) => {
                let active_names: HashSet<String> =
                    groups.iter().map(|g| g.metadata.name.clone()).collect();
                let mut passes = FuturesUnordered::new();
                for group in groups {
                    let name = group.metadata.name;
                    let actor = FileDeletionActor::new(name.clone(), self.db.clone());
                    passes.push(async move { (name, actor.work().await) });
                }
                let mut updates = Vec::new();
                while let Some((name, result)) = passes.next().await {
                    match result {
                        Ok(usage) => updates.push((name, usage)),
                        Err(err) => {
                            record_retention_error();
                            error!("Retention pass failed for {name}: {err}");
                        }
                    }
                }
                {
                    let mut usage_map = self.storage_usage.write().await;
                    usage_map.retain(|name, _| active_names.contains(name));
                    for (name, usage) in updates {
                        usage_map.insert(name, usage);
                    }
                }
                record_retention_run_at(SystemTime::now());
            }
            Ok(Err(err)) => {
                record_retention_error();
                error!("Could not refresh retention groups: {err}");
            }
            Err(err) => {
                record_retention_error();
                error!("Retention group refresh task failed: {err}");
            }
        }
    }

    pub async fn supervise(self) {
        loop {
            self.run_retention_pass().await;
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    pub async fn purge_cameras(db: crate::db::Service) {
        loop {
            let db_clone = db.clone();
            match spawn_blocking(move || db_clone.purge_deleted_cameras()).await {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    record_retention_error();
                    error!("Could not purge cameras: {err}");
                }
                Err(err) => {
                    record_retention_error();
                    error!("Camera purge task failed: {err}");
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::metrics::{StorageGroupUsage, new_storage_group_usage_registry};

    #[test]
    fn retention_pass_updates_storage_group_usage_and_drops_deleted_groups() {
        let (_dir, db) = crate::db::tests::configured_service();
        let usage = new_storage_group_usage_registry();
        let recorder = crate::metrics::build_recorder();
        let handle = recorder.handle();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            usage.write().await.insert(
                "stale-group".to_string(),
                StorageGroupUsage {
                    used_bytes: 10,
                    max_bytes: 20,
                },
            );
        });

        metrics::with_local_recorder(&recorder, || {
            runtime.block_on(async {
                let supervisor = FileDeletionSupervisor::new(db, Arc::clone(&usage));
                supervisor.run_retention_pass().await;
            });
        });

        let snapshot = runtime.block_on(async { usage.read().await.clone() });
        assert!(!snapshot.contains_key("stale-group"));
        assert_eq!(
            snapshot.get("primary"),
            Some(&StorageGroupUsage {
                used_bytes: 0,
                max_bytes: 1024 * 1024 * 1024,
            })
        );
        assert!(
            handle
                .render()
                .contains("exopticon_retention_last_run_timestamp_seconds ")
        );
    }
}
