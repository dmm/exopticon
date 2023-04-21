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

use std::sync::{Arc, Mutex};

use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use thiserror::Error;

use crate::api::camera_groups::CameraGroup;

pub struct Null {
    camera_groups: Vec<CameraGroup>,
}

impl Null {
    #[allow(dead_code)]
    pub fn new(camera_groups: Vec<CameraGroup>) -> Self {
        Self { camera_groups }
    }
}

#[derive(Default)]
pub struct NullBuilder {
    #[allow(dead_code)]
    camera_groups: Vec<CameraGroup>,
}

#[allow(dead_code)]
impl NullBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn build(self) -> Null {
        Null {
            camera_groups: self.camera_groups,
        }
    }

    pub fn camera_groups(&mut self, camera_groups: &[CameraGroup]) {
        self.camera_groups.extend_from_slice(camera_groups);
    }
}

#[derive(Clone)]
pub enum ServiceKind {
    Real(r2d2::Pool<ConnectionManager<diesel::PgConnection>>),
    #[allow(dead_code)]
    Null(Arc<Mutex<Null>>),
}

#[derive(Clone)]
pub struct Service {
    pub pool: ServiceKind,
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
            pool: ServiceKind::Real(build_pool(database_url)),
        }
    }

    #[allow(dead_code)]
    pub fn new_null(null_db: Option<Null>) -> Self {
        let db = null_db.map_or_else(|| NullBuilder::new().build(), |d| d);
        Self {
            pool: ServiceKind::Null(Arc::new(Mutex::new(db))),
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

#[cfg(test)]
mod tests {
    use super::*;

    use diesel::{sql_query, Connection, PgConnection, RunQueryDsl};
    use std::sync::atomic::AtomicU32;
    use url::Url;

    // TestDb inspired by:
    // https://github.com/diesel-rs/diesel/issues/1549#issuecomment-892978784
    static TEST_DB_COUNTER: AtomicU32 = AtomicU32::new(0);

    pub struct TestDb {
        base_url: String,
        name: String,
        db: Service,
    }

    impl TestDb {
        pub fn new() -> Self {
            let name = format!(
                "test_db_{}_{}",
                std::process::id(),
                TEST_DB_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            );
            let base_url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL");
            let conn = PgConnection::establish(&base_url).unwrap();
            sql_query(format!("CREATE DATABASE {};", name))
                .execute(&conn)
                .unwrap();

            let mut url = Url::parse(&base_url).unwrap();
            url.set_path(&name);

            let conn2 = PgConnection::establish(&url.to_string()).unwrap();

            crate::embedded_migrations::run(&conn2).unwrap();

            let db = Service::new(&url.to_string());
            Self { base_url, name, db }
        }
    }

    impl Drop for TestDb {
        fn drop(&mut self) {
            let conn = PgConnection::establish(&self.base_url).unwrap();

            sql_query(format!(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
                self.name
            ))
            .execute(&conn)
            .unwrap();
            sql_query(format!("DROP DATABASE {}", self.name))
                .execute(&conn)
                .unwrap();
        }
    }

    pub fn run_db_test(f: fn(&Service)) {
        let testdb = TestDb::new();

        f(&testdb.db);

        drop(testdb);
    }

    #[test]
    pub fn nulldbbuilder_new_creates_empty() {
        // Arrange
        let builder = NullBuilder::new();

        // Act
        let null_db = builder.build();

        // Assert
        assert_eq!(0, null_db.camera_groups.len());
    }

    #[test]
    pub fn nulldbbuilder_addcameragroup_adds() {
        // Arrange
        let mut builder = NullBuilder::new();
        let camera_groups = vec![CameraGroup {
            id: 1,
            name: String::from("TestGroupA"),
            members: vec![],
        }];

        // Act
        builder.camera_groups(&camera_groups);
        let null_db = builder.build();

        // Assert
        assert_eq!(1, null_db.camera_groups.len());
        assert_eq!(camera_groups[0], null_db.camera_groups[0]);
    }
}
