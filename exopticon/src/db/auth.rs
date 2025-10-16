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
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use uuid::Uuid;

use crate::{
    api::auth::SlimAccessToken,
    schema::{user_sessions, users},
};

use super::Service;

/// Full user model struct, represents full value from database.
#[derive(Queryable, Identifiable, Serialize)]
#[serde(rename_all = "camelCase")]
#[diesel(table_name = users)]
pub struct User {
    /// user id
    pub id: Uuid,
    ///  username
    pub username: String,
    /// hashed password
    pub password: String,
}

impl From<User> for crate::api::auth::User {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
        }
    }
}

/// User login session or token
#[derive(Associations, Insertable, Serialize, Queryable, Clone)]
#[diesel(belongs_to(User))]
#[diesel(table_name = user_sessions)]
pub struct UserSession {
    /// user session id
    pub id: Uuid,
    /// user session name
    pub name: String,
    /// id of user associated with session
    pub user_id: Uuid,
    /// session key value
    pub session_key: String,
    /// flag indicating where it is an api token or user session
    pub is_token: bool,
    /// Expiration timestamp
    pub expiration: DateTime<Utc>,
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
        use crate::schema::users::dsl;
        let mut conn = self.pool.get()?;

        let u = dsl::users
            .filter(dsl::username.eq(username))
            .first::<User>(&mut conn)?;

        if let Ok(matching) = bcrypt::verify(password, &u.password)
            && matching
        {
            return Ok(u.into());
        }
        error!("Validation faild :(");
        Err(super::Error::NotFound)
    }

    pub fn fetch_user(&self, user_id: Uuid) -> Result<crate::api::auth::User, super::Error> {
        use crate::schema::users::dsl;

        let mut conn = self.pool.get()?;
        let u = dsl::users
            .filter(dsl::id.eq(user_id))
            .first::<User>(&mut conn)?;

        Ok(u.into())
    }

    pub fn create_user_session(
        &self,
        session: &crate::api::auth::CreateUserSession,
    ) -> Result<String, super::Error> {
        use crate::schema::user_sessions::dsl;

        let mut conn = self.pool.get()?;

        diesel::insert_into(dsl::user_sessions)
            .values((
                dsl::id.eq(Uuid::now_v7()),
                dsl::name.eq(&session.name),
                dsl::user_id.eq(&session.user_id),
                dsl::session_key.eq(&session.session_key),
                dsl::is_token.eq(&session.is_token),
                dsl::expiration.eq(&session.expiration),
            ))
            .execute(&mut conn)?;
        Ok(session.session_key.clone())
    }

    pub fn delete_user_session(&self, session_id: Uuid) -> Result<(), super::Error> {
        use crate::schema::user_sessions::dsl::*;
        let mut conn = self.pool.get()?;

        diesel::delete(user_sessions.filter(id.eq(session_id))).execute(&mut conn)?;
        Ok(())
    }

    pub fn validate_user_session(
        &self,
        session_key_text: &str,
    ) -> Result<crate::api::auth::User, super::Error> {
        use crate::schema::user_sessions::dsl::*;
        let mut conn = self.pool.get()?;

        // remove expired sessions
        diesel::delete(user_sessions.filter(expiration.lt(Utc::now()))).execute(&mut conn)?;
        let session = user_sessions
            .filter(session_key.eq(&session_key_text))
            .filter(expiration.gt(Utc::now()))
            .first::<UserSession>(&mut conn)?;

        let user = crate::schema::users::dsl::users
            .filter(crate::schema::users::dsl::id.eq(session.user_id))
            .first::<User>(&mut conn)?;

        Ok(user.into())
    }

    pub fn fetch_users_tokens(&self, user_id2: Uuid) -> Result<Vec<SlimAccessToken>, super::Error> {
        use crate::schema::user_sessions::dsl::*;
        let mut conn = self.pool.get()?;

        let sessions = user_sessions
            .filter(user_id.eq(user_id2))
            .filter(is_token.eq(true))
            .load::<UserSession>(&mut conn)?;
        Ok(sessions.into_iter().map(std::convert::Into::into).collect())
    }
}
