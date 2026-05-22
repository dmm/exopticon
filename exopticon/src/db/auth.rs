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
use chrono::Utc;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

use crate::{api::auth::SlimAccessToken, schema::users};

use super::{Service, datetime_to_micros, micros_to_datetime};

#[derive(Queryable, Identifiable, Serialize)]
#[serde(rename_all = "camelCase")]
#[diesel(primary_key(username))]
#[diesel(table_name = users)]
pub struct User {
    pub username: String,
    pub display_name: String,
    pub password: String,
}

impl From<User> for crate::api::auth::User {
    fn from(user: User) -> Self {
        Self {
            username: user.username,
            display_name: user.display_name,
        }
    }
}

#[derive(Serialize, Queryable, Clone)]
#[diesel(table_name = user_sessions)]
pub struct UserSession {
    pub id: i64,
    pub name: String,
    pub user_name: String,
    pub session_key: String,
    pub is_token: bool,
    pub expiration_us: i64,
}

impl TryFrom<UserSession> for SlimAccessToken {
    type Error = super::Error;

    fn try_from(u: UserSession) -> Result<Self, Self::Error> {
        Ok(Self {
            id: u.id,
            name: u.name,
            user_name: u.user_name,
            expiration: micros_to_datetime("user_sessions.expiration_us", u.expiration_us)?,
        })
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
        error!("Validation failed :(");
        Err(super::Error::NotFound)
    }

    pub fn create_user_session(
        &self,
        session: &crate::api::auth::CreateUserSession,
    ) -> Result<String, super::Error> {
        use crate::schema::user_sessions::dsl;

        let mut conn = self.pool.get()?;

        diesel::insert_into(dsl::user_sessions)
            .values((
                dsl::name.eq(&session.name),
                dsl::user_name.eq(&session.user_name),
                dsl::session_key.eq(&session.session_key),
                dsl::is_token.eq(&session.is_token),
                dsl::expiration_us.eq(datetime_to_micros(session.expiration)),
            ))
            .execute(&mut conn)?;
        Ok(session.session_key.clone())
    }

    pub fn delete_user_session(&self, session_id: i64) -> Result<(), super::Error> {
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

        diesel::delete(user_sessions.filter(expiration_us.lt(datetime_to_micros(Utc::now()))))
            .execute(&mut conn)?;
        let session = user_sessions
            .filter(session_key.eq(&session_key_text))
            .filter(expiration_us.gt(datetime_to_micros(Utc::now())))
            .first::<UserSession>(&mut conn)?;

        let user = crate::schema::users::dsl::users
            .filter(crate::schema::users::dsl::username.eq(session.user_name))
            .first::<User>(&mut conn)?;

        Ok(user.into())
    }

    pub fn fetch_users_tokens(
        &self,
        user_name2: &str,
    ) -> Result<Vec<SlimAccessToken>, super::Error> {
        use crate::schema::user_sessions::dsl::*;
        let mut conn = self.pool.get()?;

        let sessions = user_sessions
            .filter(user_name.eq(user_name2))
            .filter(is_token.eq(true))
            .load::<UserSession>(&mut conn)?;
        sessions
            .into_iter()
            .map(std::convert::TryInto::try_into)
            .collect()
    }
}
