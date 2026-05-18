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

use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::AppState;

use super::{ResourceMetadata, UserError};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CameraSpec {
    pub storage_group_name: String,
    pub ip: String,
    pub onvif_port: i32,
    pub mac: String,
    pub username: String,
    pub rtsp_url: String,
    pub ptz_type: String,
    pub ptz_profile_token: String,
    pub enabled: bool,
    pub ptz_x_step_size: i16,
    pub ptz_y_step_size: i16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CameraStatus {
    pub phase: String,
    pub active: bool,
    pub last_started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_packet_at: Option<chrono::DateTime<chrono::Utc>>,
    pub codec: Option<String>,
    pub average_bitrate: Option<i64>,
    pub error_message: Option<String>,
}

impl CameraStatus {
    pub fn disabled() -> Self {
        Self {
            phase: "disabled".to_string(),
            active: false,
            last_started_at: None,
            last_packet_at: None,
            codec: None,
            average_bitrate: None,
            error_message: None,
        }
    }

    pub fn starting() -> Self {
        Self {
            phase: "starting".to_string(),
            active: true,
            last_started_at: Some(chrono::Utc::now()),
            last_packet_at: None,
            codec: None,
            average_bitrate: None,
            error_message: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Camera {
    pub metadata: ResourceMetadata,
    pub spec: CameraSpec,
    pub status: CameraStatus,
}

pub async fn fetch(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Camera>, UserError> {
    let db = state.db_service;
    let status_registry = state.camera_status_registry;

    let mut camera = spawn_blocking(move || db.fetch_camera(&name)).await??;
    let status = status_registry
        .read()
        .await
        .get(&camera.metadata.name)
        .cloned();
    if let Some(status) = status {
        camera.status = status;
    }

    Ok(Json(camera))
}

pub async fn fetch_all(State(state): State<AppState>) -> Result<Json<Vec<Camera>>, UserError> {
    let db = state.db_service;
    let status_registry = state.camera_status_registry;

    let mut cameras = spawn_blocking(move || db.fetch_all_cameras()).await??;
    let statuses = status_registry.read().await;
    for camera in &mut cameras {
        if let Some(status) = statuses.get(&camera.metadata.name).cloned() {
            camera.status = status;
        }
    }

    Ok(Json(cameras))
}

pub async fn ptz_relative_move(
    Path((name, direction)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<(), UserError> {
    let zoom = 0.0;

    let db = state.db_service.clone();
    let camera = spawn_blocking(move || db.fetch_camera_row(&name)).await??;
    let onvif_cam = onvif::camera::Camera {
        host: camera.ip,
        port: camera.onvif_port,
        username: camera.username,
        password: camera.password,
    };

    let x_step = f32::from(camera.ptz_x_step_size);
    let y_step = f32::from(camera.ptz_y_step_size);
    let (x, y) = match direction.as_str() {
        "left" => (x_step / -100.0f32, 0.0),
        "right" => (x_step / 100f32, 0.0),
        "up" => (0.0, y_step / 100f32),
        "down" => (0.0, y_step / -100f32),
        _ => {
            return Err(UserError::Validation(
                "invalid direction provided".to_string(),
            ));
        }
    };

    if camera.ptz_type == "onvif_continuous" {
        if onvif_cam
            .continuous_move(&camera.ptz_profile_token, x, y, zoom, 500.0)
            .await
            .is_err()
        {
            return Err(UserError::InternalError(
                "begin continuous move failed".to_string(),
            ));
        }

        tokio::time::sleep(Duration::from_millis(500)).await;

        if onvif_cam.stop(&camera.ptz_profile_token).await.is_err() {
            return Err(UserError::InternalError(
                "stop continuous move failed".to_string(),
            ));
        }
    } else if onvif_cam
        .relative_move(&camera.ptz_profile_token, x, y, zoom)
        .await
        .is_err()
    {
        return Err(UserError::InternalError("relative move failed".to_string()));
    }

    Ok(())
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/", axum::routing::get(fetch_all))
        .route("/:name", axum::routing::get(fetch))
        .route(
            "/:name/ptz/:direction",
            axum::routing::post(ptz_relative_move),
        )
}
