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
use crate::models::{Camera, CreateCamera, DbExecutor, FetchAllCamera, FetchCamera, UpdateCamera};
use actix::{Handler, Message};
use diesel::*;

impl Message for CreateCamera {
    type Result = Result<Camera, ServiceError>;
}

impl Handler<CreateCamera> for DbExecutor {
    type Result = Result<Camera, ServiceError>;

    fn handle(&mut self, msg: CreateCamera, _: &mut Self::Context) -> Self::Result {
        use crate::schema::cameras::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(cameras)
            .values(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for UpdateCamera {
    type Result = Result<Camera, ServiceError>;
}

impl Handler<UpdateCamera> for DbExecutor {
    type Result = Result<Camera, ServiceError>;

    fn handle(&mut self, msg: UpdateCamera, _: &mut Self::Context) -> Self::Result {
        use crate::schema::cameras::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::update(cameras.filter(id.eq(msg.id)))
            .set(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for FetchCamera {
    type Result = Result<Camera, ServiceError>;
}

impl Handler<FetchCamera> for DbExecutor {
    type Result = Result<Camera, ServiceError>;

    fn handle(&mut self, msg: FetchCamera, _: &mut Self::Context) -> Self::Result {
        use crate::schema::cameras::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let c = cameras
            .find(msg.id)
            .get_result::<Camera>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        Ok(c)
    }
}

impl Message for FetchAllCamera {
    type Result = Result<Vec<Camera>, ServiceError>;
}

impl Handler<FetchAllCamera> for DbExecutor {
    type Result = Result<Vec<Camera>, ServiceError>;

    fn handle(&mut self, _msg: FetchAllCamera, _: &mut Self::Context) -> Self::Result {
        use crate::schema::cameras::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        cameras
            .load::<Camera>(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}
