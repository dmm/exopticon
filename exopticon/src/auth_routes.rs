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

// Disable this lint because we pass by value to implement the
// actix-web interface.
#![allow(clippy::needless_pass_by_value)]

use std::pin::Pin;
use std::rc::Rc;

use actix::fut::Ready;
use actix_http::body::EitherBody;
use actix_http::Payload;
use actix_identity::{Identity, RequestIdentity};
use actix_service::{Service, Transform};
use actix_web::error::ErrorUnauthorized;
use actix_web::web::Path;
use actix_web::{
    dev::ServiceRequest, dev::ServiceResponse, http, web::Data, web::Json, Error, HttpResponse,
    Responder,
};
use actix_web::{FromRequest, HttpRequest};
use chrono::{Duration, Utc};
use futures::future::{ok, Future, LocalBoxFuture};
use qstring::QString;
use rand::Rng;

use crate::app::RouteState;
use crate::auth_handler::AuthData;
use crate::models::{
    CreateUserSession, CreateUserToken, DeleteUserSession, DeleteUserToken, FetchUser,
    FetchUserSession, FetchUserTokens, SlimUser,
};

/// Route to make login attempt
pub async fn login(
    auth_data: Json<AuthData>,
    id: Identity,
    state: Data<RouteState>,
) -> impl Responder {
    let slim_user = state.db.send(auth_data.into_inner()).await;

    match slim_user {
        Ok(Ok(slim_user)) => {
            // authn successful create user session
            let session_key = base64::encode(&rand::thread_rng().gen::<[u8; 32]>());
            let valid_time = Duration::days(7);
            let expiration = match Utc::now().checked_add_signed(valid_time) {
                None => {
                    error!("Expiration time out of bounds!");
                    return HttpResponse::InternalServerError().finish();
                }
                Some(time) => time,
            };

            let _session = state
                .db
                .send(CreateUserSession {
                    name: "".to_string(),
                    user_id: slim_user.id,
                    session_key: session_key.clone(),
                    is_token: false,
                    expiration,
                })
                .await;
            //            id.remember(slim_user.id.to_string());
            id.remember(session_key);
            HttpResponse::Ok().into()
        }
        Err(_) => HttpResponse::InternalServerError().finish(),
        Ok(Err(e)) => {
            error!("Error during login: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[allow(clippy::unused_async)]
pub async fn check_login(id: Identity, state: Data<RouteState>) -> impl Responder {
    match id.identity() {
        None => HttpResponse::NotFound().finish(),
        Some(session_key) => {
            let session = state.db.send(FetchUserSession { session_key }).await;

            match session {
                Ok(Ok(_)) => HttpResponse::Ok().finish(),
                Ok(Err(_)) | Err(_) => HttpResponse::NotFound().finish(),
            }
        }
    }
}

/// Route to make logout attempt
pub async fn logout(id: Identity, state: Data<RouteState>) -> HttpResponse {
    let ret = match id.identity() {
        None => HttpResponse::NotFound().finish(),
        Some(session_key) => match state.db.send(DeleteUserSession { session_key }).await {
            Ok(Ok(_)) => HttpResponse::Ok().finish(),
            Ok(Err(_)) | Err(_) => HttpResponse::NotFound().finish(),
        },
    };

    id.forget();
    ret
}

/// Route to return personal access tokens
pub async fn fetch_personal_access_tokens(user: SlimUser, state: Data<RouteState>) -> HttpResponse {
    let db = state.db.clone();

    match db.send(FetchUserTokens { user_id: user.id }).await {
        Ok(Ok(tokens)) => HttpResponse::Ok().json(tokens),
        Ok(Err(err)) => {
            error!("Error fetching personal access tokens: {}", err);
            HttpResponse::NotFound().finish()
        }
        Err(err) => {
            error!("Error fetching personal access tokens: {}", err);
            HttpResponse::NotFound().finish()
        }
    }
}

/// Route the create personal access token
pub async fn create_personal_access_token(
    req: Json<CreateUserToken>,
    user: SlimUser,
    state: Data<RouteState>,
) -> HttpResponse {
    let db = state.db.clone();
    let session_key = base64::encode(&rand::thread_rng().gen::<[u8; 32]>());
    let create_user_session = CreateUserSession {
        name: req.name.clone(),
        user_id: user.id,
        session_key,
        is_token: true,
        expiration: req.expiration,
    };

    match db.send(create_user_session).await {
        Ok(Ok(token)) => HttpResponse::Ok().json(token.session_key),
        Ok(Err(err)) => {
            error!("Error fetching personal access tokens: {}", err);
            HttpResponse::NotFound().finish()
        }
        Err(err) => {
            error!("Error fetching personal access tokens: {}", err);
            HttpResponse::NotFound().finish()
        }
    }
}

/// Route to delete personal access token
pub async fn delete_personal_access_token(
    path: Path<i32>,
    _user: SlimUser,
    state: Data<RouteState>,
) -> HttpResponse {
    let db = state.db.clone();

    match db
        .send(DeleteUserToken {
            token_id: path.into_inner(),
        })
        .await
    {
        Ok(Ok(())) => HttpResponse::Ok().finish(),
        Ok(Err(err)) => {
            error!("Error fetching personal access tokens: {}", err);
            HttpResponse::NotFound().finish()
        }
        Err(err) => {
            error!("Error fetching personal access tokens: {}", err);
            HttpResponse::NotFound().finish()
        }
    }
}

impl FromRequest for SlimUser {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Error>>>>;

    fn from_request(req: &HttpRequest, pl: &mut Payload) -> Self::Future {
        let fut = Identity::from_request(req, pl);
        let state: &Data<RouteState> = req.app_data().unwrap();
        let db = state.db.clone();

        let header_key = match req.headers().get("PRIVATE-KEY") {
            Some(header) => match header.to_str() {
                Ok(val) => {
                    info!("Matched private-key header!");
                    Some(val.to_string())
                }
                Err(_) => None,
            },
            None => None,
        };

        Box::pin(async move {
            let cookie_key = fut.await?.identity();

            // if PRIVATE-KEY header exists, use it, otherwise try a cookie
            let session_key = match header_key {
                Some(key) => Some(key),
                None => cookie_key,
            };

            let user_id = if let Some(session_key) = session_key {
                let session = db.send(FetchUserSession { session_key }).await;

                match session {
                    Ok(Ok(session)) => {
                        let user_id = session.user_id;
                        info!("authenticated user id: {}", user_id);
                        Ok(user_id)
                    }
                    Ok(Err(_)) | Err(_) => Err(ErrorUnauthorized("unauthorized")),
                }
            } else {
                Err(ErrorUnauthorized("unauthorized"))
            }?;

            match db.send(FetchUser { user_id }).await {
                Ok(Ok(user)) => Ok(user),
                Ok(Err(_)) | Err(_) => Err(ErrorUnauthorized("unauthorized")),
            }
        })
    }
}

/// Struct implementing Authentication middleware for api
pub struct WebAuth;

// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for WebAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = WebAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(WebAuthMiddleware {
            service: Rc::new(service),
        })
    }
}

