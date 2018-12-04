use actix_web::{AsyncResponder, FutureResponse, HttpResponse, Path, ResponseError, State};
use chrono::NaiveDateTime;
use futures::future::Future;

use crate::app::AppState;
use crate::models::{FetchBetweenVideoUnit, FetchVideoUnit};

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
