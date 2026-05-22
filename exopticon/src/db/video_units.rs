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

use chrono::{DateTime, Utc};
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl};

use crate::schema::{video_files, video_units};

use super::{Service, datetime_to_micros, micros_to_datetime};

/// Full video unit model, represents entire database row
#[derive(Identifiable, Serialize, Queryable, Clone)]
#[serde(rename_all = "camelCase")]
#[diesel(table_name = video_units)]
pub struct VideoUnit {
    /// id of video unit
    pub id: i64,
    /// name of associated camera
    pub camera_name: String,
    /// begin time in UTC epoch microseconds
    pub begin_time_us: i64,
    /// end time in UTC epoch microseconds
    pub end_time_us: i64,
}

impl TryFrom<VideoUnit> for crate::api::video_units::VideoUnit {
    type Error = super::Error;

    fn try_from(v: VideoUnit) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.id,
            camera_name: v.camera_name,
            begin_time: micros_to_datetime("video_units.begin_time_us", v.begin_time_us)?,
            end_time: micros_to_datetime("video_units.end_time_us", v.end_time_us)?,
        })
    }
}

/// Represents request to create new video unit record
#[derive(Debug, Insertable)]
#[diesel(table_name = video_units)]
struct NewVideoUnit {
    /// name of associated camera
    camera_name: String,
    /// begin time in UTC epoch microseconds
    begin_time_us: i64,
    /// end time in UTC epoch microseconds
    end_time_us: i64,
}

/// Full video file model, represents full database row
#[derive(Queryable, Associations, Identifiable, Insertable, Serialize)]
#[serde(rename_all = "camelCase")]
#[diesel(table_name = video_files)]
#[diesel(belongs_to(VideoUnit))]
pub struct VideoFile {
    /// id of video file
    pub id: i64,
    /// filename of video file
    pub filename: String,
    /// size in bytes of video file
    pub size: i32,
    /// id of associated video unit
    pub video_unit_id: i64,
}

impl From<VideoFile> for crate::api::video_units::VideoFile {
    fn from(v: VideoFile) -> Self {
        Self {
            id: v.id,
            filename: v.filename,
            size: v.size,
            video_unit_id: v.video_unit_id,
        }
    }
}

/// Represents request to create new video file
#[derive(Debug, Insertable)]
#[diesel(table_name = video_files)]
struct NewVideoFile {
    /// filename for new video file
    filename: String,
    /// size in bytes of new video file
    size: i32,
    /// id of video unit to own this video file
    video_unit_id: i64,
}

type VideoSegment = (
    crate::api::video_units::VideoUnit,
    crate::api::video_units::VideoFile,
);

impl Service {
    // create VideoSegment
    pub fn create_video_segment(
        &self,
        video_unit: &crate::api::video_units::CreateVideoUnit,
        video_file: crate::api::video_units::CreateVideoFile,
    ) -> Result<VideoSegment, super::Error> {
        let mut conn = self.pool.get()?;
        let res: (VideoUnit, VideoFile) = conn.transaction::<_, super::Error, _>(|conn| {
            let video_unit = diesel::insert_into(video_units::dsl::video_units)
                .values(NewVideoUnit {
                    camera_name: video_unit.camera_name.clone(),
                    begin_time_us: datetime_to_micros(video_unit.begin_time),
                    end_time_us: datetime_to_micros(video_unit.end_time),
                })
                .get_result::<VideoUnit>(conn)?;

            let video_file = diesel::insert_into(video_files::dsl::video_files)
                .values(NewVideoFile {
                    filename: video_file.filename,
                    size: video_file.size,
                    video_unit_id: video_unit.id,
                })
                .get_result(conn)?;
            Ok((video_unit, video_file))
        })?;

        let res2: (
            crate::api::video_units::VideoUnit,
            crate::api::video_units::VideoFile,
        ) = (res.0.try_into()?, res.1.into());
        Ok(res2)
    }

    // update video unit/video file
    pub fn close_video_segment(
        &self,
        video_unit_id: i64,
        video_file_id: i64,
        end_time: DateTime<Utc>,
        file_size: i32,
    ) -> Result<VideoSegment, super::Error> {
        let mut conn = self.pool.get()?;
        let res = conn.transaction::<_, super::Error, _>(|conn| {
            let video_unit = diesel::update(
                video_units::dsl::video_units
                    .filter(crate::schema::video_units::columns::id.eq(video_unit_id)),
            )
            .set(crate::schema::video_units::columns::end_time_us.eq(datetime_to_micros(end_time)))
            .get_result::<VideoUnit>(conn)?;

            let video_file = diesel::update(
                video_files::dsl::video_files
                    .filter(crate::schema::video_files::columns::id.eq(video_file_id)),
            )
            .set(crate::schema::video_files::columns::size.eq(file_size))
            .get_result::<VideoFile>(conn)?;

            Ok((video_unit, video_file))
        })?;

        Ok((res.0.try_into()?, res.1.into()))
    }

    // Fetch between video unit
    pub fn fetch_video_units_between(
        &self,
        camera_name: &str,
        begin_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<VideoSegment>, super::Error> {
        let mut conn = self.pool.get()?;
        let res = conn.transaction::<_, super::Error, _>(|conn| {
            use crate::schema::video_units::dsl;
            let segments: Vec<(VideoUnit, VideoFile)> = dsl::video_units
                .inner_join(video_files::table)
                .filter(dsl::camera_name.eq(camera_name))
                .filter(dsl::begin_time_us.le(datetime_to_micros(end_time)))
                .filter(dsl::end_time_us.ge(datetime_to_micros(begin_time)))
                .order(dsl::begin_time_us.asc())
                .limit(999)
                .load(conn)?;

            Ok(segments)
        })?;

        res.into_iter()
            .map(|v| Ok((v.0.try_into()?, v.1.into())))
            .collect()
    }

    // Delete Video Units
    pub fn delete_video_unit(&self, delete_id: i64) -> anyhow::Result<()> {
        use crate::schema;
        use crate::schema::video_files::dsl::*;
        use crate::schema::video_units::dsl::*;

        let mut conn = self.pool.get()?;

        // Delete VideoFiles associated with VideoUnit

        // fetch video files to be deleted
        let files: Vec<String> = video_files
            .inner_join(video_units)
            .filter(schema::video_files::columns::video_unit_id.eq(&delete_id))
            .select(filename)
            .load(&mut conn)?;

        for f in files {
            debug!("Deleting file: {}", f);
            match std::fs::remove_file(&f) {
                Ok(()) => {}
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::NotFound {
                        // this is arguably a non-error error
                        error!("Failed to delete file because it is missing: {}", f);
                    } else {
                        error!("Failed to delete file for other reasons... {}", err);
                    }
                }
            }
        }

        // delete video files owned by VideoUnit
        diesel::delete(
            video_files.filter(schema::video_files::columns::video_unit_id.eq(delete_id)),
        )
        .execute(&mut conn)?;

        diesel::delete(video_units.filter(schema::video_units::columns::id.eq(delete_id)))
            .execute(&mut conn)?;

        Ok(())
    }
}
