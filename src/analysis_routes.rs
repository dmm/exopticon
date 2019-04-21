use actix::SystemService;
use actix_web::{AsyncResponder, FutureResponse, HttpResponse, Json, State};
use futures::future::Future;

use crate::analysis_supervisor::{AnalysisSupervisor, StartAnalysisActor};
use crate::app::RouteState;

/// Route function to create new analysis engine instance
pub fn create_analysis_engine(
    (analysis_engine_request, _state): (Json<StartAnalysisActor>, State<RouteState>),
) -> FutureResponse<HttpResponse> {
    info!("Create analysis route");
    AnalysisSupervisor::from_registry()
        .send(analysis_engine_request.into_inner())
        .from_err()
        .and_then(|actor| Ok(HttpResponse::Ok().json(actor)))
        .responder()
}
