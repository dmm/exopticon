use actix::*;

use crate::file_deletion_actor::FileDeletionActor;
use crate::models::DbExecutor;

pub struct StartDeletionWorker {
    pub db_addr: Addr<DbExecutor>,
    pub camera_group_id: i32,
}

impl Message for StartDeletionWorker {
    type Result = ();
}

pub struct FileDeletionSupervisor {
    workers: Vec<(i32, Addr<FileDeletionActor>)>,
}

impl Actor for FileDeletionSupervisor {
    type Context = Context<Self>;
}

impl Handler<StartDeletionWorker> for FileDeletionSupervisor {
    type Result = ();

    fn handle(&mut self, msg: StartDeletionWorker, _ctx: &mut Context<Self>) -> Self::Result {
        debug!(
            "FileDeletionSupervisor: Starting worker for camera group id: {}",
            msg.camera_group_id
        );
        let id = msg.camera_group_id;
        let address = Arbiter::start(|_| FileDeletionActor::new(msg.camera_group_id, msg.db_addr));
        self.workers.push((id, address));
    }
}

impl FileDeletionSupervisor {
    pub fn new() -> FileDeletionSupervisor {
        FileDeletionSupervisor {
            workers: Vec::new(),
        }
    }
}
