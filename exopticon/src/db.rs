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
pub mod video_units;

use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use thiserror::Error;

#[derive(Clone)]
pub struct Service {
    pub pool: r2d2::Pool<ConnectionManager<diesel::PgConnection>>,
}

fn build_pool(database_url: &str) -> r2d2::Pool<ConnectionManager<diesel::PgConnection>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    r2d2::Pool::builder()
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

#[derive(Error, Debug)]
pub enum OtherError {
    #[error("DbPoolError: {description:?} {cause:?}")]
    DbPoolError {
        description: String,
        cause: r2d2::Error,
    },
    #[error("DbError: {description:?} {cause:?}")]
    DbError {
        description: String,
        cause: diesel::result::Error,
    },
}

impl From<r2d2::Error> for Error {
    fn from(err: r2d2::Error) -> Self {
        Self::Other(OtherError::DbPoolError {
            description: err.to_string(),
            cause: err,
        })
    }
}

impl From<diesel::result::Error> for Error {
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

#[cfg(test)]
mod tests {}
