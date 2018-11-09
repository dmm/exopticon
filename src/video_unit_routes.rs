use actix_web::{AsyncResponder, FutureResponse, HttpResponse, Json, Path, ResponseError, State};
use chrono::NaiveDateTime;
use futures::future::Future;

use app::AppState;
use models::{CreateVideoUnit, FetchBetweenVideoUnit, FetchVideoUnit, UpdateVideoUnit};

pub fn create_video_unit(
    (video_unit_request, state): (Json<CreateVideoUnit>, State<AppState>),
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(video_unit_request.into_inner())
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(video_unit) => Ok(HttpResponse::Ok().json(video_unit)),
            Err(err) => Ok(err.error_response()),
        }).responder()
}

pub fn update_video_unit(
    (path, video_unit_request, state): (Path<i32>, Json<UpdateVideoUnit>, State<AppState>),
) -> FutureResponse<HttpResponse> {
    let video_unit_update = UpdateVideoUnit {
        id: path.into_inner(),
        ..video_unit_request.into_inner()
    };
    state
        .db
        .send(video_unit_update)
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(video_unit) => Ok(HttpResponse::Ok().json(video_unit)),
            Err(err) => Ok(err.error_response()),
        }).responder()
}

pub fn fetch_video_unit(
    (path, state): (Path<i32>, State<AppState>),
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(FetchVideoUnit {
            id: path.into_inner(),
        }).from_err()
        .and_then(|db_response| match db_response {
            Ok(video_unit) => Ok(HttpResponse::Ok().json(video_unit)),
            Err(err) => Ok(err.error_response()),
        }).responder()
}

pub fn fetch_video_units_between(
    (begin, end, state): (Path<NaiveDateTime>, Path<NaiveDateTime>, State<AppState>),
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(FetchBetweenVideoUnit {
            begin_time: begin.into_inner(),
            end_time: end.into_inner(),
        }).from_err()
        .and_then(|db_response| match db_response {
            Ok(video_units) => Ok(HttpResponse::Ok().json(video_units)),
            Err(err) => Ok(err.error_response()),
        }).responder()
}
