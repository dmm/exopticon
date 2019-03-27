use std::collections::HashMap;

use actix::prelude::*;

use crate::analysis_actor::AnalysisActor;

/// Message telling supervisor to start new analysis actor
pub struct StartAnalysisActor {
    /// id of analysis actor
    pub id: i32,
    /// name of executable implementing analysis worker
    pub executable_name: String,
    /// arguments to provide analysis worker on startup
    pub arguments: Vec<String>,
}

impl Message for StartAnalysisActor {
    type Result = ();
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
}

impl Actor for AnalysisSupervisor {
    type Context = Context<Self>;
}

impl Handler<StartAnalysisActor> for AnalysisSupervisor {
    type Result = ();

    fn handle(&mut self, msg: StartAnalysisActor, _ctx: &mut Context<Self>) -> Self::Result {
        info!("Starting analysis actor id: {}", msg.id);
        let id = msg.id.to_owned();
        let address = AnalysisActor::new(msg.id, msg.executable_name, msg.arguments).start();
        self.actors.insert(id, address);
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
        }
    }
}
