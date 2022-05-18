/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020-2022 David Matthew Mattli <dmm@mattli.us>
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

use actix_web::{web::Data, web::Json, web::Path, HttpResponse};

use crate::app::RouteState;
use crate::errors::ServiceError;
use crate::models::{
    CreateStorageGroup, FetchAllStorageGroup, FetchStorageGroup, UpdateStorageGroup,
};

/// Route to create new storage group
pub async fn create_storage_group(
    storage_group_request: Json<CreateStorageGroup>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    let db_response = state.db.send(storage_group_request.into_inner()).await?;

    match db_response {
        Ok(storage_group) => Ok(HttpResponse::Ok().json(storage_group)),
        Err(err) => Ok(HttpResponse::InternalServerError().body(err.to_string())),
    }
}

/// Route to update existing storage group
pub async fn update_storage_group(
    path: Path<i32>,
    storage_group_request: Json<UpdateStorageGroup>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    let storage_group_update = UpdateStorageGroup {
        id: path.into_inner(),
        ..storage_group_request.into_inner()
    };
    let db_response = state.db.send(storage_group_update).await?;

    match db_response {
        Ok(storage_group) => Ok(HttpResponse::Ok().json(storage_group)),
        Err(err) => Ok(HttpResponse::InternalServerError().body(err.to_string())),
    }
}

/// Route to fetch storage group by id
pub async fn fetch_storage_group(
    path: Path<i32>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    let db_response = state
        .db
        .send(FetchStorageGroup {
            id: path.into_inner(),
        })
        .await?;

    match db_response {
        Ok(storage_group) => Ok(HttpResponse::Ok().json(storage_group)),
        Err(err) => Ok(HttpResponse::InternalServerError().body(err.to_string())),
    }
}

/// Route to fetch all storage groups
pub async fn fetch_all_storage_groups(
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    let db_response = state.db.send(FetchAllStorageGroup {}).await?;

    match db_response {
        Ok(storage_groups) => Ok(HttpResponse::Ok().json(storage_groups)),
        Err(err) => Ok(HttpResponse::InternalServerError().body(err.to_string())),
    }
}
