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

use actix_identity::{Identity, RequestIdentity};
use actix_service::{Service, Transform};
use actix_web::{
    dev::ServiceRequest, dev::ServiceResponse, http, web::Data, web::Json, Error, HttpResponse,
    Responder,
};
use futures::future::{ok, Future, LocalBoxFuture, Ready};
use futures::FutureExt;
use qstring::QString;

use crate::app::RouteState;
use crate::auth_handler::AuthData;

/// Route to make login attempt
pub async fn login(
    auth_data: Json<AuthData>,
    id: Identity,
    state: Data<RouteState>,
) -> impl Responder {
    let slim_user = state.db.send(auth_data.into_inner()).await;

    match slim_user {
        Ok(Ok(slim_user)) => {
            id.remember(slim_user.id.to_string());
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
pub async fn check_login(id: Identity) -> impl Responder {
    match id.identity() {
        None => HttpResponse::NotFound().finish(),
        Some(_) => HttpResponse::Ok().finish(),
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
impl<S> Transform<S, ServiceRequest> for WebAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type InitError = ();
    type Transform = WebAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(WebAuthMiddleware { service })
    }
}

/// struct representing authentication middleware for user facing routes
pub struct WebAuthMiddleware<S> {
    /// current service to check authentication for
    service: S,
}

#[allow(clippy::type_complexity)]
impl<S> Service<ServiceRequest> for WebAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    //    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        if let Some(user_id) = req.get_identity() {
            let user_id = user_id
                .parse::<i32>()
                .expect("user_id should always be a i32");
            info!("authenticated user id: {}", user_id);
            self.service.call(req).boxed_local()
        } else if req.path() == "/login" {
            self.service.call(req).boxed_local()
        } else {
            async {
                let rpath = req.path().to_string();
                let qs = QString::from(req.query_string());
                let path = qs.get("redirect_uri").unwrap_or(&rpath);
                Ok(req.into_response(
                    HttpResponse::Found()
                        .append_header((
                            http::header::LOCATION,
                            format!("/login?redirect_path={}", path),
                        ))
                        .finish(),
                ))
            }
            .boxed_local()
        }
    }
}

/// Struct implementing authentication middleware
pub struct Auth;

// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware { service })
    }
}

/// Struct implementing authentication middleware for api routes
pub struct AuthMiddleware<S> {
    /// service to check authentication for
    service: S,
}

#[allow(clippy::type_complexity)]
impl<S> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        if let Some(user_id) = req.get_identity() {
            let user_id = user_id
                .parse::<i32>()
                .expect("user_id should always be a i32");
            info!("authenticated user id: {}", user_id);
            let fut = self.service.call(req);
            Box::pin(async move {
                let result = fut.await?;

                Ok(result)
            })
        } else {
            Box::pin(async move { Ok(req.into_response(HttpResponse::Unauthorized().finish())) })
        }
    }
}
