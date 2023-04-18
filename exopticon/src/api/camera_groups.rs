/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2022 David Matthew Mattli <dmm@mattli.us>
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
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use tokio::task::spawn_blocking;

use crate::{db::Service, AppState};

use super::UserError;

// Route Models

/// `CameraGroup` api resource
#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct CameraGroup {
    pub id: i32,
    pub name: String,
    pub members: Vec<i32>,
}

/// Request to create new `CameraGroup`
#[derive(Clone, Serialize, Deserialize)]
pub struct CreateCameraGroup {
    pub name: String,
    pub members: Vec<i32>,
}

// Routes

pub async fn create(
    State(state): State<AppState>,
    Json(camera_group_request): Json<CreateCameraGroup>,
) -> Result<Json<CameraGroup>, UserError> {
    let db = state.db_service;
    let req = camera_group_request;
    let camera_group = crate::business::camera_groups::CameraGroup::new(&req.name, req.members)?;
    let camera_group = spawn_blocking(move || db.create_camera_group(camera_group)).await??;
    Ok(Json(camera_group))
}

pub async fn update(
    State(state): State<AppState>,
    Json(camera_group_request): Json<CameraGroup>,
) -> Result<Json<CameraGroup>, UserError> {
    let db = state.db_service;
    let req = camera_group_request;
    let camera_group = crate::business::camera_groups::CameraGroup::new(&req.name, req.members)?;
    let camera_group =
        spawn_blocking(move || db.update_camera_group(req.id, camera_group)).await??;
    Ok(Json(camera_group))
}

pub async fn delete(id: Path<i32>, State(state): State<AppState>) -> Result<(), UserError> {
    let db = state.db_service;
    spawn_blocking(move || db.delete_camera_group(id.0)).await??;
    Ok(())
}

pub async fn fetch(
    id: Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<CameraGroup>, UserError> {
    let db = state.db_service;
    let camera_group = spawn_blocking(move || db.fetch_camera_group(id.0)).await??;
    Ok(Json(camera_group))
}

pub async fn fetch_all(State(state): State<AppState>) -> Result<Json<Vec<CameraGroup>>, UserError> {
    let db = state.db_service;
    let camera_groups = spawn_blocking(move || db.fetch_all_camera_groups()).await??;
    Ok(Json(camera_groups))
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/", get(fetch_all).post(create))
        .route("/:id", get(fetch).post(update).delete(delete))
}

#[cfg(test)]
mod tests {}
