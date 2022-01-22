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

use std::fs;

use crate::errors::ServiceError;
use crate::models::{
    CreateEvent, CreateObservationSnapshot, CreateObservations, DbExecutor, Event, EventFile,
    EventModel, EventObservation, FetchEvent, FetchObservation, FetchObservationSnapshot,
    FetchObservations, FetchObservationsByVideoUnit, GetEventFile, Observation,
    ObservationSnapshot, QueryEvents, VideoUnit,
};
use crate::observation_routes::get_snapshot;

use actix::{Handler, Message};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use diesel::{
    self,
    prelude::*,
    sql_types::{BigInt, Text, Timestamp},
};

impl Message for CreateObservations {
    type Result = Result<Vec<Observation>, ServiceError>;
}

impl Handler<CreateObservations> for DbExecutor {
    type Result = Result<Vec<Observation>, ServiceError>;

    fn handle(&mut self, msg: CreateObservations, _: &mut Self::Context) -> Self::Result {
        use crate::schema::observations::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(observations)
            .values(&msg.observations)
            .get_results(conn)
            .map_err(|error| {
                error!("CreateObservations error: {}", error);
                ServiceError::InternalServerError
            })
    }
}

impl Message for FetchObservation {
    type Result = Result<Observation, ServiceError>;
}

impl Handler<FetchObservation> for DbExecutor {
    type Result = Result<Observation, ServiceError>;

    fn handle(&mut self, msg: FetchObservation, _: &mut Self::Context) -> Self::Result {
        use crate::schema::observations::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();
        let o = observations
            .find(msg.id)
            .get_result::<Observation>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        Ok(o)
    }
}

impl Message for FetchObservations {
    type Result = Result<Vec<(Observation, VideoUnit)>, ServiceError>;
}

impl Handler<FetchObservations> for DbExecutor {
    type Result = Result<Vec<(Observation, VideoUnit)>, ServiceError>;

    fn handle(&mut self, msg: FetchObservations, _: &mut Self::Context) -> Self::Result {
        use crate::schema::observations::dsl::*;
        use crate::schema::video_units::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();
        observations
            .inner_join(video_units)
            .filter(camera_id.eq(msg.camera_id))
            .filter(begin_time.le(msg.end_time.naive_utc()))
            .filter(end_time.ge(msg.begin_time.naive_utc()))
            .order((begin_time.asc(), frame_offset.asc()))
            .limit(10000)
            .load(conn)
            .map_err(|error| {
                error!("FetchObservations error: {}", error);
                ServiceError::InternalServerError
            })
    }
}

impl Message for FetchObservationsByVideoUnit {
    type Result = Result<Vec<Observation>, ServiceError>;
}

impl Handler<FetchObservationsByVideoUnit> for DbExecutor {
    type Result = Result<Vec<Observation>, ServiceError>;

    fn handle(&mut self, msg: FetchObservationsByVideoUnit, _: &mut Self::Context) -> Self::Result {
        use crate::schema::observations::dsl::*;

        let conn: &PgConnection = &self.0.get().unwrap();
        observations
            .filter(video_unit_id.eq(msg.video_unit_id))
            .order(frame_offset.asc())
            .limit(1000)
            .load(conn)
            .map_err(|error| {
                error!("FetchObservationsByVideoUnit error: {}", error);
                ServiceError::InternalServerError
            })
    }
}

#[derive(Debug, QueryableByName)]
struct EventInterval {
    #[sql_type = "Timestamp"]
    pub begin_time: NaiveDateTime,
    #[sql_type = "Timestamp"]
    pub end_time: NaiveDateTime,
}

impl Message for CreateEvent {
    type Result = Result<EventModel, ServiceError>;
}

impl Handler<CreateEvent> for DbExecutor {
    type Result = Result<EventModel, ServiceError>;

