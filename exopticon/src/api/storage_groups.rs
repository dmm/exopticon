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
    extract::{Path, State},
    Json, Router,
};
use tokio::task::spawn_blocking;

use crate::AppState;

use super::UserError;

/// Public `StorageGroup` model
#[derive(Debug, Serialize)]
pub struct StorageGroup {
    /// storage group id
    pub id: i32,
    /// storage group name
    pub name: String,
    /// full path to video storage path, e.g. /mnt/video/8/
    pub storage_path: String,
    /// maximum allowed storage size in bytes
    pub max_storage_size: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStorageGroup {
    /// storage group name
    pub name: String,
    /// full path to video storage path, e.g. /mnt/video/8/
    pub storage_path: String,
    /// maximum allowed storage size in bytes
    pub max_storage_size: i64,
}

/// Represents a request to update fields of a `StorageGroup`
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStorageGroup {
    /// storage group id
    pub id: i32,
    /// storage group name
    pub name: Option<String>,
    /// full path to video storage path, e.g. /mnt/video/8/
    pub storage_path: Option<String>,
    /// maximum allowed storage size in bytes
    pub max_storage_size: Option<i64>,
}

// Routes

pub async fn create(
    State(state): State<AppState>,
    Json(storage_group_request): Json<CreateStorageGroup>,
) -> Result<Json<StorageGroup>, UserError> {
    let db = state.db_service;
    let storage_group =
        spawn_blocking(move || db.create_storage_group(storage_group_request)).await??;

    Ok(Json(storage_group))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(update_request): Json<UpdateStorageGroup>,
) -> Result<Json<StorageGroup>, UserError> {
    let db = state.db_service;

    let updated_group =
        spawn_blocking(move || db.update_storage_group(id, update_request)).await??;

    Ok(Json(updated_group))
}

pub async fn delete(Path(id): Path<i32>, State(state): State<AppState>) -> Result<(), UserError> {
    let db = state.db_service;

    spawn_blocking(move || db.delete_storage_group(id)).await??;

    Ok(())
}

pub async fn fetch(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<StorageGroup>, UserError> {
    let db = state.db_service;

    let storage_group = spawn_blocking(move || db.fetch_storage_group(id)).await??;

    Ok(Json(storage_group))
}

pub async fn fetch_all(
    State(state): State<AppState>,
) -> Result<Json<Vec<StorageGroup>>, UserError> {
    let db = state.db_service;
    let groups = spawn_blocking(move || db.fetch_all_storage_groups()).await??;

    Ok(Json(groups))
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/", axum::routing::get(fetch_all).post(create))
        .route(
            "/:id",
            axum::routing::get(fetch).post(update).delete(delete),
        )
}
