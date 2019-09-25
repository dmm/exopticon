use actix_web::error::ResponseError;
use actix_web::{AsyncResponder, FutureResponse, HttpResponse, Path, Query, State};
use chrono::{DateTime, Utc};
use futures::future::Future;

use crate::app::RouteState;
use crate::models::{FetchBetweenVideoUnit, FetchVideoUnit};

/// Implements route that fetches a single `VideoUnit` specified by id.
///
/// # Arguments
///
/// * `path` - `Path` containing `VideoUnit` id
/// * `state` - `RouteState` struct
///p
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

/// Struct used to match time parameters on api route
#[derive(Deserialize)]
pub struct DateRange {
    /// end time - inclusive
    pub end_time: DateTime<Utc>,
    /// begin_time - exclusive
    pub begin_time: DateTime<Utc>,
}

/// Implements route that fetches `VideoUnit`s from the database
/// between the specified times, inclusively.
///
/// # Arguments
///
/// * `camera_id` - id of camera to fetch video for
/// * `begin` - begin time in UTC
/// * `end` - end time in UTC
/// * `state` - `RouteState` struct
///
pub fn fetch_video_units_between(
    (camera_id, range, state): (Path<i32>, Query<DateRange>, State<RouteState>),
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(FetchBetweenVideoUnit {
            camera_id: camera_id.into_inner(),
            begin_time: range.begin_time,
            end_time: range.end_time,
        })
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(video_units) => Ok(HttpResponse::Ok().json(video_units)),
            Err(err) => Ok(err.error_response()),
        })
        .responder()
}