    fn handle(&mut self, msg: CreateEvent, _: &mut Self::Context) -> Self::Result {
        use crate::schema::event_observations::dsl::*;
        use crate::schema::events::dsl::*;

        let eid = msg.id;
        let eid2 = msg.id;
        let msg2 = msg.clone();
        let msg3 = msg.clone();

        let new_event_observations: Vec<EventObservation> = msg
            .observations
            .into_iter()
            .map(|oid| EventObservation {
                event_id: eid,
                observation_id: oid,
            })
            .collect();

        let query = r#"
            SELECT MIN(begin_time + (frame_offset * interval '1 microsecond')) as begin_time,
                   MAX(begin_time + (frame_offset * interval '1 microsecond')) as end_time
            FROM observations obs
            INNER JOIN video_units as vu
              ON obs.video_unit_id = vu.id
            WHERE obs.id = ANY($1)
            "#;

        let conn: &PgConnection = &self.0.get().unwrap();

        conn.transaction(|| {
            let event_interval: EventInterval = diesel::sql_query(query)
                .bind::<diesel::sql_types::Array<diesel::sql_types::BigInt>, _>(msg2.observations)
                .get_result(conn)
                .map_err(|error| {
                    error!("Failed to fetch event interval {}", error);
                    ServiceError::InternalServerError
                })?;

            let new_event = Event {
                id: msg2.id,
                tag: msg2.tag,
                camera_id: msg2.camera_id,
                begin_time: DateTime::from_utc(event_interval.begin_time, Utc),
                end_time: DateTime::from_utc(event_interval.end_time, Utc),
                display_observation_id: msg2.display_observation_id,
            };

            diesel::insert_into(events)
                .values(&new_event)
                .on_conflict(crate::schema::events::dsl::id)
                .do_update()
                .set(&new_event)
                .execute(conn)
                .map_err(|error| {
                    error!("Failed to upsert event: {}", error);
                    ServiceError::InternalServerError
                })?;

            diesel::delete(event_observations.filter(event_id.eq(eid2)))
                .execute(conn)
                .map_err(|error| {
                    error!("Failed to delete event observations: {}", error);
                    ServiceError::InternalServerError
                })?;

            diesel::insert_into(event_observations)
                .values(&new_event_observations)
                .execute(conn)
                .map_err(|error| {
                    error!("Failed to insert new observations: {}", error);

                    ServiceError::InternalServerError
                })?;

            let event_model = EventModel {
                id: msg3.id,
                tag: msg3.tag,
                camera_id: msg3.camera_id,
                begin_time: DateTime::from_utc(event_interval.begin_time, Utc),
                end_time: DateTime::from_utc(event_interval.end_time, Utc),
                observations: msg3.observations,
                display_observation_id: msg3.display_observation_id,
            };

            Ok(event_model)
        })
    }
}

impl Message for FetchEvent {
    type Result = Result<EventModel, ServiceError>;
}

impl Handler<FetchEvent> for DbExecutor {
    type Result = Result<EventModel, ServiceError>;

    fn handle(&mut self, msg: FetchEvent, _: &mut Self::Context) -> Self::Result {
        use crate::schema::event_observations::dsl::*;
        use crate::schema::events::dsl::*;

        let conn: &PgConnection = &self.0.get().unwrap();
        conn.transaction(|| {
            let event1: Event = events
                .find(msg.event_id)
                .get_result(conn)
                .map_err(|_error| ServiceError::InternalServerError)?;

            let obs = event_observations
                .select(crate::schema::event_observations::dsl::observation_id)
                .filter(event_id.eq(msg.event_id))
                .load::<i64>(conn)?;
            Ok(EventModel {
                id: event1.id,
                tag: event1.tag.clone(),
                camera_id: event1.camera_id,
                begin_time: event1.begin_time,
                end_time: event1.end_time,
                observations: obs,
                display_observation_id: event1.display_observation_id,
            })
        })
    }
}

impl Message for QueryEvents {
    type Result = Result<Vec<EventModel>, ServiceError>;
}

impl Handler<QueryEvents> for DbExecutor {
    type Result = Result<Vec<EventModel>, ServiceError>;

