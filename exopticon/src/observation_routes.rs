// We have to pass by value to satisfy the actix route interface.
#![allow(clippy::needless_pass_by_value)]

use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::app::RouteState;
use crate::models::{FetchObservation, FetchObservations, FetchVideoUnit};
use crate::video_unit_routes::DateRange;
use actix_web::{web, web::Data, web::Path, web::Query, Error, HttpResponse};

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
pub async fn fetch_observations_between(
    camera_id: Path<i32>,
    range: Query<DateRange>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let db_response = state
        .db
        .send(FetchObservations {
            camera_id: camera_id.into_inner(),
            begin_time: range.begin_time,
            end_time: range.end_time,
        })
        .await?;

    match db_response {
        Ok(video_units) => Ok(HttpResponse::Ok().json(video_units)),
        Err(err) => Ok(HttpResponse::InternalServerError().body(err.to_string())),
    }
}

async fn fetch_observation_image(filename: &str, frame_offset: i64) -> Result<Vec<u8>, ()> {
    let uri = format!("file://{}", filename);
    match web::block(move || get_snapshot(&uri, frame_offset)).await {
        Ok(jpg) => Ok(jpg),
        Err(_) => Err(()),
    }
}

pub fn get_snapshot(path: &str, microsecond_offset: i64) -> Result<Vec<u8>, ()> {
    let worker_path = env::var("EXOPTICONWORKERS").unwrap_or_else(|_| "/".to_string());
    debug!("WORKER PATH: {}", worker_path);
    let _executable_path: PathBuf = [
        worker_path,
        "../../debug".to_string(),
        "snapshot_worker".to_string(),
    ]
    .iter()
    .collect();

    let child = Command::new("/exopticon/target/debug/exsnap")
        .arg(path)
        .arg(microsecond_offset.to_string())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to launch");

    let output = child.wait_with_output().expect("failed to wait on child");

    Ok(output.stdout)
}

/// Implements fetching observation snapshot
pub async fn fetch_observation_snapshot(
    observation_id: Path<i64>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let obs_response = state
        .db
        .send(FetchObservation {
            id: observation_id.into_inner(),
        })
        .await?;

    match obs_response {
        Ok(observation) => {
            let mut unit_response = state
                .db
                .send(FetchVideoUnit {
                    id: observation.video_unit_id,
                })
                .await??;
            let file = unit_response.1.pop();
            if let Some(file) = file {
                let snap =
                    fetch_observation_image(&file.filename, observation.frame_offset).await?;
                Ok(HttpResponse::Ok().content_type("image/jpeg").body(snap))
            } else {
                Ok(HttpResponse::InternalServerError().body("video unit db failed"))
            }
        }
        Err(err) => Ok(HttpResponse::InternalServerError().body(err.to_string())),
    }
}
