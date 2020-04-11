use actix_web::{web::Data, web::Json, Error, HttpResponse};

use crate::app::RouteState;
use crate::models::CreateUser;

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
        .await?;

    match db_response {
        Ok(slim_user) => Ok(HttpResponse::Ok().json(slim_user)),
        Err(err) => Ok(HttpResponse::InternalServerError().json(err.to_string())),
    }
}
