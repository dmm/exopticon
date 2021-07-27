/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

use actix_web::{web::Data, web::Json, HttpResponse};

use crate::app::RouteState;
use crate::errors::ServiceError;
use crate::models::{CreateAlertRule, FetchAllAlertRule};

/// Route to create `AlertRule`
pub async fn create_alert_rule(
    alert_rule_request: Json<CreateAlertRule>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    let alert_rule = state.db.send(alert_rule_request.into_inner()).await??;

    Ok(HttpResponse::Ok().json(alert_rule))
}

/// Route to fetch all `AlertRules`
pub async fn fetch_all_alert_rules(state: Data<RouteState>) -> Result<HttpResponse, ServiceError> {
    let alert_rules = state.db.send(FetchAllAlertRule {}).await??;

    Ok(HttpResponse::Ok().json(alert_rules))
}
