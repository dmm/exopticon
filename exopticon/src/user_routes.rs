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

use actix_identity::Identity;
use actix_web::{web::Data, web::Json, Error, HttpResponse};

use crate::app::RouteState;
use crate::errors::ServiceError;
use crate::models::{CreateUser, FetchUser, FetchUserSession};

/// We have to pass by value to satisfy the actix route interface.
#[allow(clippy::needless_pass_by_value)]
/// Implements route to create user, returns future returning created
/// user or error.
///
/// # Arguments
/// `create_user` - `Json` representation of `CreateUser` struct
/// `state` - `RouteState` struct
///
pub async fn create_user(
    create_user: Json<CreateUser>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let db_response = state
        .db
        .send(CreateUser {
            username: create_user.username.clone(),
            password: create_user.password.clone(),
            timezone: create_user.timezone.clone(),
        })
        .await
        .map_err(|_| ServiceError::InternalServerError)?;

    match db_response {
        Ok(slim_user) => Ok(HttpResponse::Ok().json(slim_user)),
        Err(err) => Ok(HttpResponse::InternalServerError().json(err.to_string())),
    }
}

/// Implements route to fetch current user, returns future returning
/// current user or error.
pub async fn fetch_current_user(
    id: Identity,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let user_id = match id.identity() {
        None => return Ok(HttpResponse::NotFound().finish()),
        Some(session_key) => {
            let session = state.db.send(FetchUserSession { session_key }).await;

            match session {
                Ok(Ok(user_session)) => user_session.user_id,
                Ok(Err(_)) | Err(_) => {
                    error!("Failed to fetch user session");
                    return Ok(HttpResponse::NotFound().finish());
                }
            }
        }
    };

    let db_response = state.db.send(FetchUser { user_id }).await.map_err(|err| {
        error!("Failed to fetch user: {}", err);
        ServiceError::InternalServerError
    })?;

    match db_response {
        Ok(slim_user) => Ok(HttpResponse::Ok().json(slim_user)),
        Err(err) => {
            error!("Failed to fetch user: {}", err);
            Ok(HttpResponse::InternalServerError().json(err.to_string()))
        }
    }
}
