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

macro_rules! db_write {
    ($service:expr, $operation:literal, |$conn:ident| $body:block) => {
        super::Service::measure_db_operation($operation, super::DbOperationKind::Write, || {
            let mut conn = $service.connection($operation)?;
            diesel::SqliteConnection::immediate_transaction::<_, super::Error, _>(
                &mut conn,
                |$conn| $body,
            )
        })
    };
}

macro_rules! db_read {
    ($service:expr, $operation:literal, |$conn:ident| $body:block) => {
        super::Service::measure_db_operation($operation, super::DbOperationKind::Read, || {
            let mut conn = $service.connection($operation)?;
            diesel::SqliteConnection::transaction::<_, super::Error, _>(&mut conn, |$conn| $body)
        })
    };
}

pub mod auth;
pub mod camera_groups;
pub mod cameras;
pub mod config;
pub mod storage_groups;
pub mod video_units;

use chrono::{DateTime, Utc};
use diesel::SqliteConnection;
use diesel::connection::SimpleConnection;
use diesel::r2d2::ConnectionManager;
use metrics::{counter, histogram};
use std::time::Instant;
use thiserror::Error;

pub type DbConnection = SqliteConnection;
pub type DbPool = r2d2::Pool<ConnectionManager<DbConnection>>;
type DbPooledConnection = r2d2::PooledConnection<ConnectionManager<DbConnection>>;

const DB_OPERATION_DURATION_SECONDS: &str = "exopticon_db_operation_duration_seconds";
const DB_POOL_CHECKOUT_DURATION_SECONDS: &str = "exopticon_db_pool_checkout_duration_seconds";
const DB_ERRORS_TOTAL: &str = "exopticon_db_errors_total";

#[derive(Clone, Copy)]
enum DbOperationKind {
    Read,
    Write,
}

impl DbOperationKind {
    const fn as_label(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

#[derive(Clone)]
pub struct Service {
    pub pool: DbPool,
}

#[derive(Debug)]
struct SqliteConnectionCustomizer;

impl r2d2::CustomizeConnection<SqliteConnection, diesel::r2d2::Error>
    for SqliteConnectionCustomizer
{
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        for statement in [
            "PRAGMA busy_timeout = 2000;",
            "PRAGMA journal_mode = WAL;",
            "PRAGMA synchronous = NORMAL;",
            "PRAGMA wal_autocheckpoint = 1000;",
            "PRAGMA wal_checkpoint(TRUNCATE);",
            "PRAGMA foreign_keys = ON;",
        ] {
            conn.batch_execute(statement)
                .map_err(diesel::r2d2::Error::QueryError)?;
        }
        Ok(())
    }
}

fn build_pool(database_url: &str) -> DbPool {
    let manager = ConnectionManager::<DbConnection>::new(database_url);
    r2d2::Pool::builder()
        .connection_customizer(Box::new(SqliteConnectionCustomizer))
        .build(manager)
        .expect("Failed to create pool.")
}

impl Service {
    pub fn new(database_url: &str) -> Self {
        Self {
            pool: build_pool(database_url),
        }
    }

    fn connection(&self, operation: &'static str) -> Result<DbPooledConnection, Error> {
        let start = Instant::now();
        let result = self.pool.get();
        let outcome = if result.is_ok() { "success" } else { "error" };
        histogram!(
            DB_POOL_CHECKOUT_DURATION_SECONDS,
            "operation" => operation,
            "outcome" => outcome,
        )
        .record(start.elapsed().as_secs_f64());

        result.map_err(Error::from)
    }

