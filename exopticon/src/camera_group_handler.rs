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

use crate::errors::ServiceError;
use crate::models::{
    Camera, CameraGroup, CameraGroupAndCameras, CreateCameraGroup, DbExecutor, FetchAllCameraGroup,
    FetchAllCameraGroupAndCameras, FetchCameraGroup, FetchCameraGroupFiles, UpdateCameraGroup,
    VideoUnit,
};
use crate::schema::camera_groups::dsl::*;
use actix::{Handler, Message};
use diesel::{self, prelude::*};

/// A segment of video paired with the source camera
type CameraVideoSegment = (VideoUnit, i64);

impl Message for CreateCameraGroup {
    type Result = Result<CameraGroup, ServiceError>;
}

impl Handler<CreateCameraGroup> for DbExecutor {
    type Result = Result<CameraGroup, ServiceError>;

    fn handle(&mut self, msg: CreateCameraGroup, _: &mut Self::Context) -> Self::Result {
        use crate::schema::camera_groups::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(camera_groups)
            .values(&msg)
            .get_result(conn)
            .map_err(|_error| {
                error!("Error creating camera group!");
                ServiceError::InternalServerError
            })
    }
}

impl Message for UpdateCameraGroup {
    type Result = Result<CameraGroup, ServiceError>;
}

impl Handler<UpdateCameraGroup> for DbExecutor {
    type Result = Result<CameraGroup, ServiceError>;

    fn handle(&mut self, msg: UpdateCameraGroup, _: &mut Self::Context) -> Self::Result {
        use crate::schema::camera_groups::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();
        diesel::update(camera_groups.filter(id.eq(msg.id)))
            .set(&msg)
            .get_result(conn)
            .map_err(|_error| {
                error!("Error updating camera group");
                ServiceError::InternalServerError
            })
    }
}

impl Message for FetchCameraGroup {
    type Result = Result<CameraGroup, ServiceError>;
}

impl Handler<FetchCameraGroup> for DbExecutor {
    type Result = Result<CameraGroup, ServiceError>;

    fn handle(&mut self, msg: FetchCameraGroup, _: &mut Self::Context) -> Self::Result {
        use crate::schema::camera_groups::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let group = camera_groups
            .filter(id.eq(msg.id))
            .load::<CameraGroup>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?
            .pop();

        match group {
            None => Err(ServiceError::NotFound),
            Some(g) => Ok(g),
        }
    }
}

impl Message for FetchAllCameraGroup {
    type Result = Result<Vec<CameraGroup>, ServiceError>;
}
impl Handler<FetchAllCameraGroup> for DbExecutor {
    type Result = Result<Vec<CameraGroup>, ServiceError>;

    fn handle(&mut self, _msg: FetchAllCameraGroup, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();

        camera_groups
            .load::<CameraGroup>(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for FetchAllCameraGroupAndCameras {
    type Result = Result<Vec<CameraGroupAndCameras>, ServiceError>;
}

impl Handler<FetchAllCameraGroupAndCameras> for DbExecutor {
    type Result = Result<Vec<CameraGroupAndCameras>, ServiceError>;

    fn handle(
        &mut self,
        _msg: FetchAllCameraGroupAndCameras,
        _: &mut Self::Context,
    ) -> Self::Result {
        use crate::schema::cameras::dsl::*;
        use diesel::prelude::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let mut groups_and_cameras: Vec<CameraGroupAndCameras> = Vec::new();

        let groups = camera_groups
            .load::<CameraGroup>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        for g in groups {
            let c = cameras
                .filter(camera_group_id.eq(g.id))
                .load::<Camera>(conn)
                .map_err(|_error| ServiceError::InternalServerError)?;

            groups_and_cameras.push(CameraGroupAndCameras { 0: g, 1: c });
        }

        Ok(groups_and_cameras)
    }
}

impl Message for FetchCameraGroupFiles {
    type Result = Result<(i64, i64, Vec<CameraVideoSegment>), ServiceError>;
}

impl Handler<FetchCameraGroupFiles> for DbExecutor {
    type Result = Result<(i64, i64, Vec<CameraVideoSegment>), ServiceError>;

    fn handle(&mut self, msg: FetchCameraGroupFiles, _: &mut Self::Context) -> Self::Result {
        use crate::schema::camera_groups;
        use crate::schema::cameras::dsl::*;
        use crate::schema::observation_snapshots::dsl::*;
        use crate::schema::observations::dsl::*;
        use crate::schema::video_files::dsl::*;
        use crate::schema::video_units::dsl::*;
        use diesel::dsl::{any, sum};

        let conn: &PgConnection = &self.0.get().unwrap();

        let max_size = camera_groups
            .select(max_storage_size)
            .filter(camera_groups::columns::id.eq(msg.camera_group_id))
            .first(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        let current_observation_snapshot_size = observation_snapshots
            .select(sum(snapshot_size))
            .inner_join(observations.inner_join(video_units.inner_join(cameras)))
            .filter(camera_group_id.eq(msg.camera_group_id))
            .first(conn)
            .map(|result| match result {
                Some(val) => val,
                None => 0,
            })
            .map_err(|error| {
                error!("current snapshot size error: {}", error);
                ServiceError::InternalServerError
            })?;

        let current_size = video_files
            .select(sum(size))
            .inner_join(video_units.inner_join(cameras))
            .filter(camera_group_id.eq(msg.camera_group_id))
            .filter(size.ne(-1))
            .first(conn)
            .map(|result| match result {
                Some(val) => val,
                None => 0,
            })
            .map_err(|error| {
                error!("video file size error: {}", error);
                ServiceError::InternalServerError
            })?
            + current_observation_snapshot_size;

        let units: Vec<(Camera, VideoUnit)> = cameras
            .inner_join(video_units)
            .filter(camera_group_id.eq(msg.camera_group_id))
            .order(begin_time.asc())
            .limit(msg.count)
            .load(conn)
            .map_err(|error| {
                error!("video unit error: {}", error);
                ServiceError::InternalServerError
            })?;

        let mut unitgroups = Vec::new();

        for unitpair in &units {
            let file_size: i64 = video_files
                .filter(crate::schema::video_files::columns::video_unit_id.eq(unitpair.1.id))
                .select(sum(size))
                .first(conn)
                .map(|result| match result {
                    Some(val) => val,
                    None => 0,
                })
                .map_err(|error| {
                    error!("owned video files error: {}", error);
                    ServiceError::InternalServerError
                })?;

            let obs: Vec<i64> = observations
                .select(crate::schema::observations::columns::id)
                .filter(crate::schema::observations::columns::video_unit_id.eq(unitpair.1.id))
                .load(conn)
                .map_err(|error| {
                    error!("owned observations error: {}", error);
                    ServiceError::InternalServerError
                })?;

            let snap_size: i64 = observation_snapshots
                .select(sum(snapshot_size))
                .filter(observation_id.eq(any(&obs)))
                .first(conn)
                .map(|result| match result {
                    Some(val) => val,
                    None => 0,
                })
                .map_err(|error| {
                    error!("observation snapshot size error: {}", error);
                    ServiceError::InternalServerError
                })?;

            let video_unit_size: i64 = snap_size + file_size as i64;

            unitgroups.push((unitpair.1.clone(), video_unit_size));
        }

        Ok((max_size, current_size, unitgroups))
    }
}