    fn handle(&mut self, msg: QueryEvents, _: &mut Self::Context) -> Self::Result {
        use crate::schema::event_observations::dsl::*;
        use crate::schema::events::dsl::*;

        let page_size = 100;

        let now = Utc::now();
        let start = match now.checked_sub_signed(Duration::hours(12)) {
            Some(s) => s,
            None => return Err(ServiceError::InternalServerError),
        };

        let conn: &PgConnection = &self.0.get().unwrap();
        conn.transaction(|| {
            let mut query = events.into_boxed();

            if let Some(beg_time) = msg.begin_time {
                query = query.filter(begin_time.gt(beg_time));

                if let Some(endo_time) = msg.end_time {
                    query = query.filter(end_time.lt(endo_time));
                }
            } else {
                query = query.filter(begin_time.gt(&start));
            }

            if let Some(page) = msg.page {
                query = query.offset(page * page_size);
            }

            let event1: Vec<Event> = query
                .order(begin_time.desc())
                .limit(page_size)
                .load(conn)
                .map_err(|_error| ServiceError::InternalServerError)?;

            let mut event_models = Vec::new();
            for ev in &event1 {
                let obs = event_observations
                    .select(crate::schema::event_observations::dsl::observation_id)
                    .filter(event_id.eq(ev.id))
                    .load::<i64>(conn)?;

                event_models.push(EventModel {
                    id: ev.id,
                    tag: ev.tag.clone(),
                    camera_id: ev.camera_id,
                    begin_time: ev.begin_time,
                    end_time: ev.end_time,
                    observations: obs,
                    display_observation_id: ev.display_observation_id,
                });
            }

            Ok(event_models)
        })
    }
}

#[derive(Debug, QueryableByName)]
struct EventSlice {
    #[sql_type = "Text"]
    pub filename: String,
    #[sql_type = "BigInt"]
    pub start_offset: i64,
    #[sql_type = "BigInt"]
    pub end_offset: i64,
}

impl Message for GetEventFile {
    type Result = Result<EventFile, ServiceError>;
}

impl Handler<GetEventFile> for DbExecutor {
    type Result = Result<EventFile, ServiceError>;

    fn handle(&mut self, msg: GetEventFile, _: &mut Self::Context) -> Self::Result {
        let query = r#"
            SELECT vf.filename AS filename,
            MIN(ob.frame_offset) AS start_offset,
            MAX(ob.frame_offset) AS end_offset
            FROM event_observations eo
            INNER JOIN observations ob
              ON eo.observation_id = ob.id
            INNER JOIN video_units vu
              ON ob.video_unit_id = vu.id
            INNER JOIN video_files vf
              ON vf.video_unit_id = vu.id
            WHERE eo.event_id = $1
            GROUP BY vu.id, vf.filename
            ORDER BY vu.begin_time asc
            "#;

        let conn: &PgConnection = &self.0.get().unwrap();
        conn.transaction(|| {
            let event_files: Vec<EventSlice> = diesel::sql_query(query)
                .bind::<diesel::pg::types::sql_types::Uuid, _>(msg.event_id)
                .load(conn)
                .map_err(|error| {
                    error!("Failed to fetch event files {}", error);
                    ServiceError::InternalServerError
                })?;

            Ok(EventFile {
                files: event_files
                    .iter()
                    .map(|e| (e.filename.clone(), e.start_offset, e.end_offset))
                    .collect(),
            })
        })
    }
}

impl Message for CreateObservationSnapshot {
    type Result = Result<ObservationSnapshot, ServiceError>;
}

impl Handler<CreateObservationSnapshot> for DbExecutor {
    type Result = Result<ObservationSnapshot, ServiceError>;

