use actix_web::{web::Data, web::Json, Error, HttpResponse};

use crate::app::RouteState;
use crate::models::{CreateAlertRule, FetchAllAlertRule};

/// Route to create `AlertRule`
pub async fn create_alert_rule(
    alert_rule_request: Json<CreateAlertRule>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let alert_rule = state.db.send(alert_rule_request.into_inner()).await??;

    Ok(HttpResponse::Ok().json(alert_rule))
}

/// Route to fetch all `AlertRules`
pub async fn fetch_all_alert_rules(state: Data<RouteState>) -> Result<HttpResponse, Error> {
    let alert_rules = state.db.send(FetchAllAlertRule {}).await??;

    Ok(HttpResponse::Ok().json(alert_rules))
}
