use actix::*;

use crate::capture_supervisor::{CaptureSupervisor, StartCaptureWorker};
use crate::file_deletion_supervisor::{FileDeletionSupervisor, StartDeletionWorker};
use crate::models::{
    CameraGroup, CameraGroupAndCameras, DbExecutor, FetchAllCameraGroup,
    FetchAllCameraGroupAndCameras,
};
use actix_web::actix::fut;

pub enum ExopticonMode {
    Standby,
    Run,
}

pub struct RootSupervisor {
    pub capture_supervisor: Addr<CaptureSupervisor>,
    pub deletion_supervisor: Addr<FileDeletionSupervisor>,
    pub db_worker: Addr<DbExecutor>,
    pub mode: ExopticonMode,
}

impl Actor for RootSupervisor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        match self.mode {
            ExopticonMode::Standby => {}
            ExopticonMode::Run => {
                self.start_workers(ctx);
            }
        };
    }
}

pub struct StartFileDeletionWorkers;

impl Message for StartFileDeletionWorkers {
    type Result = ();
}

impl Handler<StartFileDeletionWorkers> for RootSupervisor {
    type Result = ();
    fn handle(&mut self, _msg: StartFileDeletionWorkers, _ctx: &mut Context<Self>) -> Self::Result {
    }
}

impl RootSupervisor {
    fn start_workers(&self, ctx: &mut Context<Self>) {
        let capture_future = self
            .db_worker
            .send(FetchAllCameraGroupAndCameras {})
            .into_actor(self)
            .then(|res, act, _ctx| {
                match res {
                    Ok(Ok(res)) => act.start_capture_workers(res),
                    _ => (),
                }
                fut::ok(())
            });

        ctx.spawn(capture_future);

        let fut = self
            .db_worker
            .send(FetchAllCameraGroup {})
            .into_actor(self)
            .then(|res, act, _ctx| {
                match res {
                    Ok(Ok(res)) => act.start_deletion_workers(res),
                    _ => (),
                }
                fut::ok(())
            });
        ctx.spawn(fut);
    }
    fn start_capture_workers(&self, cameras: Vec<CameraGroupAndCameras>) {
        for g in cameras {
            for c in g.1 {
                if c.enabled {
                    self.capture_supervisor.do_send(StartCaptureWorker {
                        db_addr: self.db_worker.clone(),
                        id: c.id,
                        stream_url: c.rtsp_url,
                        storage_path: g.0.storage_path.clone(),
                    });
                }
            }
        }
    }

    fn start_deletion_workers(&self, camera_groups: Vec<CameraGroup>) {
        for c in camera_groups {
            self.deletion_supervisor.do_send(StartDeletionWorker {
                db_addr: self.db_worker.clone(),
                camera_group_id: c.id,
            });
        }
    }

    pub fn new(start_mode: ExopticonMode, db_worker: Addr<DbExecutor>) -> RootSupervisor {
        let capture_supervisor = CaptureSupervisor::new().start();
        let deletion_supervisor = FileDeletionSupervisor::new().start();

        RootSupervisor {
            capture_supervisor: capture_supervisor,
            deletion_supervisor: deletion_supervisor,
            db_worker: db_worker,
            mode: start_mode,
        }
    }
}
