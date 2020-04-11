use actix::SystemService;
use actix_web::{web::Json, Error, HttpResponse};

use crate::analysis_supervisor::{AnalysisSupervisor, StartAnalysisActor};

/// Route function to create new analysis engine instance
pub async fn create_analysis_engine(
    analysis_engine_request: Json<StartAnalysisActor>,
) -> Result<HttpResponse, Error> {
    info!("Create analysis route");
    let res = AnalysisSupervisor::from_registry()
        .send(analysis_engine_request.into_inner())
        .await;

    match res {
        Ok(actor) => Ok(HttpResponse::Ok().json(actor)),
        Err(_) => Ok(HttpResponse::InternalServerError().finish()),
    }
}
