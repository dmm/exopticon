/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2022 David Matthew Mattli <dmm@mattli.us>
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

pub mod auth;
pub mod camera_groups;
pub mod cameras;
pub mod storage_groups;
pub mod uuid;
pub mod video_units;

use std::time::Duration;

use diesel::{connection::SimpleConnection, r2d2::ConnectionManager, SqliteConnection};
use thiserror::Error;

#[derive(Debug)]
pub struct ConnectionOptions {
    pub enable_wal: bool,
    pub enable_foreign_keys: bool,
    pub busy_timeout: Option<Duration>,
}

impl diesel::r2d2::CustomizeConnection<SqliteConnection, diesel::r2d2::Error>
    for ConnectionOptions
{
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        (|| {
            if self.enable_wal {
                conn.batch_execute("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
            }
            if self.enable_foreign_keys {
                conn.batch_execute("PRAGMA foreign_keys = ON;")?;
            }
            if let Some(d) = self.busy_timeout {
                conn.batch_execute(&format!("PRAGMA busy_timeout = {};", d.as_millis()))?;
            }
            Ok(())
        })()
        .map_err(diesel::r2d2::Error::QueryError)
    }
}

#[derive(Clone)]
pub struct Service {
    pub pool: r2d2::Pool<ConnectionManager<diesel::SqliteConnection>>,
}

fn build_pool(database_url: &str) -> r2d2::Pool<ConnectionManager<diesel::SqliteConnection>> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    r2d2::Pool::builder()
        .max_size(16)
        .connection_customizer(Box::new(ConnectionOptions {
            enable_wal: true,
            enable_foreign_keys: true,
            busy_timeout: Some(Duration::from_secs(5)),
        }))
        .build(manager)
        .expect("Failed to create pool.")
}

impl Service {
    pub fn new(database_url: &str) -> Self {
        Self {
            pool: build_pool(database_url),
        }
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("not found")]
    NotFound,
    #[error("other database error")]
    Other(OtherError),
}

#[derive(Debug)]
pub enum OtherError {
    DbPoolError {
        description: String,
        cause: r2d2::Error,
    },

    DbError {
        description: String,
        cause: diesel::result::Error,
    },
}

impl From<r2d2::Error> for Error {
    #[must_use]
    fn from(err: r2d2::Error) -> Self {
        Self::Other(OtherError::DbPoolError {
            description: err.to_string(),
            cause: err,
        })
    }
}

impl From<diesel::result::Error> for Error {
    #[must_use]
    fn from(err: diesel::result::Error) -> Self {
        if err == diesel::result::Error::NotFound {
            Self::NotFound
        } else {
            Self::Other(OtherError::DbError {
                description: err.to_string(),
                cause: err,
            })
        }
    }
}
