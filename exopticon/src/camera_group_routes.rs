// We have to pass by value to satisfy the actix route interface.
#![allow(clippy::needless_pass_by_value)]

use actix_web::{error::ResponseError, web::Data, web::Json, web::Path, Error, HttpResponse};
use futures::future::Future;

use crate::app::RouteState;
use crate::models::{CreateCameraGroup, FetchAllCameraGroup, FetchCameraGroup, UpdateCameraGroup};

/// Route to create new camera group
pub fn create_camera_group(
    camera_group_request: Json<CreateCameraGroup>,
    state: Data<RouteState>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    state
        .db
        .send(camera_group_request.into_inner())
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera_group) => Ok(HttpResponse::Ok().json(camera_group)),
            Err(err) => Ok(err.render_response()),
        })
}

/// Route to update existing camera group
pub fn update_camera_group(
    path: Path<i32>,
    camera_group_request: Json<UpdateCameraGroup>,
    state: Data<RouteState>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    let camera_group_update = UpdateCameraGroup {
        id: path.into_inner(),
        ..camera_group_request.into_inner()
    };
    state
        .db
        .send(camera_group_update)
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera_group) => Ok(HttpResponse::Ok().json(camera_group)),
            Err(err) => Ok(err.render_response()),
        })
}

/// Route to fetch camera group by id
pub fn fetch_camera_group(
    path: Path<i32>,
    state: Data<RouteState>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    state
        .db
        .send(FetchCameraGroup {
            id: path.into_inner(),
        })
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera_group) => Ok(HttpResponse::Ok().json(camera_group)),
            Err(err) => Ok(err.render_response()),
        })
}

/// Route to fetch all camera groups
pub fn fetch_all_camera_groups(
    state: Data<RouteState>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    state
        .db
        .send(FetchAllCameraGroup {})
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera_groups) => Ok(HttpResponse::Ok().json(camera_groups)),
            Err(err) => Ok(err.render_response()),
        })
}
