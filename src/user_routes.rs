use actix_web::{AsyncResponder, FutureResponse, HttpResponse, Json, Path, ResponseError, State};
use chrono_tz::Tz;
use futures::future::Future;

use crate::app::AppState;
use crate::models::CreateUser;

pub fn create_user(
    (create_user, state): (Json<CreateUser>, State<AppState>),
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(CreateUser {
            username: create_user.username.clone(),
            password: create_user.password.clone(),
            timezone: create_user.timezone.clone(),
        })
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(slim_user) => Ok(HttpResponse::Ok().json(slim_user)),
            Err(service_error) => Ok(service_error.error_response()),
        })
        .responder()
}
