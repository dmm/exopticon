use actix_web::{AsyncResponder, FutureResponse, HttpResponse, Json, ResponseError, State};
use futures::future::Future;

use crate::app::RouteState;
use crate::models::CreateUser;

/// Implements route to create user, returns future returning created
/// user or error.
///
/// # Arguments
/// `create_user` - `Json` representation of `CreateUser` struct
/// `state` - `RouteState` struct
///
pub fn create_user(
    (create_user, state): (Json<CreateUser>, State<RouteState>),
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
