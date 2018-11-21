use actix::*;

use capture_actor::CaptureActor;
use models::DbExecutor;

pub struct StartCaptureWorker {
    pub id: i32,
    pub stream_url: String,
    pub storage_path: String,
    pub db_addr: Addr<DbExecutor>,
}

impl Message for StartCaptureWorker {
    type Result = ();
}

pub struct StopCaptureWorker {
    pub id: i32,
}

impl Message for StopCaptureWorker {
    type Result = ();
}

pub struct CaptureSupervisor {
    workers: Vec<(i32, Addr<CaptureActor>)>,
}

impl Actor for CaptureSupervisor {
    type Context = Context<Self>;
}

impl Handler<StartCaptureWorker> for CaptureSupervisor {
    type Result = ();

    fn handle(&mut self, msg: StartCaptureWorker, _ctx: &mut Context<Self>) -> Self::Result {
        println!("Supervisor: Starting camera id: {}", msg.id);
        let id = msg.id.to_owned();
        // Launch each CaptureActor in a new thread. This is really
        // only necessary for debug builds because they are otherwise too slow on one thread.
        let address = Arbiter::start(|_| {
            CaptureActor::new(msg.db_addr, msg.id, msg.stream_url, msg.storage_path)
        });
        self.workers.push((id, address));
    }
}

impl Handler<StopCaptureWorker> for CaptureSupervisor {
    type Result = ();

    fn handle(&mut self, msg: StopCaptureWorker, _ctx: &mut Context<Self>) -> Self::Result {
        println!("Supervisor: Stopping camera id: {}", msg.id);
        let camera_address = self.workers.iter().find(|(id, _)| *id == msg.id);
        match camera_address {
            Some(_a) => {
                println!("Found camera!");
            }
            None => {}
        };
    }
}

impl CaptureSupervisor {
    pub fn new() -> CaptureSupervisor {
        CaptureSupervisor {
            workers: Vec::new(),
        }
    }
}
