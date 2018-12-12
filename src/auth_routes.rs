use actix_web::middleware::identity::RequestIdentity;
use actix_web::{AsyncResponder, FutureResponse, HttpRequest, HttpResponse, Json, ResponseError};
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
