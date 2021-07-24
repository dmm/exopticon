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
    Camera, CreateVideoFile, CreateVideoUnit, CreateVideoUnitFile, DbExecutor,
    DeleteVideoUnitFiles, FetchBetweenVideoUnit, FetchOldVideoUnitFile, FetchVideoUnit,
    Observation, UpdateVideoFile, UpdateVideoUnit, UpdateVideoUnitFile, VideoFile, VideoUnit,
};
use actix::{Handler, Message};
use diesel::{self, prelude::*};
use uuid::Uuid;

/// A segment of video
type VideoSegment = (VideoUnit, VideoFile);

/// A segment of video paired with the source camera
type CameraVideoSegment = (Camera, VideoSegment);

impl Message for CreateVideoUnit {
    type Result = Result<VideoUnit, ServiceError>;
}

impl Handler<CreateVideoUnit> for DbExecutor {
    type Result = Result<VideoUnit, ServiceError>;

    fn handle(&mut self, msg: CreateVideoUnit, _: &mut Self::Context) -> Self::Result {
        use crate::schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(video_units)
            .values(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for CreateVideoUnitFile {
    type Result = Result<(VideoUnit, VideoFile), ServiceError>;
}

impl Handler<CreateVideoUnitFile> for DbExecutor {
    type Result = Result<(VideoUnit, VideoFile), ServiceError>;
    fn handle(&mut self, msg: CreateVideoUnitFile, _: &mut Self::Context) -> Self::Result {
        use crate::schema::video_files::dsl::*;
        use crate::schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();
        // TODO: Wrap this in a transaction
        let video_unit: VideoUnit = diesel::insert_into(video_units)
            .values(CreateVideoUnit {
                id: msg.video_unit_id,
                camera_id: msg.camera_id,
                monotonic_index: msg.monotonic_index,
                begin_time: msg.begin_time,
                end_time: msg.begin_time,
            })
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        let video_file = diesel::insert_into(video_files)
            .values(CreateVideoFile {
                video_unit_id: video_unit.id,
                filename: msg.filename,
                size: -1,
            })
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        Ok((video_unit, video_file))
    }
}

impl Message for UpdateVideoUnit {
    type Result = Result<VideoUnit, ServiceError>;
}

impl Handler<UpdateVideoUnit> for DbExecutor {
    type Result = Result<VideoUnit, ServiceError>;

    fn handle(&mut self, msg: UpdateVideoUnit, _: &mut Self::Context) -> Self::Result {
        use crate::schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::update(video_units.filter(id.eq(msg.id)))
            .set(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for UpdateVideoUnitFile {
    type Result = Result<(VideoUnit, VideoFile), ServiceError>;
}

impl Handler<UpdateVideoUnitFile> for DbExecutor {
    type Result = Result<(VideoUnit, VideoFile), ServiceError>;
    fn handle(&mut self, msg: UpdateVideoUnitFile, _: &mut Self::Context) -> Self::Result {
        use crate::schema;
        use crate::schema::video_files::dsl::*;
        use crate::schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let video_unit = diesel::update(
            video_units.filter(schema::video_units::columns::id.eq(msg.video_unit_id)),
        )
        .set(UpdateVideoUnit {
            id: msg.video_unit_id,
            camera_id: None,
            monotonic_index: None,
            begin_time: None,
            end_time: Some(msg.end_time),
        })
        .get_result(conn)
        .map_err(|_error| ServiceError::InternalServerError)?;

        let video_file = diesel::update(
            video_files.filter(schema::video_files::columns::id.eq(msg.video_file_id)),
        )
        .set(UpdateVideoFile {
            id: msg.video_file_id,
            video_unit_id: None,
            filename: None,
            size: Some(msg.size),
        })
        .get_result(conn)
        .map_err(|_error| ServiceError::InternalServerError)?;

        Ok((video_unit, video_file))
    }
}
/// Type Alias for video unit handler return type
type VideoUnitTuple = (VideoUnit, Vec<VideoFile>, Vec<Observation>);

impl Message for FetchVideoUnit {
    type Result = Result<VideoUnitTuple, ServiceError>;
}

impl Handler<FetchVideoUnit> for DbExecutor {
    type Result = Result<VideoUnitTuple, ServiceError>;

    fn handle(&mut self, msg: FetchVideoUnit, _: &mut Self::Context) -> Self::Result {
        use crate::schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let vu = video_units
            .find(msg.id)
            .get_result::<VideoUnit>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        let files = crate::models::VideoFile::belonging_to(&vu)
            .load::<VideoFile>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        let observations = crate::models::Observation::belonging_to(&vu)
            .load::<Observation>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        Ok((vu, files, observations))
    }
}

impl Message for FetchBetweenVideoUnit {
    type Result = Result<Vec<VideoUnitTuple>, ServiceError>;
}

impl Handler<FetchBetweenVideoUnit> for DbExecutor {
    type Result = Result<Vec<VideoUnitTuple>, ServiceError>;

    fn handle(&mut self, msg: FetchBetweenVideoUnit, _: &mut Self::Context) -> Self::Result {
        use crate::schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let vus: Vec<VideoUnit> = video_units
            .filter(camera_id.eq(msg.camera_id))
            .filter(begin_time.le(msg.end_time.naive_utc()))
            .filter(end_time.ge(msg.begin_time.naive_utc()))
            .order(begin_time.asc())
            .limit(1000)
            .load(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        let files = crate::models::VideoFile::belonging_to(&vus)
            .load::<VideoFile>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        let grouped_files = files.grouped_by(&vus);

        let observations = crate::models::Observation::belonging_to(&vus)
            .load::<Observation>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        let grouped_observations = observations.grouped_by(&vus);

        let grouped_units = vus
            .into_iter()
            .zip(grouped_files)
            .zip(grouped_observations)
            .map(|((vu, file_group), obs_group)| (vu, file_group, obs_group))
            .collect();

        Ok(grouped_units)
    }
}

impl Message for FetchOldVideoUnitFile {
    type Result = Result<Vec<CameraVideoSegment>, ServiceError>;
}

impl Handler<FetchOldVideoUnitFile> for DbExecutor {
    type Result = Result<Vec<CameraVideoSegment>, ServiceError>;

    fn handle(&mut self, msg: FetchOldVideoUnitFile, _: &mut Self::Context) -> Self::Result {
        use crate::schema::cameras::dsl::*;
        use crate::schema::video_files::dsl::*;
        use crate::schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        cameras
            .inner_join(video_units.inner_join(video_files))
            .filter(camera_group_id.eq(msg.camera_group_id))
            .filter(size.gt(-1))
            .filter(begin_time.ne(end_time))
            .order(begin_time.asc())
            .limit(msg.count)
            .load(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for DeleteVideoUnitFiles {
    type Result = Result<(), ServiceError>;
}

impl Handler<DeleteVideoUnitFiles> for DbExecutor {
    type Result = Result<(), ServiceError>;

    fn handle(&mut self, msg: DeleteVideoUnitFiles, _: &mut Self::Context) -> Self::Result {
        use crate::schema;
        use crate::schema::event_observations::dsl::*;
        use crate::schema::events::dsl::*;
        use crate::schema::observation_snapshots::dsl::*;
        use crate::schema::observations::dsl::*;
        use crate::schema::video_files::dsl::*;
        use crate::schema::video_units::dsl::*;
        use diesel::dsl::any;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::delete(
            video_files.filter(schema::video_files::columns::id.eq(any(&msg.video_file_ids))),
        )
        .execute(conn)
        .map_err(|_error| ServiceError::InternalServerError)?;

        diesel::delete(event_observations)
            .filter(
                schema::event_observations::columns::observation_id.eq_any(
                    observations
                        .filter(
                            schema::observations::columns::video_unit_id
                                .eq(any(&msg.video_unit_ids)),
                        )
                        .select(schema::observations::columns::id),
                ),
            )
            .execute(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        // Fetch all observation snapshots that need to be deleted
        let snaps: Vec<String> = observation_snapshots
            .inner_join(observations)
            .filter(schema::observations::columns::video_unit_id.eq(any(&msg.video_unit_ids)))
            .select(snapshot_path)
            .load(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        for s in snaps {
            if std::fs::remove_file(&s).is_err() {
                error!("Failed to delete file: {}", &s);
            }
        }

        diesel::delete(observation_snapshots)
            .filter(
                schema::observation_snapshots::columns::observation_id.eq_any(
                    observations
                        .filter(
                            schema::observations::columns::video_unit_id
                                .eq(any(&msg.video_unit_ids)),
                        )
                        .select(schema::observations::columns::id),
                ),
            )
            .execute(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        // Fetch all events to be deleted
        let old_events = events
            .left_outer_join(schema::event_observations::table)
            .inner_join(schema::observations::table)
            .select(schema::events::columns::id)
            .filter(schema::event_observations::columns::observation_id.is_null())
            .load::<Uuid>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        diesel::delete(events.filter(schema::events::columns::id.eq(any(&old_events))))
            .execute(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        diesel::delete(
            observations
                .filter(schema::observations::columns::video_unit_id.eq(any(&msg.video_unit_ids))),
        )
        .execute(conn)
        .map_err(|_error| ServiceError::InternalServerError)?;

        diesel::delete(
            video_units.filter(schema::video_units::columns::id.eq(any(msg.video_unit_ids))),
        )
        .execute(conn)
        .map_err(|_error| ServiceError::InternalServerError)?;

        Ok(())
    }
}
