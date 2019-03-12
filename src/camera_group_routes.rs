use actix_web::{AsyncResponder, FutureResponse, HttpResponse, Json, Path, ResponseError, State};
use futures::future::Future;

use crate::app::RouteState;
use crate::models::{CreateCameraGroup, FetchAllCameraGroup, FetchCameraGroup, UpdateCameraGroup};

/// Route to create new camera group
pub fn create_camera_group(
    (camera_group_request, state): (Json<CreateCameraGroup>, State<RouteState>),
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(camera_group_request.into_inner())
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera_group) => Ok(HttpResponse::Ok().json(camera_group)),
            Err(err) => Ok(err.error_response()),
        })
        .responder()
}

/// Route to update existing camera group
pub fn update_camera_group(
    (path, camera_group_request, state): (Path<i32>, Json<UpdateCameraGroup>, State<RouteState>),
) -> FutureResponse<HttpResponse> {
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
            Err(err) => Ok(err.error_response()),
        })
        .responder()
}

/// Route to fetch camera group by id
pub fn fetch_camera_group(
    (path, state): (Path<i32>, State<RouteState>),
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(FetchCameraGroup {
            id: path.into_inner(),
        })
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera_group) => Ok(HttpResponse::Ok().json(camera_group)),
            Err(err) => Ok(err.error_response()),
        })
        .responder()
}

/// Route to fetch all camera groups
pub fn fetch_all_camera_groups((state,): (State<RouteState>,)) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(FetchAllCameraGroup {})
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera_groups) => Ok(HttpResponse::Ok().json(camera_groups)),
            Err(err) => Ok(err.error_response()),
        })
        .responder()
}
