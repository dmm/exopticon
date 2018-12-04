use actix_web::{AsyncResponder, FutureResponse, HttpResponse, Json, Path, ResponseError, State};
use futures::future::Future;

use crate::app::AppState;
use crate::models::{CreateCameraGroup, FetchAllCameraGroup, FetchCameraGroup, UpdateCameraGroup};

pub fn create_camera_group(
    (camera_group_request, state): (Json<CreateCameraGroup>, State<AppState>),
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(camera_group_request.into_inner())
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera_group) => Ok(HttpResponse::Ok().json(camera_group)),
            Err(err) => Ok(err.error_response()),
        }).responder()
}

pub fn update_camera_group(
    (path, camera_group_request, state): (Path<i32>, Json<UpdateCameraGroup>, State<AppState>),
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
        }).responder()
}

pub fn fetch_camera_group(
    (path, state): (Path<i32>, State<AppState>),
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(FetchCameraGroup {
            id: path.into_inner(),
        }).from_err()
        .and_then(|db_response| match db_response {
            Ok(camera_group) => Ok(HttpResponse::Ok().json(camera_group)),
            Err(err) => Ok(err.error_response()),
        }).responder()
}

pub fn fetch_all_camera_groups((state,): (State<AppState>,)) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(FetchAllCameraGroup {})
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera_groups) => Ok(HttpResponse::Ok().json(camera_groups)),
            Err(err) => Ok(err.error_response()),
        }).responder()
}
