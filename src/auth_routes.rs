use actix_web::http;
use actix_web::http::{header, HttpTryFrom};
use actix_web::middleware::identity::RequestIdentity;
use actix_web::middleware::{Middleware, Response, Started};
use actix_web::{
    App, AsyncResponder, FutureResponse, HttpRequest, HttpResponse, Json, ResponseError, Result,
};
use futures::future::Future;

use crate::app::AppState;
use crate::auth_handler::AuthData;

pub fn login(
    (auth_data, req): (Json<AuthData>, HttpRequest<AppState>),
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

pub fn logout(req: HttpRequest<AppState>) -> HttpResponse {
    req.forget();
    HttpResponse::Ok().into()
}

pub struct WebAuthMiddleware;

impl Middleware<AppState> for WebAuthMiddleware {
    fn start(&self, req: &HttpRequest<AppState>) -> Result<Started> {
        if let Some(user_id) = req.identity() {
            let user_id = user_id
                .parse::<i32>()
                .expect("user_id should always be a i32");
            info!("authenticated user id: {}", user_id);
            Ok(Started::Done)
        } else if (req.path() == "/login") {
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

pub struct AuthMiddleware;

impl Middleware<AppState> for AuthMiddleware {
    fn start(&self, req: &HttpRequest<AppState>) -> Result<Started> {
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
