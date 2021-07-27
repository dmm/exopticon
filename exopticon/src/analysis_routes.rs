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

use actix::registry::SystemService;
use actix_web::{web::Data, web::Json, web::Path, HttpResponse};

use crate::analysis_supervisor::{AnalysisSupervisor, SyncAnalysisActors};
use crate::app::RouteState;
use crate::errors::ServiceError;
use crate::models::{
    CreateAnalysisEngine, CreateAnalysisInstanceModel, DeleteAnalysisEngine,
    DeleteAnalysisInstanceModel, FetchAnalysisEngine, FetchAnalysisInstanceModel,
    UpdateAnalysisEngine, UpdateAnalysisInstanceModel,
};

#[derive(PartialEq, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AnalysisType {
    None = 0,
    Motion = 1,
    Yolo = 2,
    Coral = 3,
    Event = 4,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisConfiguration {
    pub camera_id: i32,
    pub analysis_type: AnalysisType,
}

pub struct FetchAnalysisConfiguration {
    pub camera_id: i32,
}

/// Route to create new `AnalysisEngine`
pub async fn create_analysis_engine(
    analysis_engine_request: Json<CreateAnalysisEngine>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    let analysis_engine = state
        .db
        .send(analysis_engine_request.into_inner())
        .await??;

    Ok(HttpResponse::Ok().json(analysis_engine))
}

/// Route to fetch an `AnalysisEngine`
pub async fn fetch_analysis_engine(
    path: Path<i32>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    let analysis_engine = state
        .db
        .send(FetchAnalysisEngine {
            id: path.into_inner(),
        })
        .await??;
    Ok(HttpResponse::Ok().json(analysis_engine))
}

/// Route to update an `AnalysisEngine`
pub async fn update_analysis_engine(
    path: Path<i32>,
    analysis_engine_request: Json<UpdateAnalysisEngine>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    let analysis_engine_update = UpdateAnalysisEngine {
        id: path.into_inner(),
        ..analysis_engine_request.into_inner()
    };

    let new_analysis_engine = state.db.send(analysis_engine_update).await??;

    Ok(HttpResponse::Ok().json(new_analysis_engine))
}

/// route to delete an `AnalysisEngine`
pub async fn delete_analysis_engine(
    path: Path<i32>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    state
        .db
        .send(DeleteAnalysisEngine {
            id: path.into_inner(),
        })
        .await??;

    Ok(HttpResponse::Ok().finish())
}

/// route to create an `AnalysisInstanceModel`
pub async fn create_analysis_instance(
    analysis_instance_request: Json<CreateAnalysisInstanceModel>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    let new_analysis_instance = state
        .db
        .send(analysis_instance_request.into_inner())
        .await??;

    AnalysisSupervisor::from_registry()
        .send(SyncAnalysisActors {})
        .await?;

    Ok(HttpResponse::Ok().json(new_analysis_instance))
}

/// route to fetch an `AnalysisInstanceModel`
pub async fn fetch_analysis_instance(
    path: Path<i32>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    let analysis_instance = state
        .db
        .send(FetchAnalysisInstanceModel {
            id: path.into_inner(),
        })
        .await??;

    Ok(HttpResponse::Ok().json(analysis_instance))
}

/// route to update an `AnalysisInstanceModel`
pub async fn update_analysis_instance(
    path: Path<i32>,
    analysis_instance_update: Json<UpdateAnalysisInstanceModel>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    let updated_instance = state
        .db
        .send(UpdateAnalysisInstanceModel {
            id: path.into_inner(),
            ..analysis_instance_update.into_inner()
        })
        .await??;

    AnalysisSupervisor::from_registry()
        .send(SyncAnalysisActors {})
        .await?;

    Ok(HttpResponse::Ok().json(updated_instance))
}

/// route to delete an `AnalysisInstanceModel`
pub async fn delete_analysis_instance(
    path: Path<i32>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    state
        .db
        .send(DeleteAnalysisInstanceModel {
            id: path.into_inner(),
        })
        .await??;

    AnalysisSupervisor::from_registry()
        .send(SyncAnalysisActors {})
        .await?;

    Ok(HttpResponse::Ok().finish())
}

pub async fn fetch_analysis_configuration(
    path: Path<i32>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    let analysis_configuration = state
        .db
        .send(FetchAnalysisConfiguration {
            camera_id: path.into_inner(),
        })
        .await??;
    Ok(HttpResponse::Ok().json(analysis_configuration))
}

pub async fn update_analysis_configuration(
    path: Path<i32>,
    analysis_configuration: Json<AnalysisConfiguration>,
    state: Data<RouteState>,
) -> Result<HttpResponse, ServiceError> {
    let analysis_configuration = state
        .db
        .send(AnalysisConfiguration {
            camera_id: path.into_inner(),
            analysis_type: analysis_configuration.analysis_type,
        })
        .await??;

    AnalysisSupervisor::from_registry()
        .send(SyncAnalysisActors {})
        .await?;

    Ok(HttpResponse::Ok().json(analysis_configuration))
}
