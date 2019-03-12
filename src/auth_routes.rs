// Disable this lint because we pass by value to implement the
// actix-web interface.
#![allow(clippy::needless_pass_by_value)]
use actix_web::http;
use actix_web::middleware::identity::RequestIdentity;
use actix_web::middleware::{Middleware, Started};
use actix_web::{
    AsyncResponder, FutureResponse, HttpRequest, HttpResponse, Json, ResponseError, Result,
};
use futures::future::Future;

use crate::app::RouteState;
use crate::auth_handler::AuthData;

/// Route to make login attempt
pub fn login(
    (auth_data, req): (Json<AuthData>, HttpRequest<RouteState>),
) -> FutureResponse<HttpResponse> {
    req.state()
        .db
        .send(auth_data.into_inner())
        .from_err()
        .and_then(move |res| match res {
            Ok(slim_user) => {
                req.remember(slim_user.id.to_string());
                Ok(HttpResponse::Ok().into())
            }
            Err(err) => Ok(err.error_response()),
        })
        .responder()
}

/// Route to make logout attempt
pub fn logout(req: HttpRequest<RouteState>) -> HttpResponse {
    req.forget();
    HttpResponse::Ok().into()
}

/// Struct implementing Authentication middleware for api
pub struct WebAuthMiddleware;

impl Middleware<RouteState> for WebAuthMiddleware {
    fn start(&self, req: &HttpRequest<RouteState>) -> Result<Started> {
        if let Some(user_id) = req.identity() {
            let user_id = user_id
                .parse::<i32>()
                .expect("user_id should always be a i32");
            info!("authenticated user id: {}", user_id);
            Ok(Started::Done)
        } else if req.path() == "/login" {
            return Ok(Started::Done);
        } else {
            Ok(Started::Response(
                HttpResponse::Found()
                    .header(http::header::LOCATION, "/login")
                    .finish(),
            ))
        }
    }
}

/// Struct implementing authentication middleware
pub struct AuthMiddleware;

impl Middleware<RouteState> for AuthMiddleware {
    fn start(&self, req: &HttpRequest<RouteState>) -> Result<Started> {
        if let Some(user_id) = req.identity() {
            let user_id = user_id
                .parse::<i32>()
                .expect("user_id should always be a i32");
            info!("authenticated user id: {}", user_id);
            Ok(Started::Done)
        } else {
            Ok(Started::Response(HttpResponse::Unauthorized().finish()))
        }
    }
}
