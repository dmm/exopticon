/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2024 David Matthew Mattli <dmm@mattli.us>
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
use std::env;

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use axum_extra::TypedHeader;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Basic;

pub async fn metrics_auth_middleware(
    TypedHeader(auth_header): TypedHeader<Authorization<Basic>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if env::var("EXOPTICON_METRICS_ENABLED") != Ok("true".to_string()) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let Ok(username) = env::var("EXOPTICON_METRICS_USERNAME") else {
        error!("Metrics enabled but no username set");
        return Err(StatusCode::UNAUTHORIZED);
    };

    let Ok(password) = env::var("EXOPTICON_METRICS_PASSWORD") else {
        error!("Metrics enabled but no password set");
        return Err(StatusCode::UNAUTHORIZED);
    };

    if auth_header.username() == username && auth_header.password() == password {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
