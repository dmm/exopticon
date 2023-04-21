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

use chrono::NaiveDateTime;
use diesel::{dsl::sum, ExpressionMethods, QueryDsl, RunQueryDsl};

use crate::{
    db::{
        cameras::Camera,
        video_units::{VideoFile, VideoUnit},
    },
    schema::storage_groups,
};

/// Full storage group model. Represents a full row returned from the
/// database.
#[derive(Identifiable, PartialEq, Eq, Associations, Debug, Queryable, Insertable)]
#[table_name = "storage_groups"]
pub struct StorageGroup {
    /// storage group id
    pub id: i32,
    /// storage group name
    pub name: String,
    /// full path to video storage path, e.g. /mnt/video/8/
    pub storage_path: String,
    /// maximum allowed storage size in bytes
    pub max_storage_size: i64,
    /// insertion time
    pub inserted_at: NaiveDateTime,
    /// update time
    pub updated_at: NaiveDateTime,
}

impl From<StorageGroup> for crate::api::storage_groups::StorageGroup {
    fn from(g: StorageGroup) -> Self {
        Self {
            id: g.id,
            name: g.name,
            storage_path: g.storage_path,
            max_storage_size: g.max_storage_size,
        }
    }
}

/// Full storage group model. Represents a full row returned from the
/// database.
#[derive(PartialEq, Eq, Associations, Debug, Queryable, Insertable)]
#[table_name = "storage_groups"]
pub struct CreateStorageGroup {
    /// storage group name
    pub name: String,
    /// full path to video storage path, e.g. /mnt/video/8/
    pub storage_path: String,
    /// maximum allowed storage size in bytes
    pub max_storage_size: i64,
}

/// Represents a storage group update request
#[derive(AsChangeset, Debug)]
#[table_name = "storage_groups"]
pub struct UpdateStorageGroup {
    /// if provided, updated name for storage group
    pub name: Option<String>,
    /// if provided, updated storage path for storage group
    pub storage_path: Option<String>,
    /// if provided, updated storage size for storage group
    pub max_storage_size: Option<i64>,
}

impl From<crate::api::storage_groups::UpdateStorageGroup> for UpdateStorageGroup {
    fn from(g: crate::api::storage_groups::UpdateStorageGroup) -> Self {
        Self {
            name: g.name,
            storage_path: g.storage_path,
            max_storage_size: g.max_storage_size,
        }
    }
}

/// Represents the current state of storage group, with a snapshot of
/// older video units
pub struct StorageGroupOldFiles {
    pub storage_group_capacity: i64,
    pub storage_group_size: i64,
    pub video_units: Vec<(i64, VideoUnit, VideoFile)>,
}

impl super::Service {
    pub fn create_storage_group(
        &self,
        group: crate::api::storage_groups::CreateStorageGroup,
    ) -> Result<crate::api::storage_groups::StorageGroup, super::Error> {
        match &self.pool {
            super::ServiceKind::Real(pool) => {
                use crate::schema::storage_groups::dsl;
                let conn = pool.get()?;

                let new_storage_group: StorageGroup = diesel::insert_into(storage_groups::table)
                    .values((
                        dsl::name.eq(group.name),
                        dsl::storage_path.eq(group.storage_path),
                        dsl::max_storage_size.eq(group.max_storage_size),
                    ))
                    .get_result(&conn)?;

                Ok(new_storage_group.into())
            }
            super::ServiceKind::Null(_) => todo!(),
        }
    }

    pub fn update_storage_group(
        &self,
        id: i32,
        group: crate::api::storage_groups::UpdateStorageGroup,
    ) -> Result<crate::api::storage_groups::StorageGroup, super::Error> {
        match &self.pool {
            super::ServiceKind::Real(pool) => {
                use crate::schema::storage_groups::dsl;
                let conn = pool.get()?;

                let updated_storage_group: StorageGroup =
                    diesel::update(dsl::storage_groups.filter(dsl::id.eq(id)))
                        .set::<UpdateStorageGroup>(group.into())
                        .get_result(&conn)?;

                Ok(updated_storage_group.into())
            }
            super::ServiceKind::Null(_) => todo!(),
        }
    }

    pub fn fetch_storage_group(
        &self,
        id: i32,
    ) -> Result<crate::api::storage_groups::StorageGroup, super::Error> {
        match &self.pool {
            super::ServiceKind::Real(pool) => {
                use crate::schema::storage_groups::dsl;
                let conn = pool.get()?;

                let group = dsl::storage_groups
                    .find(id)
                    .get_result::<StorageGroup>(&conn)?;

                Ok(group.into())
            }
            super::ServiceKind::Null(_) => todo!(),
        }
    }

    pub fn fetch_all_storage_groups(
        &self,
    ) -> Result<Vec<crate::api::storage_groups::StorageGroup>, super::Error> {
        match &self.pool {
            super::ServiceKind::Real(pool) => {
                use crate::schema::storage_groups::dsl;
                let conn = pool.get()?;

                let groups = dsl::storage_groups.load::<StorageGroup>(&conn)?;

                Ok(groups.into_iter().map(std::convert::Into::into).collect())
            }
            super::ServiceKind::Null(_) => todo!(),
        }
    }

    pub fn delete_storage_group(&self, sid: i32) -> Result<(), super::Error> {
        match &self.pool {
            super::ServiceKind::Real(pool) => {
                use crate::schema::storage_groups::dsl::*;
                let conn = pool.get()?;

                diesel::delete(storage_groups.filter(id.eq(sid))).execute(&conn)?;
                Ok(())
            }
            super::ServiceKind::Null(_) => todo!(),
        }
    }

    pub fn fetch_storage_group_old_units(
        &self,
        sid: i32,
        count: i64,
    ) -> Result<StorageGroupOldFiles, super::Error> {
        use crate::schema::cameras::dsl::*;
        use crate::schema::video_files::dsl::*;
        use crate::schema::video_units::dsl::*;

        match &self.pool {
            crate::db::ServiceKind::Real(pool) => {
                let conn = pool.get()?;

                let storage_group_capacity = storage_groups::dsl::storage_groups
                    .select(storage_groups::max_storage_size)
                    .filter(storage_groups::columns::id.eq(sid))
                    .first::<i64>(&conn)?;

                let storage_group_size = video_files
                    .select(sum(size))
                    .inner_join(video_units.inner_join(cameras))
                    .filter(storage_group_id.eq(sid))
                    .filter(size.ne(-1))
                    .first::<Option<i64>>(&conn)?
                    .unwrap_or(0);

                let c: Vec<(Camera, (VideoUnit, VideoFile))> = cameras
                    .inner_join(video_units.inner_join(video_files))
                    .filter(storage_group_id.eq(sid))
                    .filter(size.gt(-1))
                    .filter(begin_time.ne(end_time))
                    .order(begin_time.asc())
                    .limit(count)
                    .load(&conn)?;

                let units = c
                    .into_iter()
                    .map(|(_c, (unit, file))| (file.size.into(), unit, file))
                    .collect();

                Ok(StorageGroupOldFiles {
                    storage_group_capacity,
                    storage_group_size,
                    video_units: units,
                })
            }
            crate::db::ServiceKind::Null(_) => todo!(),
        }
    }
}
