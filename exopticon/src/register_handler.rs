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

use actix::{Handler, Message};
use diesel::prelude::*;

use crate::errors::ServiceError;
use crate::models::{CreateUser, DbExecutor, SlimUser, User};
use crate::utils::hash_password;

/// `UserData` is used to extract data from a post request by the client
#[derive(Debug, Deserialize)]
pub struct UserData {
    /// plaintext password
    pub password: String,
}

impl Message for CreateUser {
    type Result = Result<SlimUser, ServiceError>;
}

impl Handler<CreateUser> for DbExecutor {
    type Result = Result<SlimUser, ServiceError>;
    fn handle(&mut self, msg: CreateUser, _: &mut Self::Context) -> Self::Result {
        use crate::schema::users::dsl::users;
        let conn: &PgConnection = &self.0.get().unwrap();

        let password: String = hash_password(&msg.password)?;
        let user: User = diesel::insert_into(users)
            .values(CreateUser {
                username: msg.username,
                password,
                timezone: msg.timezone,
            })
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        Ok(user.into())
    }
}
