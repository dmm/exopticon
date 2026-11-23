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
    Json, Router,
    extract::{Path, State},
};
use oxvif::OnvifClient;
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::AppState;

use super::configuration::{create_camera, delete_camera, edit_camera};
use super::{ResourceMetadata, UserError};

mod onvif_probe;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CameraSpec {
    pub storage_group_name: String,
    pub stream_source: StreamSource,
    pub ptz: Option<PtzSettings>,
    pub enabled: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum StreamSource {
    StaticRtsp {
        url: String,
    },
    Onvif {
        device_url: String,
        profile_token: String,
    },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PtzSettings {
    pub device_url: Option<String>,
    pub profile_token: Option<String>,
    pub x_step_size: i16,
    pub y_step_size: i16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CameraStatus {
    pub phase: String,
    pub active: bool,
    pub last_started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub average_bitrate: Option<i64>,
    pub error_message: Option<String>,
}

impl CameraStatus {
    pub fn disabled() -> Self {
        Self {
            phase: "disabled".to_string(),
            active: false,
            last_started_at: None,
            video_codec: None,
            audio_codec: None,
            average_bitrate: None,
            error_message: None,
        }
    }

    pub fn starting() -> Self {
        Self {
            phase: "starting".to_string(),
            active: true,
            last_started_at: Some(chrono::Utc::now()),
            video_codec: None,
            audio_codec: None,
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
    let db = state.db_service.clone();
    let camera = spawn_blocking(move || db.fetch_camera_row(&name)).await??;
    move_camera(&camera, &direction).await
}

async fn move_camera(
    camera: &crate::db::cameras::Camera,
    direction: &str,
) -> Result<(), UserError> {
    if !camera.ptz_enabled {
        return Err(UserError::Validation(
            "PTZ is disabled for this camera".to_string(),
        ));
    }
    let zoom = 0.0;
    let x_step = f32::from(camera.ptz_x_step_size);
    let y_step = f32::from(camera.ptz_y_step_size);
    let (x, y) = match direction {
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

    let (endpoint, profile_token) = camera.ptz_target().ok_or_else(|| {
        UserError::Validation("PTZ device service URL and profile token are required".to_string())
    })?;
    let onvif_cam = OnvifClient::new(endpoint).with_credentials(&camera.username, &camera.password);

    if onvif_cam
        .ptz_relative_move(endpoint, profile_token, x, y, zoom)
        .await
        .is_err()
    {
        return Err(UserError::InternalError("relative move failed".to_string()));
    }

    Ok(())
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/", axum::routing::get(fetch_all).post(create_camera))
        .route("/onvif/probe", axum::routing::post(onvif_probe::probe))
        .route(
            "/{name}",
            axum::routing::get(fetch)
                .patch(edit_camera)
                .delete(delete_camera),
        )
        .route(
            "/{name}/ptz/{direction}",
            axum::routing::post(ptz_relative_move),
        )
}

#[cfg(test)]
mod tests;
