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
use bcrypt::verify;
use chrono::Utc;
use diesel::prelude::*;
use serde::Deserialize;

use crate::errors::ServiceError;
use crate::models::{
    CreateUserSession, DbExecutor, DeleteUserSession, DeleteUserToken, FetchUser, FetchUserSession,
    FetchUserTokens, SlimAccessToken, SlimUser, User, UserSession,
};

/// Represents data for an authentication attempt
#[derive(Debug, Deserialize)]
pub struct AuthData {
    /// username
    pub username: String,
    /// plaintext password
    pub password: String,
}

impl Message for AuthData {
    type Result = Result<SlimUser, ServiceError>;
}

impl Handler<AuthData> for DbExecutor {
    type Result = Result<SlimUser, ServiceError>;
    fn handle(&mut self, msg: AuthData, _: &mut Self::Context) -> Self::Result {
        use crate::schema::users::dsl::{username, users};
        let conn: &PgConnection = &self.0.get().unwrap();
        let mismatch_error = Err(ServiceError::BadRequest(
            "Username and Password don't match".into(),
        ));

        let mut items = users
            .filter(username.eq(&msg.username))
            .load::<User>(conn)
            .map_err(|error| {
                error!("Unable to load users! {}", error);
                ServiceError::InternalServerError
            })?;

        if let Some(user) = items.pop() {
            if let Ok(matching) = verify(&msg.password, &user.password) {
                if matching {
                    return Ok(user.into());
                }
            } else {
                return mismatch_error;
            }
        }
        mismatch_error
    }
}

impl Message for FetchUser {
    type Result = Result<SlimUser, ServiceError>;
}

impl Handler<FetchUser> for DbExecutor {
    type Result = Result<SlimUser, ServiceError>;

    fn handle(&mut self, msg: FetchUser, _: &mut Self::Context) -> Self::Result {
        use crate::schema::users::dsl::users;
        let conn: &PgConnection = &self.0.get().unwrap();

        let item = users
            .find(msg.user_id)
            .get_result::<User>(conn)
            .map_err(|error| {
                error!("Unable to load users! {}", error);
                ServiceError::InternalServerError
            })?;

        Ok(item.into())
    }
}

impl Message for CreateUserSession {
    type Result = Result<UserSession, ServiceError>;
}

impl Handler<CreateUserSession> for DbExecutor {
    type Result = Result<UserSession, ServiceError>;

    fn handle(&mut self, msg: CreateUserSession, _: &mut Self::Context) -> Self::Result {
        use crate::schema::user_sessions::dsl::user_sessions;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(user_sessions)
            .values(&msg)
            .get_result(conn)
            .map_err(|error| {
                error!("Failed to insert user session: {}", error);
                ServiceError::InternalServerError
            })
    }
}

impl Message for DeleteUserToken {
    type Result = Result<(), ServiceError>;
}

impl Handler<DeleteUserToken> for DbExecutor {
    type Result = Result<(), ServiceError>;

    fn handle(&mut self, msg: DeleteUserToken, _: &mut Self::Context) -> Self::Result {
        use crate::schema::user_sessions::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::delete(user_sessions.filter(id.eq(msg.token_id))).execute(conn)?;

        Ok(())
    }
}

impl Message for FetchUserSession {
    type Result = Result<UserSession, ServiceError>;
}

impl Handler<FetchUserSession> for DbExecutor {
    type Result = Result<UserSession, ServiceError>;

    fn handle(&mut self, msg: FetchUserSession, _: &mut Self::Context) -> Self::Result {
        use crate::schema::user_sessions::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        // remove expired sessions
        diesel::delete(user_sessions.filter(expiration.lt(Utc::now()))).execute(conn)?;

        let s = user_sessions
            .filter(session_key.eq(msg.session_key))
            .filter(expiration.gt(Utc::now()))
            .first(conn)
            .map_err(|error| {
                error!("Error fetching user session: {}", error);
                ServiceError::InternalServerError
            })?;

        Ok(s)
    }
}

impl Message for DeleteUserSession {
    type Result = Result<(), ServiceError>;
}

impl Handler<DeleteUserSession> for DbExecutor {
    type Result = Result<(), ServiceError>;

    fn handle(&mut self, msg: DeleteUserSession, _: &mut Self::Context) -> Self::Result {
        use crate::schema::user_sessions::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::delete(user_sessions.filter(session_key.eq(msg.session_key))).execute(conn)?;
        Ok(())
    }
}

impl Message for FetchUserTokens {
    type Result = Result<Vec<SlimAccessToken>, ServiceError>;
}

impl Handler<FetchUserTokens> for DbExecutor {
    type Result = Result<Vec<SlimAccessToken>, ServiceError>;

    fn handle(&mut self, msg: FetchUserTokens, _: &mut Self::Context) -> Self::Result {
        use crate::schema::user_sessions::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let sessions = user_sessions
            .filter(user_id.eq(msg.user_id))
            .filter(is_token.eq(true))
            .load::<UserSession>(conn)
            .map_err(|error| {
                error!("Error fetching user tokens: {}", error);
                ServiceError::InternalServerError
            })?;

        let tokens: Vec<SlimAccessToken> = sessions.iter().map(SlimAccessToken::from).collect();

        Ok(tokens)
    }
}
