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

use actix_web::web::{self, Path};
use actix_web::web::{block, Data};
use actix_web::{web::Json, HttpResponse};

use crate::db::DbService;

use super::UserError;

// Route Models

/// `CameraGroup` api resource
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
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
    camera_group_request: Json<CreateCameraGroup>,
    data: Data<DbService>,
) -> Result<HttpResponse, UserError> {
    let db = data.into_inner();
    let req = camera_group_request.into_inner();
    let camera_group = block(move || db.create_camera_group(&req.name, req.members)).await??;
    Ok(HttpResponse::Ok().json(camera_group))
}

pub async fn delete(id: Path<i32>, data: Data<DbService>) -> Result<HttpResponse, UserError> {
    let db = data.into_inner();
    db.delete_camera_group(id.into_inner())?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn fetch(id: Path<i32>, data: Data<DbService>) -> Result<HttpResponse, UserError> {
    let db = data.into_inner();
    let camera_group = block(move || db.fetch_camera_group(id.into_inner())).await??;
    Ok(HttpResponse::Ok().json(camera_group))
}

pub async fn fetch_all(data: Data<DbService>) -> Result<HttpResponse, UserError> {
    let db = data.into_inner();
    let camera_groups = block(move || db.fetch_all_camera_groups()).await??;
    Ok(HttpResponse::Ok().json(camera_groups))
}

/// Route configuration for `CameraGroup`s
pub fn config(cfg: &mut web::ServiceConfig) {
    println!("Configuring camera groups!");
    cfg.service(
        web::resource("/camera_groups")
            .route(web::get().to(fetch_all))
            .route(web::post().to(create))
            .route(web::delete().to(delete)),
    );
    cfg.service(web::resource("/camera_groups/{id}").route(web::get().to(fetch)));
}

#[cfg(test)]
mod tests {
    use actix_web::body::to_bytes;
    use actix_web::http::{self};

    use crate::db::{DbService, NullDb};

    use super::*;

    #[actix_web::test]
    async fn test_fetch_nonexistant_camera_group() {
        // Arrange
        let db = Data::new(DbService::new_null(None));

        // Act
        let resp = fetch_all(db).await.unwrap();

        // Assert
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_web::test]
    async fn fetch_camera_groups_returns_all() {
        // Arrange
        let camera_groups = vec![
            CameraGroup {
                id: 1,
                name: "group1".to_string(),
                members: Vec::new(),
            },
            CameraGroup {
                id: 2,
                name: "group2".to_string(),
                members: Vec::new(),
            },
        ];
        let db = Data::new(DbService::new_null(Some(NullDb::new(
            camera_groups.clone(),
        ))));

        // Act
        let res = fetch_all(db).await.unwrap();

        // Assert
        assert_eq!(res.status(), http::StatusCode::OK);
        let groups: Vec<CameraGroup> =
            serde_json::from_slice(&to_bytes(res.into_body()).await.unwrap()).unwrap();
        assert_eq!(camera_groups.len(), groups.len());
    }

    #[actix_web::test]
    async fn delete_camera_group() {
        // Arrange
        let camera_groups = vec![
            CameraGroup {
                id: 1,
                name: "group1".to_string(),
                members: Vec::new(),
            },
            CameraGroup {
                id: 2,
                name: "group2".to_string(),
                members: Vec::new(),
            },
        ];
        let db = DbService::new_null(Some(NullDb::new(camera_groups.clone())));

        // Act
        delete(Path::from(1), Data::new(db.clone())).await.unwrap();
        let res = fetch_all(Data::new(db)).await.unwrap();

        // Assert
        assert_eq!(res.status(), http::StatusCode::OK);
        let groups: Vec<CameraGroup> =
            serde_json::from_slice(&to_bytes(res.into_body()).await.unwrap()).unwrap();
        assert_eq!(camera_groups.len() - 1, groups.len());
    }

    #[actix_web::test]
    async fn delete_nonexistant_camera_group() {
        // Arrange
        let camera_groups = vec![
            CameraGroup {
                id: 1,
                name: "group1".to_string(),
                members: Vec::new(),
            },
            CameraGroup {
                id: 2,
                name: "group2".to_string(),
                members: Vec::new(),
            },
        ];
        let db = DbService::new_null(Some(NullDb::new(camera_groups.clone())));

        // Act
        let del_res = delete(Path::from(3), Data::new(db.clone())).await;
        let res = fetch_all(Data::new(db)).await.unwrap();

        // Assert
        if let Err(super::UserError::NotFound) = del_res {
        } else {
            panic!("Expected NotFound!");
        }
        assert_eq!(res.status(), http::StatusCode::OK);
        let groups: Vec<CameraGroup> =
            serde_json::from_slice(&to_bytes(res.into_body()).await.unwrap()).unwrap();
        assert_eq!(camera_groups.len(), groups.len());
    }
}
