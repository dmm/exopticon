use actix_web::{error::ResponseError, web::Data, web::Path, web::Query, Error, HttpResponse};
use futures::future::Future;

use crate::app::RouteState;
use crate::models::FetchObservations;
use crate::video_unit_routes::DateRange;

/// Implements route that fetches `VideoUnit`s from the database
/// between the specified times, inclusively.
///
/// # Arguments
///
/// * `camera_id` - id of camera to fetch video for
/// * `begin` - begin time in UTC
/// * `end` - end time in UTC
/// * `req` - `HttpRequest`
///
pub fn fetch_observations_between(
    camera_id: Path<i32>,
    range: Query<DateRange>,
    state: Data<RouteState>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    state
        .db
        .send(FetchObservations {
            camera_id: camera_id.into_inner(),
            begin_time: range.begin_time,
            end_time: range.end_time,
        })
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(video_units) => Ok(HttpResponse::Ok().json(video_units)),
            Err(err) => Ok(err.render_response()),
        })
}