/// struct representing authentication middleware for user facing routes
pub struct WebAuthMiddleware<S> {
    /// current service to check authentication for
    service: Rc<S>,
}

#[allow(clippy::type_complexity)]
impl<S, B> Service<ServiceRequest> for WebAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    //    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();

        // pull 'identity' and 'db' out of req before we consume it
        let state: &Data<RouteState> = req.app_data().unwrap();
        let db = state.db.clone();
        let identity = req.get_identity();
        Box::pin(async move {
            // Determine if user is logged in
            let user_id = if let Some(session_key) = identity {
                let session = db.send(FetchUserSession { session_key }).await;

                match session {
                    Ok(Ok(session)) => {
                        let user_id = session.user_id;
                        info!("authenticated user id: {}", user_id);
                        Some(user_id)
                    }
                    Ok(Err(_)) | Err(_) => None,
                }
            } else {
                None
            };

            if None == user_id && req.path() != "/login" {
                let (request, _pl) = req.into_parts();
                let rpath = request.path().to_string();
                let qs = QString::from(request.query_string());
                let path = qs.get("redirect_uri").unwrap_or(&rpath);
                let response: HttpResponse<EitherBody<B>> = HttpResponse::Found()
                    .append_header((
                        http::header::LOCATION,
                        format!("/login?redirect_path={}", path),
                    ))
                    .finish()
                    .map_into_right_body();
                Ok(ServiceResponse::new(request, response))
            } else {
                // user is authenticated or request is for "/login"
                svc.call(req).await.map(ServiceResponse::map_into_left_body)
            }
        })
    }
}

/// Struct implementing authentication middleware
pub struct Auth;

// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware {
            service: Rc::new(service),
        })
    }
}

/// Struct implementing authentication middleware for api routes
pub struct AuthMiddleware<S> {
    /// service to check authentication for
    service: Rc<S>,
}

#[allow(clippy::type_complexity)]
impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        debug!("Before webauth middleware!");
        let svc = self.service.clone();

        // pull 'identity' and 'db' out of req before we consume it
        let state: &Data<RouteState> = req.app_data().unwrap();
        let db = state.db.clone();
        let identity = req.get_identity();

        // if PRIVATE-KEY header exists, use it, otherwise try a cookie
        let identity2 = match req.headers().get("PRIVATE-KEY") {
            Some(header) => match header.to_str() {
                Ok(val) => {
                    info!("Matched private-key header!");
                    Some(val.to_string())
                }
                Err(_) => identity,
            },
            None => identity,
        };
        Box::pin(async move {
            // Determine if user is logged in
            let user_id = if let Some(session_key) = identity2 {
                let session = db.send(FetchUserSession { session_key }).await;

                match session {
                    Ok(Ok(session)) => {
                        let user_id = session.user_id;
                        info!("authenticated user id: {}", user_id);
                        Some(user_id)
                    }
                    Ok(Err(_)) | Err(_) => None,
                }
            } else {
                None
            };

            if None == user_id {
                let (request, _pl) = req.into_parts();
                let response: HttpResponse<EitherBody<B>> =
                    HttpResponse::Unauthorized().finish().map_into_right_body();
                Ok(ServiceResponse::new(request, response))
            } else {
                // user is authenticated
                svc.call(req).await.map(ServiceResponse::map_into_left_body)
            }
        })
    }
}