    #[allow(clippy::too_many_lines)]
    fn handle(&mut self, msg: CreateObservationSnapshot, _: &mut Self::Context) -> Self::Result {
        use crate::schema::observation_snapshots::dsl::*;

        let conn: &PgConnection = &self.0.get().unwrap();
        conn.transaction(|| {
            let new_snapshot_path = {
                use crate::schema::camera_groups::dsl::*;
                use crate::schema::cameras::dsl::*;
                use crate::schema::observations::dsl::*;
                use crate::schema::video_units::dsl::*;

                let video_storage_path = observations
                    .inner_join(video_units.inner_join(cameras.inner_join(camera_groups)))
                    .select(crate::schema::camera_groups::dsl::storage_path)
                    .filter(crate::schema::observations::dsl::id.eq(msg.observation_id))
                    .first::<String>(conn)?;

                format!(
                    "{}/snapshots/{}.jpg",
                    video_storage_path, msg.observation_id
                )
            };

            {
                use crate::schema::camera_groups::dsl::*;
                use crate::schema::cameras::dsl::*;
                use crate::schema::observations::dsl::*;
                use crate::schema::video_files::dsl::*;
                use crate::schema::video_units::dsl::*;

                let snapshot = ObservationSnapshot {
                    observation_id: msg.observation_id,
                    snapshot_path: "".to_string(),
                    snapshot_size: 0,
                };

                let rows_inserted = diesel::insert_into(observation_snapshots)
                    .values(snapshot)
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .map_err(|error| {
                        error!("Failed to insert empty observaton_snapshot: {}", error);
                        ServiceError::InternalServerError
                    })?;

                if 0 == rows_inserted {
                    // The snapshot was already inserted
                    use crate::schema::observation_snapshots::dsl::*;
                    info!(
                        "Tried to create already created snapshot {}",
                        msg.observation_id
                    );
                    let snapshot = observation_snapshots
                        .filter(observation_id.eq(msg.observation_id))
                        .first::<ObservationSnapshot>(conn)?;
                    return Ok(snapshot);
                }

                let camera_storage_path = observations
                    .inner_join(video_units.inner_join(cameras.inner_join(camera_groups)))
                    .select(crate::schema::camera_groups::dsl::storage_path)
                    .filter(crate::schema::observations::dsl::id.eq(msg.observation_id))
                    .first::<String>(conn)?;

                let file_details = observations
                    .inner_join(video_units.inner_join(video_files))
                    .select((
                        crate::schema::observations::frame_offset,
                        crate::schema::video_files::filename,
                    ))
                    .filter(crate::schema::observations::dsl::id.eq(msg.observation_id))
                    .first::<(i64, String)>(conn)?;

                if fs::create_dir(format!("{}/snapshots/", camera_storage_path)).is_err() {
                    // ignoring error
                }

                let path = format!(
                    "{}/snapshots/{}.jpg",
                    camera_storage_path, msg.observation_id
                );
                let offset: u64 = conv::ValueFrom::value_from(file_details.0).map_err(|e| {
                    error!("Failed to convert offset: {}", e);
                    ServiceError::InternalServerError
                })?;
                if get_snapshot(&file_details.1, &path, offset).is_err() {
                    error!("Failed to create snapshot!");
                    return Err(ServiceError::InternalServerError);
                }
            }

            let x = match fs::metadata(&new_snapshot_path) {
                Ok(s) => s.len(),
                Err(e) => {
                    error!("Failed to find length of snapshot: {}", e);
                    return Err(ServiceError::InternalServerError);
                }
            };

            let snapshot_file_size: i32 = conv::ValueFrom::value_from(x).map_err(|e| {
                error!("Failed to convert snapshot size: {}", e);
                ServiceError::InternalServerError
            })?;

            diesel::update(observation_snapshots.filter(observation_id.eq(msg.observation_id)))
                .set((
                    snapshot_path.eq(new_snapshot_path.clone()),
                    snapshot_size.eq(snapshot_file_size),
                ))
                .execute(conn)
                .map_err(|error| {
                    error!("Failed to update observaton_snapshot: {}", error);
                    ServiceError::InternalServerError
                })?;

            let snapshot = ObservationSnapshot {
                observation_id: msg.observation_id,
                snapshot_path: new_snapshot_path,
                snapshot_size: snapshot_file_size,
            };

            Ok(snapshot)
        })
    }
}

impl Message for FetchObservationSnapshot {
    type Result = Result<ObservationSnapshot, ServiceError>;
}

impl Handler<FetchObservationSnapshot> for DbExecutor {
    type Result = Result<ObservationSnapshot, ServiceError>;

    fn handle(&mut self, msg: FetchObservationSnapshot, _: &mut Self::Context) -> Self::Result {
        use crate::schema::observation_snapshots::dsl::*;

        let conn: &PgConnection = &self.0.get().unwrap();
        let o = observation_snapshots
            .find(msg.observation_id)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;
        Ok(o)
    }
}
