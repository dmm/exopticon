use std::collections::HashMap;

use actix::prelude::*;

use crate::analysis_actor::AnalysisActor;
use crate::db_registry;
use crate::ws_camera_server::{FrameResolution, Subscribe, SubscriptionSubject, WsCameraServer};

/// Message telling supervisor to start new analysis actor
#[derive(Serialize, Deserialize)]
pub struct StartAnalysisActor {
    /// id of analysis actor
    pub id: i32,
    /// name of executable implementing analysis worker
    pub executable_name: String,
    /// arguments to provide analysis worker on startup
    pub arguments: Vec<String>,
    /// camera ids to
    pub subscribed_camera_ids: Vec<i32>,
}

impl Message for StartAnalysisActor {
    type Result = i32;
}

/// Message telling supervisor to stop existing analysis actor
pub struct StopAnalysisActor {
    /// id of analysis worker to stop
    pub id: i32,
}

impl Message for StopAnalysisActor {
    type Result = ();
}

/// `AnalysisSupervisor` actor
pub struct AnalysisSupervisor {
    /// supervised actors
    actors: HashMap<i32, Addr<AnalysisActor>>,
    /// tracks last actor id, only need this until we implement the database
    last_actor_id: i32,
}

impl Actor for AnalysisSupervisor {
    type Context = Context<Self>;
}

impl Default for AnalysisSupervisor {
    fn default() -> Self {
        Self {
            actors: HashMap::new(),
            last_actor_id: 1,
        }
    }
}

impl SystemService for AnalysisSupervisor {}
impl Supervised for AnalysisSupervisor {}

impl Handler<StartAnalysisActor> for AnalysisSupervisor {
    type Result = i32;

    fn handle(&mut self, msg: StartAnalysisActor, _ctx: &mut Context<Self>) -> Self::Result {
        let id = self.last_actor_id;
        self.last_actor_id += 1;
        info!("Starting analysis actor id: {}", id);
        let actor = AnalysisActor::new(
            id,
            msg.executable_name,
            msg.arguments,
            db_registry::get_db(),
        );
        let address = actor.start();
        self.actors.insert(id, address.clone());

        for camera_id in msg.subscribed_camera_ids {
            // setup camera subscriptions
            WsCameraServer::from_registry().do_send(Subscribe {
                subject: SubscriptionSubject::Camera(camera_id, FrameResolution::SD),
                client: address.clone().recipient(),
            });
        }

        id
    }
}

impl Handler<StopAnalysisActor> for AnalysisSupervisor {
    type Result = ();

    fn handle(&mut self, msg: StopAnalysisActor, _ctx: &mut Context<Self>) -> Self::Result {
        info!("Stopping analysis actor id: {}", &msg.id);
        self.actors.remove(&msg.id);
    }
}

impl AnalysisSupervisor {
    /// Returns new `AnalysisSupervisor`
    pub fn new() -> Self {
        Self {
            actors: HashMap::new(),
            last_actor_id: 1,
        }
    }
}
