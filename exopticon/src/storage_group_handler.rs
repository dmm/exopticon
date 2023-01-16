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

use crate::errors::ServiceError;
use crate::models::{
    Camera, CreateStorageGroup, DbExecutor, FetchAllStorageGroup, FetchAllStorageGroupAndCameras,
    FetchStorageGroup, FetchStorageGroupFiles, StorageGroup, StorageGroupAndCameras,
    UpdateStorageGroup, VideoUnit,
};
use crate::schema::storage_groups::dsl::*;
use actix::{Handler, Message};
use diesel::{self, prelude::*};

/// A segment of video paired with the source camera
type CameraVideoSegment = (VideoUnit, i64);

impl Message for CreateStorageGroup {
    type Result = Result<StorageGroup, ServiceError>;
}

impl Handler<CreateStorageGroup> for DbExecutor {
    type Result = Result<StorageGroup, ServiceError>;

    fn handle(&mut self, msg: CreateStorageGroup, _: &mut Self::Context) -> Self::Result {
        use crate::schema::storage_groups::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(storage_groups)
            .values(&msg)
            .get_result(conn)
            .map_err(|_error| {
                error!("Error creating storage group!");
                ServiceError::InternalServerError
            })
    }
}

impl Message for UpdateStorageGroup {
    type Result = Result<StorageGroup, ServiceError>;
}

impl Handler<UpdateStorageGroup> for DbExecutor {
    type Result = Result<StorageGroup, ServiceError>;

    fn handle(&mut self, msg: UpdateStorageGroup, _: &mut Self::Context) -> Self::Result {
        use crate::schema::storage_groups::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();
        diesel::update(storage_groups.filter(id.eq(msg.id)))
            .set(&msg)
            .get_result(conn)
            .map_err(|_error| {
                error!("Error updating storage group");
                ServiceError::InternalServerError
            })
    }
}

impl Message for FetchStorageGroup {
    type Result = Result<StorageGroup, ServiceError>;
}

impl Handler<FetchStorageGroup> for DbExecutor {
    type Result = Result<StorageGroup, ServiceError>;

    fn handle(&mut self, msg: FetchStorageGroup, _: &mut Self::Context) -> Self::Result {
        use crate::schema::storage_groups::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let group = storage_groups
            .filter(id.eq(msg.id))
            .load::<StorageGroup>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?
            .pop();

        group.ok_or(ServiceError::NotFound)
    }
}

impl Message for FetchAllStorageGroup {
    type Result = Result<Vec<StorageGroup>, ServiceError>;
}
impl Handler<FetchAllStorageGroup> for DbExecutor {
    type Result = Result<Vec<StorageGroup>, ServiceError>;

    fn handle(&mut self, _msg: FetchAllStorageGroup, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();

        storage_groups
            .load::<StorageGroup>(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for FetchAllStorageGroupAndCameras {
    type Result = Result<Vec<StorageGroupAndCameras>, ServiceError>;
}

impl Handler<FetchAllStorageGroupAndCameras> for DbExecutor {
    type Result = Result<Vec<StorageGroupAndCameras>, ServiceError>;

    fn handle(
        &mut self,
        _msg: FetchAllStorageGroupAndCameras,
        _: &mut Self::Context,
    ) -> Self::Result {
        use crate::schema::cameras::dsl::*;
        use diesel::prelude::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let mut groups_and_cameras: Vec<StorageGroupAndCameras> = Vec::new();

        let groups = storage_groups
            .load::<StorageGroup>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        for g in groups {
            let c = cameras
                .filter(storage_group_id.eq(g.id))
                .load::<Camera>(conn)
                .map_err(|_error| ServiceError::InternalServerError)?;

            groups_and_cameras.push(StorageGroupAndCameras(g, c));
        }

        Ok(groups_and_cameras)
    }
}

impl Message for FetchStorageGroupFiles {
    type Result = Result<(i64, i64, Vec<CameraVideoSegment>), ServiceError>;
}

impl Handler<FetchStorageGroupFiles> for DbExecutor {
    type Result = Result<(i64, i64, Vec<CameraVideoSegment>), ServiceError>;

    fn handle(&mut self, msg: FetchStorageGroupFiles, _: &mut Self::Context) -> Self::Result {
        use crate::schema::cameras::dsl::*;
        use crate::schema::observation_snapshots::dsl::*;
        use crate::schema::observations::dsl::*;
        use crate::schema::storage_groups;
        use crate::schema::video_files::dsl::*;
        use crate::schema::video_units::dsl::*;
        use diesel::dsl::{any, sum};

        let conn: &PgConnection = &self.0.get().unwrap();

        let max_size = storage_groups
            .select(max_storage_size)
            .filter(storage_groups::columns::id.eq(msg.storage_group_id))
            .first(conn)
            .map_err(|error| {
                error!(
                    "failed to fetch max size of files in storage group: {}",
                    error
                );
                ServiceError::InternalServerError
            })?;

        let current_observation_snapshot_size: i64 = observation_snapshots
            .select(sum(snapshot_size))
            .inner_join(observations.inner_join(video_units.inner_join(cameras)))
            .filter(storage_group_id.eq(msg.storage_group_id))
            .first::<Option<i64>>(conn)
            .map_err(|error| {
                error!("current snapshot size error: {}", error);
                ServiceError::InternalServerError
            })?
            .unwrap_or(0i64);

        let current_size: i64 = video_files
            .select(sum(size))
            .inner_join(video_units.inner_join(cameras))
            .filter(storage_group_id.eq(msg.storage_group_id))
            .filter(size.ne(-1))
            .first::<Option<i64>>(conn)
            .map_err(|error| {
                error!("video file size error: {}", error);
                ServiceError::InternalServerError
            })?
            .unwrap_or(0i64)
            + current_observation_snapshot_size;

        let units: Vec<(Camera, VideoUnit)> = cameras
            .inner_join(video_units)
            .filter(storage_group_id.eq(msg.storage_group_id))
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
                .first::<Option<i64>>(conn)
                .map_err(|error| {
                    error!("owned video files error: {}", error);
                    ServiceError::InternalServerError
                })?
                .unwrap_or(0i64);

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
                .first::<Option<i64>>(conn)
                .map_err(|error| {
                    error!("observation snapshot size error: {}", error);
                    ServiceError::InternalServerError
                })?
                .unwrap_or(0i64);

            let video_unit_size: i64 = snap_size + file_size;

            unitgroups.push((unitpair.1.clone(), video_unit_size));
        }

        Ok((max_size, current_size, unitgroups))
    }
}
