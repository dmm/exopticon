/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

// We have to pass by value to satisfy the actix route interface.
#![allow(clippy::needless_pass_by_value)]

use std::convert::TryFrom;
use std::fs;
use std::process::{Command, Stdio};
use std::time::Duration;

use actix_web::{web, web::Data, web::Path, web::Query, Error, HttpResponse};
use tempfile::tempdir;

use crate::app::RouteState;
use crate::db_registry;
use crate::models::{FetchObservation, FetchObservations, FetchVideoUnit};
use crate::video_unit_routes::DateRange;

/// Implements route the fetchs `Observation`s from the database
/// by the observation id specified.
///
/// # Arguments
/// * `observation_id` - id of observation to fetch
pub async fn fetch_observation(
    observation_id: Path<i64>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let db_response = state
        .db
        .send(FetchObservation {
            id: observation_id.into_inner(),
        })
        .await?;

    match db_response {
        Ok(observation) => Ok(HttpResponse::Ok().json(observation)),
        Err(err) => Ok(HttpResponse::InternalServerError().body(err.to_string())),
    }
}

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

/// Async fetch the image corresponding to an observation
pub async fn fetch_observation_image(observation_id: i64) -> Result<Vec<u8>, ()> {
    let db = db_registry::get_db();
    let obs_response = db
        .send(FetchObservation { id: observation_id })
        .await
        .map_err(|_| {
            error!("Error fetching FetchObservation");
        })?;

    match obs_response {
        Ok(observation) => {
            let mut unit_response = db
                .send(FetchVideoUnit {
                    id: observation.video_unit_id,
                })
                .await
                .map_err(|_| {
                    error!("Failed to send FetchVideoUnit message");
                })?
                .map_err(|_| {
                    error!("FetchVideoUnit db error!");
                })?;
            let file = unit_response.1.pop();
            if let Some(file) = file {
                let offset = u64::try_from(observation.frame_offset).map_err(|_| {
                    error!(
                        "Invalid offset in observation: {}",
                        observation.frame_offset
                    );
                })?;
                let snap = match web::block(move || get_snapshot(&file.filename, offset)).await {
                    Ok(jpg) => Ok(jpg),
                    Err(_) => Err(()),
                }?;
                Ok(snap)
            } else {
                error!("video unit db failed");
                Err(())
            }
        }
        Err(err) => {
            error!("observation error: {}", err.to_string());
            Err(())
        }
    }
}

/// returns snapshot for given video file path and offset
pub fn get_snapshot(path: &str, microsecond_offset: u64) -> Result<Vec<u8>, ()> {
    let offset = Duration::from_micros(microsecond_offset);
    debug!("Capturing snapshot: {} {}", &path, microsecond_offset);
    let child = Command::new("ffmpeg")
        .arg("-ss")
        .arg(format!("{}.{}", offset.as_secs(), offset.subsec_millis()))
        .arg("-i")
        .arg(path)
        .arg("-vframes")
        .arg("1")
        .arg("-vcodec")
        .arg("mjpeg")
        .arg("-q:v")
        .arg("2")
        .arg("-f")
        .arg("mjpeg")
        .arg("-")
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| {
            error!("failed to launch");
        })?;

    let output = child.wait_with_output().map_err(|_| {
        error!("failed to wait on child");
    })?;

    if output.stdout.is_empty() {
        Err(())
    } else {
        Ok(output.stdout)
    }
}

/// Implements fetching observation snapshot
pub async fn fetch_observation_snapshot(
    observation_id: Path<i64>,
    _state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let res = fetch_observation_image(observation_id.into_inner()).await;
    match res {
        Ok(snap) => Ok(HttpResponse::Ok().content_type("image/jpeg").body(snap)),
        Err(_) => Ok(HttpResponse::InternalServerError().body("failed to fetch image")),
    }
}

/// returns clip for given video file path, offset and time
pub fn get_clip(path: &str, microsecond_offset: u64, length: Duration) -> Result<Vec<u8>, ()> {
    debug!(
        "Capturing clip: {} {} {}",
        &path,
        microsecond_offset,
        length.as_secs()
    );
    // mp4 must be written to a file so create a temp dir
    let dir = tempdir().map_err(|_| {
        error!("failed to make tempdir");
    })?;
    let file_path = dir.path().join("output.mp4");
    debug!("File Path: {:?}", file_path);
    let offset = Duration::from_micros(microsecond_offset);
    let child = Command::new("ffmpeg")
        .arg("-noaccurate_seek")
        .arg("-ss")
        .arg(format!("{}.{}", offset.as_secs(), offset.subsec_millis()))
        .arg("-i")
        .arg(path)
        .arg("-vcodec")
        .arg("copy")
        .arg("-t")
        .arg(format!("{}.{}", length.as_secs(), length.subsec_millis()))
        .arg("-avoid_negative_ts")
        .arg("make_zero")
        .arg(&file_path)
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to launch");

    child.wait_with_output().map_err(|_| {
        error!("failed to wait on child");
    })?;

    let contents = fs::read(file_path).map_err(|_| {
        error!("Failed to read clip file.");
    })?;
    Ok(contents)
}

/// Async fetch the clip corresponding to an observation
async fn get_observation_clip(
    filename: String,
    frame_offset: u64,
    length: Duration,
) -> Result<Vec<u8>, ()> {
    match web::block(move || get_clip(&filename, frame_offset, length)).await {
        Ok(mp4) => Ok(mp4),
        Err(_) => Err(()),
    }
}

/// Route to fetch video clip for observation
pub async fn fetch_observation_clip(
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
                let offset = u64::try_from(observation.frame_offset).map_err(|_| {
                    HttpResponse::InternalServerError().body(format!(
                        "Invalid offset in observation: {}",
                        observation.frame_offset,
                    ))
                })?;
                let snap =
                    get_observation_clip(file.filename, offset, Duration::from_secs(5)).await?;
                Ok(HttpResponse::Ok().content_type("video/webm").body(snap))
            } else {
                Ok(HttpResponse::InternalServerError().body("video unit db failed"))
            }
        }
        Err(err) => Ok(HttpResponse::InternalServerError().body(err.to_string())),
    }
}
