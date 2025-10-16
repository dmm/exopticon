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

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    middleware::Next,
    routing::get,
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use base64::prelude::{BASE64_STANDARD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use tokio::task::spawn_blocking;
use uuid::Uuid;

use crate::AppState;

use super::UserError;

const SESSION_COOKIE: &str = "id";

/// Represents data for an authentication attempt
#[derive(Debug, Deserialize)]
pub struct Data {
    /// username
    pub username: String,
    /// plaintext password
    pub password: String,
}

/// User model without password. This is used as a return value for
/// user operations.
#[derive(Clone, Serialize)]
pub struct User {
    /// User id
    pub id: Uuid,
    /// username
    pub username: String,
}

/// Request to create new user session
#[derive(Clone, Deserialize)]
pub struct CreateUserSession {
    /// user session name
    pub name: String,
    /// id of user associated with session
    pub user_id: Uuid,
    /// session key value
    pub session_key: String,
    /// flag indicating where it is an api token or user session
    pub is_token: bool,
    /// Expiration timestamp
    pub expiration: DateTime<Utc>,
}

// Request to create personal access token
#[derive(Clone, Deserialize)]
pub struct CreatePersonalAccessToken {
    /// token name
    pub name: String,
    /// Expiration timestamp
    pub expiration: DateTime<Utc>,
}

/// Access Token model to return to user
#[derive(Debug, Serialize)]
pub struct SlimAccessToken {
    /// user session id
    pub id: Uuid,
    /// user session name
    pub name: String,
    /// id of user associated with session
    pub user_id: Uuid,
    /// Expiration timestamp
    pub expiration: DateTime<Utc>,
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(auth_data): Json<Data>,
) -> Result<CookieJar, UserError> {
    let db = state.db_service.clone();
    let db2 = state.db_service;

    let user = spawn_blocking(move || db.login(&auth_data.username, &auth_data.password)).await??;
    error!("Auth success!");
    // We found a valid user with that password. Create a login session.
    let session_key = BASE64_STANDARD.encode(rand::thread_rng().r#gen::<[u8; 32]>());
    let valid_time = Duration::days(7);
    let Some(expiration) = Utc::now().checked_add_signed(valid_time) else {
        error!("expiration date calculation failed!");
        return Err(UserError::InternalError(
            "expiration date calculation failed!".to_string(),
        ));
    };

    let session = CreateUserSession {
        name: String::new(),
        user_id: user.id,
        session_key,
        is_token: false,
        expiration,
    };
    error!("creating session!");
    let session_token = spawn_blocking(move || db2.create_user_session(&session)).await??;

    let cookie_builder = Cookie::build((SESSION_COOKIE, session_token))
        .path("/")
        .secure(true)
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Strict)
        .max_age(time::Duration::days(7));

    let jar = jar.add(cookie_builder);

    Ok(jar)
}

pub async fn create_personal_access_token(
    Extension(current_user): Extension<User>,
    State(state): State<AppState>,

    Json(create_token_request): Json<CreatePersonalAccessToken>,
) -> Result<Json<String>, UserError> {
    let db = state.db_service;

    let session_key = BASE64_STANDARD.encode(rand::thread_rng().r#gen::<[u8; 32]>());

    let new_session = CreateUserSession {
        name: create_token_request.name,
        user_id: current_user.id,
        session_key,
        is_token: true,
        expiration: create_token_request.expiration,
    };

    let token = spawn_blocking(move || db.create_user_session(&new_session)).await??;

    Ok(Json(token))
}

pub async fn delete_personal_access_token(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<(), UserError> {
    let db = state.db_service;

    spawn_blocking(move || db.delete_user_session(id)).await??;
    Ok(())
}

pub async fn fetch_personal_access_tokens(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<SlimAccessToken>>, UserError> {
    let db = state.db_service;

    let tokens = spawn_blocking(move || db.fetch_users_tokens(user.id)).await??;

    Ok(Json(tokens))
}

pub async fn middleware(
    State(state): State<AppState>,
    // you can add more extractors here but the last
    // extractor must implement `FromRequest` which
    // `Request` does
    mut request: axum::extract::Request,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    let jar = CookieJar::from_headers(request.headers());
    let session_cookie = jar.get("id");
    let session_key = session_cookie.map(|x| String::from(x.value()));

    if let Some(session_key) = session_key {
        let db = state.db_service;
        if let Ok(Ok(user)) = spawn_blocking(move || db.validate_user_session(&session_key)).await {
            request.extensions_mut().insert(user);
            let response = next.run(request).await;
            return Ok(response);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

pub fn personal_access_token_router() -> Router<AppState> {
    Router::<AppState>::new()
        .route(
            "/",
            get(fetch_personal_access_tokens).post(create_personal_access_token),
        )
        .route("/:id", axum::routing::delete(delete_personal_access_token))
}
