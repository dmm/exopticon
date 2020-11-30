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
    CreateObservations, DbExecutor, FetchObservation, FetchObservations,
    FetchObservationsByVideoUnit, Observation, VideoUnit,
};
use actix::{Handler, Message};
use diesel::{self, prelude::*};

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
