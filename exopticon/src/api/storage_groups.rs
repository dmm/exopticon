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
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::AppState;

use super::{ResourceMetadata, UserError};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StorageGroupSpec {
    pub storage_path: String,
    pub max_storage_size: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StorageGroup {
    pub metadata: ResourceMetadata,
    pub spec: StorageGroupSpec,
    pub status: serde_json::Value,
}

pub async fn fetch(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<StorageGroup>, UserError> {
    let db = state.db_service;

    let storage_group = spawn_blocking(move || db.fetch_storage_group(&name)).await??;

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
        .route("/", axum::routing::get(fetch_all))
        .route("/{name}", axum::routing::get(fetch))
}
