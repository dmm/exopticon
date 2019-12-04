use actix::*;

use crate::capture_actor::CaptureActor;
use crate::models::DbExecutor;

/// Message instructing `CaptureSupervisor` to start a capture actor
pub struct StartCaptureWorker {
    /// id of camera to start capture worker for
    pub id: i32,
    /// rtsp url of video stream to capture
    pub stream_url: String,
    /// full path to capture video to
    pub storage_path: String,
    /// database actor address
    pub db_addr: Addr<DbExecutor>,
}

impl Message for StartCaptureWorker {
    type Result = ();
}

/// Message instructing `CaptureSupervisor` to stop the specified worker
pub struct StopCaptureWorker {
    /// stop the capture actor associated with this camera id
    pub id: i32,
}

impl Message for StopCaptureWorker {
    type Result = ();
}

/// holds state of `CaptureSupervisor` actor
pub struct CaptureSupervisor {
    /// Child workers
    workers: Vec<(i32, Addr<CaptureActor>)>,
}

impl Actor for CaptureSupervisor {
    type Context = Context<Self>;
}

impl Handler<StartCaptureWorker> for CaptureSupervisor {
    type Result = ();

    fn handle(&mut self, msg: StartCaptureWorker, _ctx: &mut Context<Self>) -> Self::Result {
        info!("Supervisor: Starting camera id: {}", msg.id);
        let id = msg.id.to_owned();
        let address =
            CaptureActor::new(msg.db_addr, msg.id, msg.stream_url, msg.storage_path).start();
        self.workers.push((id, address));
    }
}

impl Handler<StopCaptureWorker> for CaptureSupervisor {
    type Result = ();

    fn handle(&mut self, msg: StopCaptureWorker, _ctx: &mut Context<Self>) -> Self::Result {
        info!("Stopping camera id: {}", msg.id);
        let camera_address = self.workers.iter().find(|(id, _)| *id == msg.id);
        if camera_address.is_some() {
            info!("Found Camera!");
        }
    }
}

impl CaptureSupervisor {
    /// Returns new initialized `CaptureSupervisor` struct, ready to be run
    pub fn new() -> Self {
        #![allow(clippy::missing_const_for_fn)] // Vec::new not allowed in const_fn
        Self {
            workers: Vec::new(),
        }
    }
}
