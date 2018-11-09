use actix_web::{AsyncResponder, FutureResponse, HttpResponse, Json, Path, ResponseError, State};
use futures::future::Future;

use app::AppState;
use models::{CreateCamera, FetchAllCamera, FetchCamera, UpdateCamera};

pub fn create_camera(
    (camera_request, state): (Json<CreateCamera>, State<AppState>),
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(camera_request.into_inner())
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera) => Ok(HttpResponse::Ok().json(camera)),
            Err(err) => Ok(err.error_response()),
        }).responder()
}

pub fn update_camera(
    (path, camera_request, state): (Path<i32>, Json<UpdateCamera>, State<AppState>),
) -> FutureResponse<HttpResponse> {
    let camera_update = UpdateCamera {
        id: path.into_inner(),
        ..camera_request.into_inner()
    };
    state
        .db
        .send(camera_update)
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera) => Ok(HttpResponse::Ok().json(camera)),
            Err(err) => Ok(err.error_response()),
        }).responder()
}

pub fn fetch_camera((path, state): (Path<i32>, State<AppState>)) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(FetchCamera {
            id: path.into_inner(),
        }).from_err()
        .and_then(|db_response| match db_response {
            Ok(camera) => Ok(HttpResponse::Ok().json(camera)),
            Err(err) => Ok(err.error_response()),
        }).responder()
}

pub fn fetch_all_cameras((state,): (State<AppState>,)) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(FetchAllCamera {})
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(cameras) => Ok(HttpResponse::Ok().json(cameras)),
            Err(err) => Ok(err.error_response()),
        }).responder()
}