    fn measure_db_operation<T, F>(
        operation: &'static str,
        kind: DbOperationKind,
        f: F,
    ) -> Result<T, Error>
    where
        F: FnOnce() -> Result<T, Error>,
    {
        let start = Instant::now();
        let result = f();
        let outcome = if result.is_ok() { "success" } else { "error" };

        histogram!(
            DB_OPERATION_DURATION_SECONDS,
            "operation" => operation,
            "kind" => kind.as_label(),
            "outcome" => outcome,
        )
        .record(start.elapsed().as_secs_f64());

        if let Err(err) = &result {
            record_db_error(operation, err);
        }

        result
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
    #[error("invalid timestamp stored in database: {field}={value}")]
    InvalidTimestamp { field: &'static str, value: i64 },
}

const fn datetime_to_micros(timestamp: DateTime<Utc>) -> i64 {
    timestamp.timestamp_micros()
}

fn micros_to_datetime(field: &'static str, timestamp_us: i64) -> Result<DateTime<Utc>, Error> {
    DateTime::from_timestamp_micros(timestamp_us).ok_or(Error::Other(
        OtherError::InvalidTimestamp {
            field,
            value: timestamp_us,
        },
    ))
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

fn record_db_error(operation: &'static str, err: &Error) {
    counter!(
        DB_ERRORS_TOTAL,
        "operation" => operation,
        "error_class" => classify_db_error(err),
    )
    .increment(1);
}

fn classify_db_error(err: &Error) -> &'static str {
    match err {
        Error::NotFound => "not_found",
        Error::Other(OtherError::DbPoolError { .. }) => "pool",
        Error::Other(OtherError::DbError { cause, .. }) => classify_diesel_error(cause),
        Error::Other(OtherError::InvalidTimestamp { .. }) => "invalid_timestamp",
    }
}

fn classify_diesel_error(err: &diesel::result::Error) -> &'static str {
    match err {
        diesel::result::Error::DatabaseError(kind, info) => {
            let message = info.message().to_ascii_lowercase();
            if message.contains("database is locked") || message.contains("sqlite_locked") {
                "sqlite_locked"
            } else if message.contains("database is busy") || message.contains("sqlite_busy") {
                "sqlite_busy"
            } else {
                classify_database_error_kind(*kind)
            }
        }
        diesel::result::Error::NotFound => "not_found",
        diesel::result::Error::QueryBuilderError(_) => "query_builder",
        diesel::result::Error::DeserializationError(_) => "deserialization",
        diesel::result::Error::SerializationError(_) => "serialization",
        diesel::result::Error::RollbackErrorOnCommit { .. } => "rollback_on_commit",
        diesel::result::Error::RollbackTransaction => "rollback",
        diesel::result::Error::AlreadyInTransaction => "already_in_transaction",
        diesel::result::Error::NotInTransaction => "not_in_transaction",
        diesel::result::Error::BrokenTransactionManager => "broken_transaction_manager",
        _ => "other",
    }
}

const fn classify_database_error_kind(kind: diesel::result::DatabaseErrorKind) -> &'static str {
    match kind {
        diesel::result::DatabaseErrorKind::UniqueViolation => "unique_violation",
        diesel::result::DatabaseErrorKind::ForeignKeyViolation => "foreign_key_violation",
        diesel::result::DatabaseErrorKind::UnableToSendCommand => "unable_to_send_command",
        diesel::result::DatabaseErrorKind::SerializationFailure => "serialization_failure",
        diesel::result::DatabaseErrorKind::ReadOnlyTransaction => "read_only_transaction",
        diesel::result::DatabaseErrorKind::NotNullViolation => "not_null_violation",
        diesel::result::DatabaseErrorKind::CheckViolation => "check_violation",
        diesel::result::DatabaseErrorKind::ClosedConnection => "closed_connection",
        _ => "database_error",
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use chrono::{DateTime, Duration, Utc};
    use diesel_migrations::MigrationHarness;
    use tempfile::TempDir;

    use super::Service;
    use crate::{
        api::{
            auth::CreateUserSession,
            video_units::{CreateVideoFile, CreateVideoUnit},
        },
        config::{Camera, CameraGroup, StorageGroup, User, ValidatedConfig},
    };

    fn migrated_service() -> (TempDir, Service) {
        let temp_dir = TempDir::new().expect("temp dir created");
        let database_path = temp_dir.path().join("exopticon.sqlite");
        let database_url = database_path.display().to_string();
        let service = Service::new(&database_url);

        let mut conn = service.pool.get().expect("migration connection");
        conn.run_pending_migrations(crate::MIGRATIONS)
            .expect("migrations run");
        drop(conn);

        (temp_dir, service)
    }

    fn sample_config(password: &str, members: Vec<&str>) -> ValidatedConfig {
        let password_hash = bcrypt::hash(password, 4).expect("password hash created");

        ValidatedConfig {
            storage_groups: vec![StorageGroup {
                name: "primary".to_string(),
                display_name: "Primary".to_string(),
                storage_path: "/video".to_string(),
                max_storage_size: 1024,
            }],
            cameras: vec![
                Camera {
                    name: "front".to_string(),
                    display_name: "Front".to_string(),
                    storage_group_name: "primary".to_string(),
                    ip: "192.0.2.10".to_string(),
                    onvif_port: 80,
                    mac: "00:00:00:00:00:01".to_string(),
                    username: "camera-user".to_string(),
                    password: "camera-password".to_string(),
                    rtsp_url: "rtsp://front.example/stream".to_string(),
                    ptz_type: "none".to_string(),
                    ptz_profile_token: String::new(),
                    enabled: true,
                    ptz_x_step_size: 1,
                    ptz_y_step_size: 1,
                },
                Camera {
                    name: "back".to_string(),
                    display_name: "Back".to_string(),
                    storage_group_name: "primary".to_string(),
                    ip: "192.0.2.11".to_string(),
                    onvif_port: 80,
                    mac: "00:00:00:00:00:02".to_string(),
                    username: "camera-user".to_string(),
                    password: "camera-password".to_string(),
                    rtsp_url: "rtsp://back.example/stream".to_string(),
                    ptz_type: "none".to_string(),
                    ptz_profile_token: String::new(),
                    enabled: true,
                    ptz_x_step_size: 1,
                    ptz_y_step_size: 1,
                },
            ],
            camera_groups: vec![CameraGroup {
                name: "all".to_string(),
                display_name: "All".to_string(),
                members: members
                    .into_iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
            }],
            users: vec![User {
                username: "alice".to_string(),
                display_name: "Alice".to_string(),
                password_hash,
            }],
        }
    }

    fn test_time(timestamp: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(timestamp)
            .expect("valid timestamp")
            .with_timezone(&Utc)
    }

    #[test]
    fn applies_config_and_replaces_camera_group_memberships() {
        let (_temp_dir, service) = migrated_service();
        service
            .apply_config(&sample_config("secret", vec!["front", "back"]))
            .expect("initial config applied");

        let group = service
            .fetch_camera_group("all")
            .expect("camera group fetched");
        assert_eq!(group.spec.members, vec!["front", "back"]);

        service
            .apply_config(&sample_config("secret", vec!["back"]))
            .expect("updated config applied");

        let group = service
            .fetch_camera_group("all")
            .expect("camera group fetched");
        assert_eq!(group.spec.members, vec!["back"]);

        let storage_group = service
            .fetch_storage_group("primary")
            .expect("storage group fetched");
        assert_eq!(storage_group.metadata.display_name, "Primary");
    }

    #[test]
    fn creates_closes_fetches_and_deletes_video_segment() {
        let (temp_dir, service) = migrated_service();
        service
            .apply_config(&sample_config("secret", vec!["front"]))
            .expect("config applied");

        let filename = temp_dir.path().join("segment.mkv");
        std::fs::write(&filename, b"video").expect("video file written");
        let begin_time = test_time("2026-05-21T00:00:00Z");
        let end_time = test_time("2026-05-21T00:00:05Z");

        let (video_unit, video_file) = service
            .create_video_segment(
                &CreateVideoUnit {
                    camera_name: "front".to_string(),
                    begin_time,
                    end_time: begin_time,
                },
                CreateVideoFile {
                    filename: filename.display().to_string(),
                    size: 0,
                },
            )
            .expect("video segment created");

        assert!(video_unit.id > 0);
        assert!(video_file.id > 0);
        assert_eq!(video_file.video_unit_id, video_unit.id);

        let (_video_unit, video_file) = service
            .close_video_segment(video_unit.id, video_file.id, end_time, 4)
            .expect("video segment closed");
        assert_eq!(video_file.size, 4);

        let segments = service
            .fetch_video_units_between("front", begin_time, end_time)
            .expect("video segments fetched");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0.id, video_unit.id);
        assert_eq!(segments[0].0.begin_time, begin_time);
        assert_eq!(segments[0].0.end_time, end_time);

        let old_units = service
            .fetch_storage_group_old_units("primary", 10)
            .expect("old units fetched");
        assert_eq!(old_units.video_units.len(), 1);

        service
            .delete_video_unit(video_unit.id)
            .expect("video unit deleted");
        assert!(!filename.exists());

        let old_units = service
            .fetch_storage_group_old_units("primary", 10)
            .expect("old units fetched");
        assert!(old_units.video_units.is_empty());
    }

