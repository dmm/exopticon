use actix::prelude::*;
/// Analysis Actor context
pub struct AnalysisActor {
    /// actor id
    pub id: i32,
    /// executable to run
    pub executable_name: String,
    /// arguments to provide to executable
    pub arguments: Vec<String>,
}

impl AnalysisActor {
    /// Returns initialized `AnalysisActor`
    pub fn new(id: i32, executable_name: String, arguments: Vec<String>) -> Self {
        Self {
            id,
            executable_name,
            arguments,
        }
    }
}

impl Actor for AnalysisActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        debug!("Analysis actor starting!");
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("Analysis actor stopping!");
    }
}
