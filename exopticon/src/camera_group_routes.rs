// We have to pass by value to satisfy the actix route interface.
#![allow(clippy::needless_pass_by_value)]

use actix_web::{web::Data, web::Json, web::Path, Error, HttpResponse};

use crate::app::RouteState;
use crate::models::{CreateCameraGroup, FetchAllCameraGroup, FetchCameraGroup, UpdateCameraGroup};

/// Route to create new camera group
pub async fn create_camera_group(
    camera_group_request: Json<CreateCameraGroup>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let db_response = state.db.send(camera_group_request.into_inner()).await?;

    match db_response {
        Ok(camera_group) => Ok(HttpResponse::Ok().json(camera_group)),
        Err(err) => Ok(HttpResponse::InternalServerError().body(err.to_string())),
    }
}

/// Route to update existing camera group
pub async fn update_camera_group(
    path: Path<i32>,
    camera_group_request: Json<UpdateCameraGroup>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let camera_group_update = UpdateCameraGroup {
        id: path.into_inner(),
        ..camera_group_request.into_inner()
    };
    let db_response = state.db.send(camera_group_update).await?;

    match db_response {
        Ok(camera_group) => Ok(HttpResponse::Ok().json(camera_group)),
        Err(err) => Ok(HttpResponse::InternalServerError().body(err.to_string())),
    }
}

/// Route to fetch camera group by id
pub async fn fetch_camera_group(
    path: Path<i32>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let db_response = state
        .db
        .send(FetchCameraGroup {
            id: path.into_inner(),
        })
        .await?;

    match db_response {
        Ok(camera_group) => Ok(HttpResponse::Ok().json(camera_group)),
        Err(err) => Ok(HttpResponse::InternalServerError().body(err.to_string())),
    }
}

/// Route to fetch all camera groups
pub async fn fetch_all_camera_groups(state: Data<RouteState>) -> Result<HttpResponse, Error> {
    let db_response = state.db.send(FetchAllCameraGroup {}).await?;

    match db_response {
        Ok(camera_groups) => Ok(HttpResponse::Ok().json(camera_groups)),
        Err(err) => Ok(HttpResponse::InternalServerError().body(err.to_string())),
    }
}
