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
use crate::models::{CreateVideoFile, DbExecutor, FetchEmptyVideoFile, UpdateVideoFile, VideoFile};
use actix::{Handler, Message};
use diesel::{self, prelude::*};

impl Message for CreateVideoFile {
    type Result = Result<VideoFile, ServiceError>;
}

impl Handler<CreateVideoFile> for DbExecutor {
    type Result = Result<VideoFile, ServiceError>;

    fn handle(&mut self, msg: CreateVideoFile, _: &mut Self::Context) -> Self::Result {
        use crate::schema::video_files::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(video_files)
            .values(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for UpdateVideoFile {
    type Result = Result<VideoFile, ServiceError>;
}

impl Handler<UpdateVideoFile> for DbExecutor {
    type Result = Result<VideoFile, ServiceError>;
    fn handle(&mut self, msg: UpdateVideoFile, _: &mut Self::Context) -> Self::Result {
        use crate::schema::video_files::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::update(video_files.filter(id.eq(msg.id)))
            .set(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for FetchEmptyVideoFile {
    type Result = Result<Vec<VideoFile>, ServiceError>;
}

impl Handler<FetchEmptyVideoFile> for DbExecutor {
    type Result = Result<Vec<VideoFile>, ServiceError>;

    fn handle(&mut self, _msg: FetchEmptyVideoFile, _: &mut Self::Context) -> Self::Result {
        use crate::schema::video_files::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        video_files
            .filter(size.eq(-1))
            .load::<VideoFile>(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}
