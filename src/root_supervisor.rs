use actix::*;

use actix_web::actix::fut::wrap_future;
use capture_supervisor::{CaptureSupervisor, StartCaptureWorker};
use models::{DbExecutor, FetchAllCameraGroupAndCameras};

pub enum ExopticonMode {
    Standby,
    Run,
}

pub struct RootSupervisor {
    pub capture_supervisor: Addr<CaptureSupervisor>,
    pub db_worker: Addr<DbExecutor>,
    pub mode: ExopticonMode,
}

fn start_cameras(_db_worker: &Addr<DbExecutor>) {}

impl Actor for RootSupervisor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let fetch = self.db_worker.send(FetchAllCameraGroupAndCameras {});

        let fut = wrap_future(fetch)
            .map(
                |group_result, actor: &mut RootSupervisor, _ctx| match group_result {
                    Ok(groups) => {
                        for g in groups {
                            for c in g.1 {
                                actor.capture_supervisor.do_send(StartCaptureWorker {
                                    db_addr: actor.db_worker.clone(),
                                    id: c.id,
                                    stream_url: c.rtsp_url,
                                    storage_path: g.0.storage_path.clone(),
                                });
                            }
                        }
                    }
                    Err(e) => panic!("{}", e),
                },
            ).map_err(|_e, _actor, _ctx| {});

        ctx.wait(fut);
    }
}

impl RootSupervisor {
    pub fn new(start_mode: ExopticonMode, db_worker: Addr<DbExecutor>) -> RootSupervisor {
        let capture_supervisor = CaptureSupervisor::new().start();

        match start_mode {
            ExopticonMode::Standby => {}
            ExopticonMode::Run => {
                start_cameras(&db_worker);
            }
        };

        RootSupervisor {
            capture_supervisor: capture_supervisor,
            db_worker: db_worker,
            mode: start_mode,
        }
    }
}
