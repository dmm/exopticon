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
use actix_identity::{Identity, RequestIdentity};
use actix_service::{Service, Transform};
use actix_web::{
    dev::ServiceRequest, dev::ServiceResponse, http, web::Data, web::Json, Error, HttpResponse,
    Responder,
};
use chrono::{Duration, Utc};
use futures::future::{ok, Future, LocalBoxFuture};
use qstring::QString;
use rand::Rng;

use crate::app::RouteState;
use crate::auth_handler::AuthData;
use crate::models::{CreateUserSession, FetchUserSession};

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
#[allow(clippy::unused_async)]
pub async fn logout(id: Identity) -> HttpResponse {
    id.forget();
    HttpResponse::Ok().into()
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
