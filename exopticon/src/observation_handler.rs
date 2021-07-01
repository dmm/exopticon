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
    CreateEvent, CreateObservations, DbExecutor, Event, EventModel, EventObservation,
    FetchObservation, FetchObservations, FetchObservationsByVideoUnit, Observation, QueryEvents,
    VideoUnit,
};
use actix::{Handler, Message};
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::{
    self,
    prelude::*,
    sql_types::{Text, Timestamp, Uuid},
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
    #[sql_type = "Uuid"]
    pub id: uuid::Uuid,
    #[sql_type = "Text"]
    pub tag: String,
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

        let new_event = Event {
            id: msg.id,
            tag: msg.tag,
        };

        let new_event_observations: Vec<EventObservation> = msg
            .observations
            .into_iter()
            .map(|oid| EventObservation {
                event_id: eid,
                observation_id: oid,
            })
            .collect();

        let conn: &PgConnection = &self.0.get().unwrap();
        conn.transaction(|| {
            diesel::insert_into(events)
                .values(&new_event)
                .on_conflict(crate::schema::events::dsl::id)
                .do_update()
                .set(&new_event)
                .execute(conn)
                .map_err(|_error| ServiceError::InternalServerError)?;
            diesel::delete(event_observations.filter(event_id.eq(eid2)))
                .execute(conn)
                .map_err(|_error| ServiceError::InternalServerError)?;

            diesel::insert_into(event_observations)
                .values(&new_event_observations)
                .execute(conn)
                .map_err(|_error| ServiceError::InternalServerError)?;

            let query = r#"
            SELECT event_id as id,
                   '' as tag,
                   MIN(begin_time + (frame_offset * interval '1 microsecond')) as begin_time,
                   MAX(begin_time + (frame_offset * interval '1 microsecond')) as end_time
            FROM event_observations as eo
            INNER JOIN observations obs
              ON eo.observation_id = obs.id
            INNER JOIN video_units as vu
              ON obs.video_unit_id = vu.id
            WHERE event_id = $1
            GROUP BY event_id;
            "#;

            let event_interval: EventInterval = diesel::sql_query(query)
                .bind::<diesel::sql_types::Uuid, _>(msg2.id)
                .get_result(conn)
                .map_err(|_error| ServiceError::InternalServerError)?;

            let event_model = EventModel {
                id: event_interval.id,
                tag: msg2.tag,
                begin_time: DateTime::from_utc(event_interval.begin_time, Utc),
                end_time: DateTime::from_utc(event_interval.end_time, Utc),
                observations: msg2.observations,
            };

            Ok(event_model)
        })
    }
}

impl Message for QueryEvents {
    type Result = Result<Vec<EventModel>, ServiceError>;
}

impl Handler<QueryEvents> for DbExecutor {
    type Result = Result<Vec<EventModel>, ServiceError>;

    fn handle(&mut self, _msg: QueryEvents, _: &mut Self::Context) -> Self::Result {
        use crate::schema::event_observations::dsl::*;

        let query = r#"
            SELECT ev.id as id,
                   ev.tag as tag,
                   MIN(begin_time + (frame_offset * interval '1 microsecond')) as begin_time,
                   MAX(begin_time + (frame_offset * interval '1 microsecond')) as end_time
            FROM events as ev
            INNER JOIN event_observations as eo
              ON ev.id = eo.event_id
            INNER JOIN observations obs
              ON eo.observation_id = obs.id
            INNER JOIN video_units as vu
              ON obs.video_unit_id = vu.id
            GROUP BY ev.id
            ORDER BY begin_time DESC
            LIMIT 1000
            "#;

        let conn: &PgConnection = &self.0.get().unwrap();
        conn.transaction(|| {
            let event_intervals: Vec<EventInterval> = diesel::sql_query(query)
                .get_results(conn)
                .map_err(|_error| ServiceError::InternalServerError)?;

            let mut events = Vec::new();
            for ev in &event_intervals {
                let obs = event_observations
                    .select(crate::schema::event_observations::dsl::observation_id)
                    .filter(event_id.eq(ev.id))
                    .load::<i64>(conn)?;

                events.push(EventModel {
                    id: ev.id,
                    tag: ev.tag.clone(),
                    begin_time: DateTime::from_utc(ev.begin_time, Utc),
                    end_time: DateTime::from_utc(ev.end_time, Utc),
                    observations: obs,
                });
            }

            Ok(events)
        })
    }
}
