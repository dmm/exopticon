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
use diesel::{BelongingToDsl, Connection, ExpressionMethods, QueryDsl, RunQueryDsl};

use crate::db::cameras::Camera;
use crate::schema::{video_files, video_units};

use super::uuid::Uuid;
use super::Service;

/// Full video unit model, represents entire database row
#[derive(Identifiable, Insertable, Associations, Serialize, Queryable, Clone)]
#[serde(rename_all = "camelCase")]
#[diesel(belongs_to(Camera))]
#[diesel(table_name = video_units)]
pub struct VideoUnit {
    /// id of video unit
    pub id: Uuid,
    /// id of associated camera
    pub camera_id: Uuid,
    /// begin time in UTC
    pub begin_time: DateTime<Utc>,
    /// end time in UTC
    pub end_time: DateTime<Utc>,
}

impl From<VideoUnit> for crate::api::video_units::VideoUnit {
    fn from(v: VideoUnit) -> Self {
        Self {
            id: v.id.into(),
            camera_id: v.camera_id.into(),
            begin_time: v.begin_time,
            end_time: v.end_time,
        }
    }
}

/// Represents request to create new video unit record
#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[diesel(table_name = video_units)]
pub struct CreateVideoUnit {
    /// id of video unit
    pub id: Uuid,
    /// id of associated camera
    pub camera_id: Uuid,
    /// begin time in UTC
    pub begin_time: DateTime<Utc>,
    /// end time in UTC
    pub end_time: DateTime<Utc>,
}

/// Represents request to update video unit record
#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[diesel(table_name = video_units)]
pub struct UpdateVideoUnit {
    /// id of video unit to update
    pub id: Uuid,
    /// if present, new associated camera id
    pub camera_id: Option<Uuid>,
    /// if present, new begin time, in UTC
    pub begin_time: Option<DateTime<Utc>>,
    /// if present, new end time, in UTC
    pub end_time: Option<DateTime<Utc>>,
}

/// Full video file model, represents full database row
#[derive(Queryable, Associations, Identifiable, Insertable, Serialize)]
#[serde(rename_all = "camelCase")]
#[diesel(table_name = video_files)]
#[diesel(belongs_to(VideoUnit))]
pub struct VideoFile {
    /// id of video file
    pub id: Uuid,
    /// filename of video file
    pub filename: String,
    /// size in bytes of video file
    pub size: i32,
    /// id of associated video unit
    pub video_unit_id: Uuid,
}

impl From<VideoFile> for crate::api::video_units::VideoFile {
    fn from(v: VideoFile) -> Self {
        Self {
            id: v.id.into(),
            filename: v.filename,
            size: v.size,
            video_unit_id: v.video_unit_id.into(),
        }
    }
}

/// Represents request to create new video file
#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[diesel(table_name = video_files)]
pub struct CreateVideoFile {
    /// filename for new video file
    pub filename: String,
    /// size in bytes of new video file
    pub size: i32,
    /// id of video unit to own this video file
    pub video_unit_id: Uuid,
}

/// Represents request to update video file
#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[diesel(table_name = video_files)]
pub struct UpdateVideoFile {
    /// id of video file to update
    pub id: Uuid,
    /// if present, new id of associated video unit
    pub video_unit_id: Option<Uuid>,
    /// if present, new filename
    pub filename: Option<String>,
    /// if present, new file size
    pub size: Option<i32>,
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
                .values(VideoUnit {
                    id: Uuid::now_v7().into(),
                    camera_id: video_unit.camera_id.into(),
                    begin_time: video_unit.begin_time,
                    end_time: video_unit.end_time,
                })
                .get_result::<VideoUnit>(conn)?;

            let video_file = diesel::insert_into(video_files::dsl::video_files)
                .values(VideoFile {
                    id: Uuid::now_v7(),
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
        ) = (res.0.into(), res.1.into());
        Ok(res2)
    }

    // update video unit/video file
    pub fn close_video_segment(
        &self,
        video_unit_id: Uuid,
        video_file_id: Uuid,
        end_time: DateTime<Utc>,
        file_size: i32,
    ) -> Result<VideoSegment, super::Error> {
        let mut conn = self.pool.get()?;
        let res = conn.transaction::<_, super::Error, _>(|conn| {
            let video_unit = diesel::update(
                video_units::dsl::video_units
                    .filter(crate::schema::video_units::columns::id.eq(video_unit_id)),
            )
            .set(UpdateVideoUnit {
                id: video_unit_id,
                camera_id: None,
                begin_time: None,
                end_time: Some(end_time),
            })
            .get_result::<VideoUnit>(conn)?;

            let video_file = diesel::update(
                video_files::dsl::video_files
                    .filter(crate::schema::video_files::columns::id.eq(video_file_id)),
            )
            .set(UpdateVideoFile {
                id: video_file_id,
                video_unit_id: None,
                filename: None,
                size: Some(file_size),
            })
            .get_result::<VideoFile>(conn)?;

            Ok((video_unit, video_file))
        })?;

        Ok((res.0.into(), res.1.into()))
    }

    // Fetch between video unit
    pub fn fetch_video_units_between(
        &self,
        camera_id: Uuid,
        begin_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Vec<VideoSegment>, super::Error> {
        let mut conn = self.pool.get()?;
        let res = conn.transaction::<_, super::Error, _>(|conn| {
            use crate::schema::video_units::dsl;
            let vus: Vec<VideoUnit> = dsl::video_units
                .filter(dsl::camera_id.eq(camera_id))
                .filter(dsl::begin_time.le(end_time))
                .filter(dsl::end_time.ge(begin_time))
                .order(dsl::begin_time.asc())
                .limit(999)
                .load(conn)?;

            let files: Vec<VideoFile> = VideoFile::belonging_to(&vus).load::<VideoFile>(conn)?;

            //                    let grouped_files = files.grouped_by(&vus);

            let zipped: Vec<(VideoUnit, VideoFile)> = vus.into_iter().zip(files).collect();

            Ok(zipped)
        })?;

        let res: Vec<VideoSegment> = res.into_iter().map(|v| (v.0.into(), v.1.into())).collect();
        Ok(res)
    }

    // Delete Video Units
    pub fn delete_video_unit(&self, delete_id: Uuid) -> anyhow::Result<()> {
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

        Ok(())
    }
}
