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

use actix::{registry::SystemService, Actor, ActorFuture, Addr, AsyncContext, Context, WrapFuture};

use crate::alert_actor::AlertActor;
use crate::analysis_supervisor::AnalysisSupervisor;
use crate::capture_supervisor::CaptureSupervisor;
use crate::file_deletion_supervisor::{FileDeletionSupervisor, StartDeletionWorker};
use crate::models::{CameraGroup, DbExecutor, FetchAllCameraGroup};
use crate::notifier_supervisor::NotifierSupervisor;

/// Enumeration of Exopticon run modes
pub enum ExopticonMode {
    /// System should not run capture and deletion workers
    Standby,
    /// System should run normally
    Run,
}

/// struct representing `RootSupervisor` actor. Contains state of
/// non-web application.
pub struct RootSupervisor {
    /// Supervisor for analysis actors
    pub analysis_supervisor: Addr<AnalysisSupervisor>,
    /// Supervisor for capture actors
    pub capture_supervisor: Addr<CaptureSupervisor>,
    /// Supervisor for deletion actors
    pub deletion_supervisor: Addr<FileDeletionSupervisor>,
    /// Notifier Supervisor
    pub notifier_supervisor: Addr<NotifierSupervisor>,
    /// Alert Worker
    pub alert_actor: Addr<AlertActor>,
    /// Database actor
    pub db_worker: Addr<DbExecutor>,
    /// exopticon runtime mode
    pub mode: ExopticonMode,
}

impl Actor for RootSupervisor {
    type Context = Context<Self>;

    /// Starts child works if mode is `Run`
    fn started(&mut self, ctx: &mut Self::Context) {
        debug!("Starting root supervisor!");
        match self.mode {
            ExopticonMode::Standby => {}
            ExopticonMode::Run => {
                debug!("Run mode!");
                self.start_workers(ctx);
            }
        };
    }
}

impl RootSupervisor {
    /// Starts all child workers for this supervisor
    fn start_workers(&self, ctx: &mut Context<Self>) {
        debug!("starting workers!");
        let fut = self
            .db_worker
            .send(FetchAllCameraGroup {})
            .into_actor(self)
            .map(|res, act, _ctx| {
                if let Ok(Ok(r)) = res {
                    act.start_deletion_workers(r);
                }
            });

        ctx.spawn(fut);
    }

    /// Starts deletion workers based on the `CameraGroup`s provided.
    fn start_deletion_workers(&self, camera_groups: Vec<CameraGroup>) {
        for c in camera_groups {
            self.deletion_supervisor.do_send(StartDeletionWorker {
                db_addr: self.db_worker.clone(),
                camera_group_id: c.id,
            });
        }
    }

    /// Returns new `RootSupervisor` with initialized with the arguments provided.
    ///
    /// # Arguments
    ///
    /// * `start_mode` - `StandBy` or `Run`
    /// * `db_worker` - `Addr` of `DbExecutor`
    ///
    pub fn new(start_mode: ExopticonMode, db_worker: Addr<DbExecutor>) -> Self {
        let analysis_supervisor = AnalysisSupervisor::new().start();
        let deletion_supervisor = FileDeletionSupervisor::new().start();

        Self {
            analysis_supervisor,
            capture_supervisor: CaptureSupervisor::from_registry(),
            deletion_supervisor,
            notifier_supervisor: NotifierSupervisor::from_registry(),
            alert_actor: AlertActor::from_registry(),
            db_worker,
            mode: start_mode,
        }
    }
}
