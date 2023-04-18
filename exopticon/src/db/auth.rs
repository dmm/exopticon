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
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

use crate::{
    api::auth::SlimAccessToken,
    schema::{user_sessions, users},
};

use super::Service;

/// Full user model struct, represents full value from database.
#[derive(Queryable, Associations, Identifiable, Serialize)]
#[serde(rename_all = "camelCase")]
#[table_name = "users"]
pub struct User {
    /// user id
    pub id: i32,
    ///  username
    pub username: String,
    /// hashed password
    pub password: String,
    /// Olson timezone, e.g. America/Chicago
    pub timezone: String,
    /// insertion date time
    pub inserted_at: NaiveDateTime,
    /// modified date time
    pub updated_at: NaiveDateTime,
}

impl From<User> for crate::api::auth::User {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            timezone: String::from("Etc/UTC"),
        }
    }
}

/// User login session or token
#[derive(Associations, Insertable, Serialize, Queryable, Clone)]
#[belongs_to(User)]
pub struct UserSession {
    /// user session id
    pub id: i32,
    /// user session name
    pub name: String,
    /// id of user associated with session
    pub user_id: i32,
    /// session key value
    pub session_key: String,
    /// flag indicating where it is an api token or user session
    pub is_token: bool,
    /// Expiration timestamp
    pub expiration: DateTime<Utc>,
    /// session create timestamp
    pub inserted_at: DateTime<Utc>,
}

impl From<UserSession> for SlimAccessToken {
    fn from(u: UserSession) -> Self {
        Self {
            id: u.id,
            name: u.name,
            user_id: u.user_id,
            expiration: u.expiration,
        }
    }
}

impl Service {
    pub fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<crate::api::auth::User, super::Error> {
        match &self.pool {
            super::ServiceKind::Real(pool) => {
                use crate::schema::users::dsl;
                let conn = pool.get()?;

                let u = dsl::users
                    .filter(dsl::username.eq(username))
                    .first::<User>(&conn)?;

                if let Ok(matching) = bcrypt::verify(password, &u.password) {
                    if matching {
                        return Ok(u.into());
                    }
                }
                Err(super::Error::NotFound)
            }
            super::ServiceKind::Null(_) => todo!(),
        }
    }

    pub fn fetch_user(&self, user_id: i32) -> Result<crate::api::auth::User, super::Error> {
        match &self.pool {
            crate::db::ServiceKind::Real(pool) => {
                let conn = pool.get()?;
                use crate::schema::users::dsl;
                let u = dsl::users
                    .filter(dsl::id.eq(user_id))
                    .first::<User>(&conn)?;

                Ok(u.into())
            }
            crate::db::ServiceKind::Null(_) => todo!(),
        }
    }

    pub fn create_user_session(
        &self,
        session: &crate::api::auth::CreateUserSession,
    ) -> Result<String, super::Error> {
        match &self.pool {
            super::ServiceKind::Real(pool) => {
                use crate::schema::user_sessions::dsl;
                let conn = pool.get()?;

                diesel::insert_into(dsl::user_sessions)
                    .values((
                        dsl::name.eq(&session.name),
                        dsl::user_id.eq(&session.user_id),
                        dsl::session_key.eq(&session.session_key),
                        dsl::is_token.eq(&session.is_token),
                        dsl::expiration.eq(&session.expiration),
                    ))
                    .execute(&conn)?;
                return Ok(session.session_key.clone());
            }
            super::ServiceKind::Null(_) => todo!(),
        }
    }

    pub fn delete_user_session(&self, session_id: i32) -> Result<(), super::Error> {
        match &self.pool {
            super::ServiceKind::Real(pool) => {
                use crate::schema::user_sessions::dsl::*;
                let conn = pool.get()?;

                diesel::delete(user_sessions.filter(id.eq(session_id))).execute(&conn)?;
                Ok(())
            }
            super::ServiceKind::Null(_) => todo!(),
        }
    }

    pub fn validate_user_session(
        &self,
        session_key_text: &str,
    ) -> Result<crate::api::auth::User, super::Error> {
        use crate::schema::user_sessions::dsl::*;
        match &self.pool {
            crate::db::ServiceKind::Real(pool) => {
                let conn = pool.get()?;

                // remove expired sessions
                diesel::delete(user_sessions.filter(expiration.lt(Utc::now()))).execute(&conn)?;
                let session = user_sessions
                    .filter(session_key.eq(&session_key_text))
                    .filter(expiration.gt(Utc::now()))
                    .first::<UserSession>(&conn)?;

                let user = crate::schema::users::dsl::users
                    .filter(crate::schema::users::dsl::id.eq(session.user_id))
                    .first::<User>(&conn)?;

                Ok(user.into())
            }
            crate::db::ServiceKind::Null(_) => todo!(),
        }
    }

    pub fn fetch_users_tokens(&self, user_id2: i32) -> Result<Vec<SlimAccessToken>, super::Error> {
        match &self.pool {
            super::ServiceKind::Real(pool) => {
                use crate::schema::user_sessions::dsl::*;

                let conn = pool.get()?;

                let sessions = user_sessions
                    .filter(user_id.eq(user_id2))
                    .filter(is_token.eq(true))
                    .load::<UserSession>(&conn)?;
                Ok(sessions.into_iter().map(|x| x.into()).collect())
            }
            super::ServiceKind::Null(_) => todo!(),
        }
    }
}
