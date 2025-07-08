/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2023 David Matthew Mattli <dmm@mattli.us>
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

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use tokio::task::spawn_blocking;
use uuid::Uuid;

use crate::AppState;

/// Public video unit model
/// Full video unit model, represents entire database row
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VideoUnit {
    /// id of video unit
    pub id: Uuid,
    /// id of associated camera
    pub camera_id: Uuid,
    /// begin time in UTC
    pub begin_time: DateTime<Utc>,
    /// end time in UTC
    pub end_time: DateTime<Utc>,
}

/// Represents request to create new video unit record
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVideoUnit {
    /// id of associated camera
    pub camera_id: Uuid,
    /// begin time in UTC
    pub begin_time: DateTime<Utc>,
    /// end time in UTC
    pub end_time: DateTime<Utc>,
    /// id of video unit
    pub id: Uuid,
}

/// Full video file model, represents full database row
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFile {
    /// id of video file
    pub id: Uuid,
    /// filename of video file
    pub filename: String,
    /// size in bytes of video file
    pub size: i32,
    /// id of associated video unit
    pub video_unit_id: Uuid,
}

/// Represents request to create new video file
#[derive(Debug, Deserialize)]
pub struct CreateVideoFile {
    /// filename for new video file
    pub filename: String,
    /// size in bytes of new video file
    pub size: i32,
    /// id of video unit to own this video file
    pub video_unit_id: Uuid,
}

#[derive(Deserialize)]
pub struct Interval {
    begin_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

pub async fn fetch_video_units_between(
    interval: Query<Interval>,
    Path(camera_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<Vec<(VideoUnit, VideoFile)>>, super::UserError> {
    let db = state.db_service;

    let video_units = spawn_blocking(move || {
        db.fetch_video_units_between(camera_id.into(), interval.begin_time, interval.end_time)
    })
    .await??;

    Ok(Json(video_units))
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new().route("/:camera_id", get(fetch_video_units_between))
}
