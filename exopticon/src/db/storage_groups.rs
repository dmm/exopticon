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

use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

use crate::{
    api::{ResourceMetadata, storage_groups::StorageGroupSpec},
    db::{
        cameras::Camera,
        video_units::{VideoFile, VideoUnit},
    },
    schema::storage_groups,
};

#[derive(Identifiable, PartialEq, Eq, Debug, Queryable, Insertable, Clone)]
#[diesel(primary_key(name))]
#[diesel(table_name = storage_groups)]
pub struct StorageGroup {
    pub name: String,
    pub display_name: String,
    pub storage_path: String,
    pub max_storage_size: i64,
}

impl From<StorageGroup> for crate::api::storage_groups::StorageGroup {
    fn from(g: StorageGroup) -> Self {
        Self {
            metadata: ResourceMetadata {
                name: g.name,
                display_name: g.display_name,
            },
            spec: StorageGroupSpec {
                storage_path: g.storage_path,
                max_storage_size: g.max_storage_size,
            },
            status: serde_json::json!({}),
        }
    }
}

pub struct StorageGroupOldFiles {
    pub storage_group_capacity: i64,
    pub storage_group_size: i64,
    pub video_units: Vec<(i64, VideoUnit, VideoFile)>,
}

impl super::Service {
    pub fn fetch_storage_group(
        &self,
        storage_group_name: &str,
    ) -> Result<crate::api::storage_groups::StorageGroup, super::Error> {
        use crate::schema::storage_groups::dsl;
        let mut conn = self.pool.get()?;

        let group = dsl::storage_groups
            .find(storage_group_name)
            .get_result::<StorageGroup>(&mut conn)?;

        Ok(group.into())
    }

    pub fn fetch_all_storage_groups(
        &self,
    ) -> Result<Vec<crate::api::storage_groups::StorageGroup>, super::Error> {
        use crate::schema::storage_groups::dsl;
        let mut conn = self.pool.get()?;

        let groups = dsl::storage_groups.load::<StorageGroup>(&mut conn)?;

        Ok(groups.into_iter().map(std::convert::Into::into).collect())
    }

    pub fn fetch_storage_group_old_units(
        &self,
        storage_group_name: &str,
        count: i64,
    ) -> Result<StorageGroupOldFiles, super::Error> {
        use crate::schema::{cameras, video_files, video_units};

        let mut conn = self.pool.get()?;

        let storage_group_capacity = storage_groups::dsl::storage_groups
            .select(storage_groups::max_storage_size)
            .filter(storage_groups::columns::name.eq(storage_group_name))
            .first::<i64>(&mut conn)?;

        let storage_group_size = video_files::table
            .select(diesel::dsl::sum(video_files::size))
            .inner_join(video_units::table.inner_join(cameras::table))
            .filter(cameras::storage_group_name.eq(storage_group_name))
            .filter(video_files::size.ne(-1))
            .first::<Option<i64>>(&mut conn)?
            .unwrap_or(0);

        let c: Vec<(Camera, (VideoUnit, VideoFile))> = cameras::table
            .inner_join(video_units::table.inner_join(video_files::table))
            .filter(cameras::storage_group_name.eq(storage_group_name))
            .filter(video_files::size.gt(-1))
            .filter(video_units::begin_time.ne(video_units::end_time))
            .order(video_units::begin_time.asc())
            .limit(count)
            .load(&mut conn)?;

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
}
