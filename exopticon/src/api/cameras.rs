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
    extract::{Path, State},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::{capture_supervisor::Command, AppState};

use super::UserError;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Camera {
    /// id of camera
    pub id: i32,

    #[serde(flatten)]
    pub common: CreateCamera,
}

impl From<crate::db::cameras::Camera> for Camera {
    fn from(c: crate::db::cameras::Camera) -> Self {
        Self {
            id: c.id,
            common: CreateCamera {
                storage_group_id: c.storage_group_id,
                name: c.name,
                ip: c.ip,
                onvif_port: c.onvif_port,
                mac: c.mac,
                username: c.username,
                password: c.password,
                rtsp_url: c.rtsp_url,
                ptz_type: c.ptz_type,
                ptz_profile_token: c.ptz_profile_token,
                enabled: c.enabled,
                ptz_x_step_size: c.ptz_x_step_size,
                ptz_y_step_size: c.ptz_y_step_size,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCamera {
    /// id of associated storage group
    pub storage_group_id: i32,
    /// name of camera
    pub name: String,
    /// ip address associated with camera, e.g. 192.168.0.53
    pub ip: String,
    /// port used for ONVIF protocol
    pub onvif_port: i32,
    /// MAC address of camera, e.g. 9C-84-AE-0E-33-5A
    pub mac: String,
    /// username for ONVIF and RTSP authentication
    pub username: String,
    /// plaintext password for ONVIF and RTSP authentication
    pub password: String,
    /// url for rtsp stream
    pub rtsp_url: String,
    /// ptz type, either onvif or onvif_continuous
    pub ptz_type: String,
    /// ONVIF profile token for ptz
    pub ptz_profile_token: String,
    /// whether camera capture is enabled.
    pub enabled: bool,
    /// ptz x step size, in hundredths
    pub ptz_x_step_size: i16,
    /// ptz y step size, in hundredths
    pub ptz_y_step_size: i16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCamera {
    /// if present, new storage group id
    pub storage_group_id: Option<i32>,
    /// if present, new camera name
    pub name: Option<String>,
    /// if present, new ip address
    pub ip: Option<String>,
    /// if present, new onvif port
    pub onvif_port: Option<i32>,
    /// if present, new MAC address
    pub mac: Option<String>,
    /// if present, new username for ONVIF and RTSP streaming
    pub username: Option<String>,
    /// if present, new plaintext password of ONVIF and RTSP streaming
    pub password: Option<String>,
    /// if present, new rtsp_url
    pub rtsp_url: Option<String>,
    /// if present, new ptz type
    pub ptz_type: Option<String>,
    /// if present, new ONVIF ptz profile token
    pub ptz_profile_token: Option<String>,
    /// if present, updates enabled status
    pub enabled: Option<bool>,
    /// ptz x step size, in hundredths
    pub ptz_x_step_size: Option<i16>,
    /// ptz y step size, in hundredths
    pub ptz_y_step_size: Option<i16>,
}

pub async fn create(
    State(state): State<AppState>,
    Json(camera_request): Json<CreateCamera>,
) -> Result<Json<Camera>, UserError> {
    let db = state.db_service;

    let camera = spawn_blocking(move || db.create_camera(camera_request)).await??;

    info!("Sending capture restart signal command");
    state.capture_channel.send(Command::RestartAll).await?;

    Ok(Json(camera))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(update_request): Json<UpdateCamera>,
) -> Result<Json<Camera>, UserError> {
    let db = state.db_service;

    let updated_camera = spawn_blocking(move || db.update_camera(id, update_request)).await??;

    info!("Sending capture restart signal command");
    state.capture_channel.send(Command::RestartAll).await?;

    Ok(Json(updated_camera))
}

pub async fn delete(Path(id): Path<i32>, State(state): State<AppState>) -> Result<(), UserError> {
    let db = state.db_service;

    spawn_blocking(move || db.delete_camera(id)).await??;
    Ok(())
}

pub async fn fetch(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<Camera>, UserError> {
    let db = state.db_service;

    let camera = spawn_blocking(move || db.fetch_camera(id)).await??;

    Ok(Json(camera))
}

pub async fn fetch_all(State(state): State<AppState>) -> Result<Json<Vec<Camera>>, UserError> {
    let db = state.db_service;

    let cameras = spawn_blocking(move || db.fetch_all_cameras()).await??;

    Ok(Json(cameras))
}

pub async fn ptz_relative_move(
    Path((id, direction)): Path<(i32, String)>,
    State(state): State<AppState>,
) -> Result<(), UserError> {
    let zoom = 0.0;

    let db = state.db_service.clone();
    let camera = spawn_blocking(move || db.fetch_camera(id)).await??;
    let onvif_cam = onvif::camera::Camera {
        host: camera.common.ip,
        port: camera.common.onvif_port,
        username: camera.common.username,
        password: camera.common.password,
    };

    let x_step = f32::from(camera.common.ptz_x_step_size);
    let y_step = f32::from(camera.common.ptz_y_step_size);
    let (x, y) = match direction.as_str() {
        "left" => (x_step / -100.0f32, 0.0),
        "right" => (x_step / 100f32, 0.0),
        "up" => (0.0, y_step / 100f32),
        "down" => (0.0, y_step / -100f32),
        _ => {
            return Err(UserError::Validation(
                "invalid direction provided".to_string(),
            ))
        }
    };

    if camera.common.ptz_type == "onvif_continuous" {
        //camera.ptz_type == "onvif_continuous"
        // other cases??
        // start continuous move
        let con = onvif_cam
            .continuous_move(&camera.common.ptz_profile_token, x, y, zoom, 500.0)
            .await;

        if let Err(_err) = con {
            return Err(UserError::InternalError);
        }

        // wait
        tokio::time::sleep(Duration::from_millis(500)).await;

        // stop continuous move
        let con = onvif_cam.stop(&camera.common.ptz_profile_token).await;

        if let Err(_err) = con {
            return Err(UserError::InternalError);
        }
    } else {
        // default to using a relative move
        if let Err(_err) = onvif_cam
            .relative_move(&camera.common.ptz_profile_token, x, y, zoom)
            .await
        {
            return Err(UserError::InternalError);
        }
    }

    Ok(())
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/", axum::routing::get(fetch_all).post(create))
        .route(
            "/:id",
            axum::routing::get(fetch).post(update).delete(delete),
        )
        .route(
            "/:id/ptz/:direction",
            axum::routing::post(ptz_relative_move),
        )
}
