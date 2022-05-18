/*
 * Exopticon - A free video surveillance system.
p * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
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

use actix::*;

use crate::file_deletion_actor::FileDeletionActor;
use crate::models::DbExecutor;

/// Request supervisor to start file deletion actor
pub struct StartDeletionWorker {
    /// address of database actor
    pub db_addr: Addr<DbExecutor>,
    /// id of storage group actor is to work for
    pub storage_group_id: i32,
}

impl Message for StartDeletionWorker {
    type Result = ();
}

/// File Deletion Supervisor actor state
pub struct FileDeletionSupervisor {
    /// tuple of storage group id and address of file deletion actor
    workers: Vec<(i32, Addr<FileDeletionActor>)>,
}

impl Actor for FileDeletionSupervisor {
    type Context = Context<Self>;
}

impl Handler<StartDeletionWorker> for FileDeletionSupervisor {
    type Result = ();

    fn handle(&mut self, msg: StartDeletionWorker, _ctx: &mut Context<Self>) -> Self::Result {
        debug!(
            "FileDeletionSupervisor: Starting worker for storage group id: {}",
            msg.storage_group_id
        );
        let id = msg.storage_group_id;
        let address = FileDeletionActor::new(msg.storage_group_id, msg.db_addr).start();
        self.workers.push((id, address));
    }
}

impl FileDeletionSupervisor {
    /// Create new file deletion supervisor

    pub const fn new() -> Self {
        Self {
            workers: Vec::new(),
        }
    }
}