    #[test]
    fn reserves_opens_closes_and_cleans_unopened_video_segments() {
        let (temp_dir, service) = migrated_service();
        service
            .apply_config(&sample_config("secret", vec!["front"]))
            .expect("config applied");

        let reserved_at = test_time("2026-05-21T02:00:00Z");
        let begin_time = test_time("2026-05-21T02:00:05Z");
        let end_time = test_time("2026-05-21T02:00:35Z");
        let filename = temp_dir.path().join("reserved.mkv");

        let (video_unit, video_file) = service
            .reserve_video_segment("front", filename.display().to_string(), reserved_at)
            .expect("video segment reserved");
        assert_eq!(video_unit.begin_time, reserved_at);
        assert_eq!(video_unit.end_time, reserved_at);
        assert_eq!(video_file.size, 0);

        let opened_unit = service
            .open_video_segment(video_unit.id, video_file.id, begin_time)
            .expect("video segment opened");
        assert_eq!(opened_unit.begin_time, begin_time);
        assert_eq!(opened_unit.end_time, begin_time);

        std::fs::write(&filename, b"video").expect("video file written");
        let (_closed_unit, closed_file) = service
            .close_video_segment(video_unit.id, video_file.id, end_time, 5)
            .expect("video segment closed");
        assert_eq!(closed_file.size, 5);

        let unopened_filename = temp_dir.path().join("unopened.mkv");
        std::fs::write(&unopened_filename, b"unused").expect("unused file written");
        service
            .reserve_video_segment(
                "front",
                unopened_filename.display().to_string(),
                reserved_at,
            )
            .expect("unopened video segment reserved");

        let deleted_count = service
            .delete_unopened_video_segments("front")
            .expect("unopened video segments deleted");
        assert_eq!(deleted_count, 1);
        assert!(!unopened_filename.exists());

        let segments = service
            .fetch_video_units_between("front", begin_time, end_time)
            .expect("video segments fetched");
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].0.id, video_unit.id);
        assert_eq!(segments[0].1.size, 5);
    }

    #[test]
    fn handles_sessions_tokens_and_expired_cleanup() {
        let (_temp_dir, service) = migrated_service();
        service
            .apply_config(&sample_config("secret", vec!["front"]))
            .expect("config applied");

        let user = service.login("alice", "secret").expect("login succeeds");
        assert_eq!(user.username, "alice");

        let expiration = Utc::now() + Duration::days(1);
        let token_key = service
            .create_user_session(&CreateUserSession {
                name: "ci-token".to_string(),
                user_name: "alice".to_string(),
                session_key: "token-key".to_string(),
                is_token: true,
                expiration,
            })
            .expect("token created");
        assert_eq!(token_key, "token-key");

        let user = service
            .validate_user_session("token-key")
            .expect("session validates");
        assert_eq!(user.username, "alice");

        let tokens = service.fetch_users_tokens("alice").expect("tokens fetched");
        assert_eq!(tokens.len(), 1);
        assert!(tokens[0].id > 0);
        assert_eq!(
            tokens[0].expiration.timestamp_micros(),
            expiration.timestamp_micros()
        );

        service
            .delete_user_session(tokens[0].id)
            .expect("token deleted");
        let tokens = service.fetch_users_tokens("alice").expect("tokens fetched");
        assert!(tokens.is_empty());

        service
            .create_user_session(&CreateUserSession {
                name: String::new(),
                user_name: "alice".to_string(),
                session_key: "expired-key".to_string(),
                is_token: false,
                expiration: Utc::now() - Duration::days(1),
            })
            .expect("expired session created");
        assert!(matches!(
            service.validate_user_session("expired-key"),
            Err(super::Error::NotFound)
        ));
    }

    #[test]
    fn creates_and_closes_video_segments_concurrently() {
        let (_temp_dir, service) = migrated_service();
        service
            .apply_config(&sample_config("secret", vec!["front"]))
            .expect("config applied");

        let begin_time = test_time("2026-05-21T01:00:00Z");
        let end_time = test_time("2026-05-21T01:00:30Z");
        let mut handles = Vec::new();

        for worker in 0..4 {
            let service = service.clone();
            handles.push(thread::spawn(move || {
                for segment in 0..5 {
                    let (video_unit, video_file) = service
                        .create_video_segment(
                            &CreateVideoUnit {
                                camera_name: "front".to_string(),
                                begin_time,
                                end_time: begin_time,
                            },
                            CreateVideoFile {
                                filename: format!("/tmp/segment-{worker}-{segment}.mkv"),
                                size: 0,
                            },
                        )
                        .expect("video segment created");
                    service
                        .close_video_segment(video_unit.id, video_file.id, end_time, 1)
                        .expect("video segment closed");
                }
            }));
        }

        for handle in handles {
            handle.join().expect("worker joined");
        }

        let segments = service
            .fetch_video_units_between("front", begin_time, end_time)
            .expect("video segments fetched");
        assert_eq!(segments.len(), 20);
    }
}
