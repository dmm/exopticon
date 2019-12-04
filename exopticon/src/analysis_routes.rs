use actix::SystemService;
use actix_web::{web::Json, Error, HttpResponse};
use futures::future::Future;

use crate::analysis_supervisor::{AnalysisSupervisor, StartAnalysisActor};

/// Route function to create new analysis engine instance
pub fn create_analysis_engine(
    analysis_engine_request: Json<StartAnalysisActor>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    info!("Create analysis route");
    AnalysisSupervisor::from_registry()
        .send(analysis_engine_request.into_inner())
        .from_err()
        .and_then(|actor| Ok(HttpResponse::Ok().json(actor)))
}
