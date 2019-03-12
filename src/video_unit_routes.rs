use actix_web::{AsyncResponder, FutureResponse, HttpResponse, Path, ResponseError, State};
use chrono::NaiveDateTime;
use futures::future::Future;

use crate::app::RouteState;
use crate::models::{FetchBetweenVideoUnit, FetchVideoUnit};

/// Implements route that fetches a single `VideoUnit` specified by id.
///
/// # Arguments
///
/// * `path` - `Path` containing `VideoUnit` id
/// * `state` - `RouteState` struct
///
pub fn fetch_video_unit(
    (path, state): (Path<i32>, State<RouteState>),
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(FetchVideoUnit {
            id: path.into_inner(),
        })
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(video_unit) => Ok(HttpResponse::Ok().json(video_unit)),
            Err(err) => Ok(err.error_response()),
        })
        .responder()
}

/// Implements route that fetches `VideoUnit`s from the database
/// between the specified times, inclusively.
///
/// # Arguments
///
/// * `begin` - begin time in UTC
/// * `end` - end time in UTC
/// * `state` - `RouteState` struct
///
pub fn fetch_video_units_between(
    (begin, end, state): (Path<NaiveDateTime>, Path<NaiveDateTime>, State<RouteState>),
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(FetchBetweenVideoUnit {
            begin_time: begin.into_inner(),
            end_time: end.into_inner(),
        })
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(video_units) => Ok(HttpResponse::Ok().json(video_units)),
            Err(err) => Ok(err.error_response()),
        })
        .responder()
}
