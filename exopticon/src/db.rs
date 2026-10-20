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
    use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
    use diesel_migrations::MigrationHarness;
    use tempfile::TempDir;

    use super::{
        Service,
        video_units::{RecordingRange, VideoUnit, VideoUnitState},
    };
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
                    onvif_profile_token: None,
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
                    onvif_profile_token: None,
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

    fn configured_service() -> (TempDir, Service) {
        let (temp_dir, service) = migrated_service();
        service
            .apply_config(&sample_config("secret", vec!["front", "back"]))
            .expect("config applied");
        (temp_dir, service)
    }

    fn insert_video_unit(
        service: &Service,
        camera_name: &str,
        state: VideoUnitState,
        begin_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> i64 {
        use crate::schema::video_units;

        diesel::insert_into(video_units::table)
            .values((
                video_units::camera_name.eq(camera_name),
                video_units::state.eq(state),
                video_units::begin_time_us.eq(begin_time.timestamp_micros()),
                video_units::end_time_us.eq(end_time.timestamp_micros()),
            ))
            .returning(video_units::id)
            .get_result(&mut service.pool.get().expect("insert connection"))
            .expect("video unit inserted")
    }

    fn at(seconds: i64) -> DateTime<Utc> {
        test_time("2026-09-22T10:00:00Z") + Duration::seconds(seconds)
    }

    fn insert_segment(
        service: &Service,
        camera_name: &str,
        state: VideoUnitState,
        begin_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        filename: &str,
        size: i32,
    ) -> (i64, i64) {
        use crate::schema::video_files;

        let unit_id = insert_video_unit(service, camera_name, state, begin_time, end_time);
        let file_id = diesel::insert_into(video_files::table)
            .values((
                video_files::video_unit_id.eq(unit_id),
                video_files::filename.eq(filename),
                video_files::size.eq(size),
            ))
            .returning(video_files::id)
            .get_result(&mut service.pool.get().expect("insert connection"))
            .expect("video file inserted");
        (unit_id, file_id)
    }

    fn persisted_unit(service: &Service, unit_id: i64) -> VideoUnit {
        crate::schema::video_units::table
            .find(unit_id)
            .first(&mut service.pool.get().expect("query connection"))
            .expect("video unit loaded")
    }

    fn persisted_file(service: &Service, file_id: i64) -> super::video_units::VideoFile {
        crate::schema::video_files::table
            .find(file_id)
            .first(&mut service.pool.get().expect("query connection"))
            .expect("video file loaded")
    }

    fn remaining_segment_ids(service: &Service) -> (Vec<i64>, Vec<i64>) {
        use crate::schema::{video_files, video_units};

        let mut conn = service.pool.get().expect("query connection");
        let units = video_units::table
            .order(video_units::id)
            .select(video_units::id)
            .load(&mut conn)
            .expect("video unit IDs loaded");
        let files = video_files::table
            .order(video_files::id)
            .select(video_files::id)
            .load(&mut conn)
            .expect("video file IDs loaded");
        (units, files)
    }

    fn create_session(
        service: &Service,
        user_name: &str,
        session_key: &str,
        is_token: bool,
        expiration: DateTime<Utc>,
    ) -> String {
        service
            .create_user_session(&CreateUserSession {
                name: "ci-token".to_string(),
                user_name: user_name.to_string(),
                session_key: session_key.to_string(),
                is_token,
                expiration,
            })
            .expect("session created")
    }

    // Session validation uses the wall clock. These bounds keep expiration
    // well away from it without depending on test execution speed.
    fn unexpired() -> DateTime<Utc> {
        test_time("9999-01-01T00:00:00Z")
    }

    #[test]
    fn recording_ranges_exclude_nonfinalized_units_with_positive_intervals() {
        let (_dir, service) = configured_service();
        let base = test_time("2026-09-22T10:00:00Z");
        let at = |seconds: i64| base + Duration::seconds(seconds);

        insert_video_unit(&service, "front", VideoUnitState::Reserved, at(0), at(1));
        insert_video_unit(&service, "front", VideoUnitState::Open, at(2), at(3));
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(4), at(5));

        let ranges = service
            .fetch_recording_ranges("front", at(0), at(6))
            .expect("recording ranges fetched");

        assert_eq!(
            ranges,
            vec![super::video_units::RecordingRange {
                begin_time: at(4),
                end_time: at(5),
            }]
        );
    }

    #[test]
    fn video_unit_listing_excludes_nonfinalized_units() {
        let (_dir, service) = configured_service();
        insert_segment(
            &service,
            "front",
            VideoUnitState::Reserved,
            at(0),
            at(1),
            "reserved.mkv",
            5,
        );
        insert_segment(
            &service,
            "front",
            VideoUnitState::Open,
            at(0),
            at(1),
            "open.mkv",
            5,
        );
        let finalized = insert_segment(
            &service,
            "front",
            VideoUnitState::Finalized,
            at(0),
            at(1),
            "finalized.mkv",
            0,
        );

        let segments = service
            .fetch_video_units_between("front", at(0), at(2))
            .expect("video segments fetched");

        assert_eq!(
            segments
                .iter()
                .map(|(unit, file)| (unit.id, file.id))
                .collect::<Vec<_>>(),
            vec![finalized]
        );
    }

    #[test]
    fn retention_counts_and_selects_only_finalized_units() {
        let (_dir, service) = configured_service();
        insert_segment(
            &service,
            "front",
            VideoUnitState::Reserved,
            at(0),
            at(1),
            "reserved.mkv",
            11,
        );
        insert_segment(
            &service,
            "front",
            VideoUnitState::Open,
            at(0),
            at(1),
            "open.mkv",
            13,
        );
        let (empty, _) = insert_segment(
            &service,
            "front",
            VideoUnitState::Finalized,
            at(2),
            at(3),
            "empty.mkv",
            0,
        );
        let (finalized, _) = insert_segment(
            &service,
            "front",
            VideoUnitState::Finalized,
            at(4),
            at(5),
            "finalized.mkv",
            7,
        );

        let old = service
            .fetch_storage_group_old_units("primary", 10)
            .expect("retention candidates fetched");

        assert_eq!(old.storage_group_size, 7);
        assert_eq!(
            old.video_units
                .iter()
                .map(|(_, unit, _)| unit.id)
                .collect::<Vec<_>>(),
            vec![empty, finalized]
        );
    }

    #[test]
    fn unopened_cleanup_deletes_only_reserved_segments_regardless_of_timestamps_and_size() {
        let (dir, service) = configured_service();
        let reserved_path = dir.path().join("reserved.mkv");
        let open_path = dir.path().join("open.mkv");
        let finalized_path = dir.path().join("finalized.mkv");
        let other_camera_path = dir.path().join("back.mkv");
        for path in [
            &reserved_path,
            &open_path,
            &finalized_path,
            &other_camera_path,
        ] {
            std::fs::write(path, b"video").expect("video file written");
        }
        insert_segment(
            &service,
            "front",
            VideoUnitState::Reserved,
            at(0),
            at(1),
            reserved_path.to_str().unwrap(),
            5,
        );
        let open = insert_segment(
            &service,
            "front",
            VideoUnitState::Open,
            at(0),
            at(0),
            open_path.to_str().unwrap(),
            0,
        );
        let finalized = insert_segment(
            &service,
            "front",
            VideoUnitState::Finalized,
            at(0),
            at(0),
            finalized_path.to_str().unwrap(),
            0,
        );
        let other_camera = insert_segment(
            &service,
            "back",
            VideoUnitState::Reserved,
            at(0),
            at(0),
            other_camera_path.to_str().unwrap(),
            0,
        );

        let deleted = service
            .delete_unopened_video_segments("front")
            .expect("reserved segments deleted");

        assert_eq!(deleted, 1);
        assert!(!reserved_path.exists());
        assert!(open_path.exists());
        assert!(finalized_path.exists());
        assert!(other_camera_path.exists());
        assert_eq!(
            remaining_segment_ids(&service),
            (
                vec![open.0, finalized.0, other_camera.0],
                vec![open.1, finalized.1, other_camera.1]
            )
        );
    }

    #[test]
    fn applying_config_persists_camera_group_memberships() {
        let (_dir, service) = migrated_service();
        let config = sample_config("secret", vec!["front", "back"]);

        service.apply_config(&config).expect("config applied");

        let group = service
            .fetch_camera_group("all")
            .expect("camera group fetched");
        assert_eq!(group.spec.members, vec!["front", "back"]);
    }

    #[test]
    fn applying_config_replaces_camera_group_memberships() {
        let (_dir, service) = configured_service();
        let config = sample_config("secret", vec!["back"]);

        service
            .apply_config(&config)
            .expect("updated config applied");

        let group = service
            .fetch_camera_group("all")
            .expect("camera group fetched");
        assert_eq!(group.spec.members, vec!["back"]);
    }

    #[test]
    fn applying_config_persists_storage_group_metadata() {
        let (_dir, service) = migrated_service();
        let mut config = sample_config("secret", vec![]);
        config.storage_groups[0].display_name = "Archive".to_string();

        service.apply_config(&config).expect("config applied");

        let group = service
            .fetch_storage_group("primary")
            .expect("storage group fetched");
        assert_eq!(group.metadata.display_name, "Archive");
    }

    #[test]
    fn creating_a_segment_persists_a_reserved_unit_and_its_file() {
        let (_dir, service) = configured_service();

        let (unit, file) = service
            .create_video_segment(
                &CreateVideoUnit {
                    camera_name: "front".to_string(),
                    begin_time: at(0),
                    end_time: at(0),
                },
                CreateVideoFile {
                    filename: "segment.mkv".to_string(),
                    size: 0,
                },
            )
            .expect("video segment created");

        assert!(unit.id > 0);
        assert!(file.id > 0);
        assert_eq!(file.video_unit_id, unit.id);
        let stored_unit = persisted_unit(&service, unit.id);
        assert_eq!(stored_unit.camera_name, "front");
        assert_eq!(stored_unit.begin_time_us, at(0).timestamp_micros());
        assert_eq!(stored_unit.end_time_us, at(0).timestamp_micros());
        assert!(matches!(stored_unit.state, VideoUnitState::Reserved));
        let stored_file = persisted_file(&service, file.id);
        assert_eq!(stored_file.video_unit_id, unit.id);
        assert_eq!(stored_file.filename, "segment.mkv");
        assert_eq!(stored_file.size, 0);
    }

    #[test]
    fn reserving_a_segment_persists_the_reservation_time_and_an_empty_file_record() {
        let (_dir, service) = configured_service();

        let (unit, file) = service
            .reserve_video_segment("front", "reserved.mkv".to_string(), at(0))
            .expect("video segment reserved");

        assert_eq!(unit.begin_time, at(0));
        assert_eq!(unit.end_time, at(0));
        assert_eq!(file.size, 0);
        let stored = persisted_unit(&service, unit.id);
        assert!(matches!(stored.state, VideoUnitState::Reserved));
        assert_eq!(stored.camera_name, "front");
        assert_eq!(stored.begin_time_us, at(0).timestamp_micros());
        assert_eq!(stored.end_time_us, at(0).timestamp_micros());
        let stored_file = persisted_file(&service, file.id);
        assert_eq!(stored_file.video_unit_id, unit.id);
        assert_eq!(stored_file.filename, "reserved.mkv");
        assert_eq!(stored_file.size, 0);
    }

    #[test]
    fn opening_a_reserved_segment_persists_the_actual_begin_time_and_open_state() {
        let (_dir, service) = configured_service();
        let (unit_id, file_id) = insert_segment(
            &service,
            "front",
            VideoUnitState::Reserved,
            at(0),
            at(0),
            "segment.mkv",
            0,
        );

        let opened = service
            .open_video_segment(unit_id, file_id, at(5))
            .expect("video segment opened");

        assert_eq!(opened.begin_time, at(5));
        assert_eq!(opened.end_time, at(5));
        let stored = persisted_unit(&service, unit_id);
        assert!(matches!(stored.state, VideoUnitState::Open));
        assert_eq!(stored.begin_time_us, at(5).timestamp_micros());
        assert_eq!(stored.end_time_us, at(5).timestamp_micros());
        assert_eq!(persisted_file(&service, file_id).size, -1);
    }

    #[test]
    fn closing_an_open_segment_persists_the_finalized_interval_and_size() {
        let (_dir, service) = configured_service();
        let (unit_id, file_id) = insert_segment(
            &service,
            "front",
            VideoUnitState::Open,
            at(5),
            at(5),
            "segment.mkv",
            -1,
        );

        let (closed, file) = service
            .close_video_segment(unit_id, file_id, at(35), 5)
            .expect("video segment closed");

        assert_eq!(closed.begin_time, at(5));
        assert_eq!(closed.end_time, at(35));
        assert_eq!(file.size, 5);
        let stored = persisted_unit(&service, unit_id);
        assert!(matches!(stored.state, VideoUnitState::Finalized));
        assert_eq!(stored.begin_time_us, at(5).timestamp_micros());
        assert_eq!(stored.end_time_us, at(35).timestamp_micros());
        assert_eq!(persisted_file(&service, file_id).size, 5);
    }

    #[test]
    fn video_unit_listing_returns_the_finalized_interval_and_file_size() {
        let (_dir, service) = configured_service();
        let (unit_id, file_id) = insert_segment(
            &service,
            "front",
            VideoUnitState::Finalized,
            at(5),
            at(35),
            "segment.mkv",
            5,
        );

        let segments = service
            .fetch_video_units_between("front", at(0), at(40))
            .expect("video segments fetched");

        assert_eq!(segments.len(), 1);
        let (unit, file) = &segments[0];
        assert_eq!((unit.id, file.id), (unit_id, file_id));
        assert_eq!(unit.begin_time, at(5));
        assert_eq!(unit.end_time, at(35));
        assert_eq!(file.size, 5);
    }

    #[test]
    fn deleting_a_video_unit_removes_its_file_and_database_records() {
        let (dir, service) = configured_service();
        let filename = dir.path().join("segment.mkv");
        std::fs::write(&filename, b"video").expect("video file written");
        let (unit_id, _) = insert_segment(
            &service,
            "front",
            VideoUnitState::Finalized,
            at(0),
            at(5),
            filename.to_str().unwrap(),
            5,
        );

        service
            .delete_video_unit(unit_id)
            .expect("video unit deleted");

        assert!(!filename.exists());
        assert_eq!(remaining_segment_ids(&service), (vec![], vec![]));
        let old = service
            .fetch_storage_group_old_units("primary", 10)
            .expect("retention candidates fetched");
        assert!(old.video_units.is_empty());
        assert_eq!(old.storage_group_size, 0);
    }

    #[test]
    fn recording_ranges_are_empty_without_recordings() {
        let (_dir, service) = configured_service();

        let ranges = service
            .fetch_recording_ranges("front", at(5), at(30))
            .expect("recording ranges fetched");

        assert!(ranges.is_empty());
    }

    #[test]
    fn touching_recording_ranges_are_merged() {
        let (_dir, service) = configured_service();
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(15), at(20));
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(5), at(15));

        let ranges = service
            .fetch_recording_ranges("front", at(5), at(30))
            .expect("recording ranges fetched");

        assert_eq!(
            ranges,
            vec![RecordingRange {
                begin_time: at(5),
                end_time: at(20)
            },]
        );
    }

    #[test]
    fn overlapping_recording_ranges_are_merged() {
        let (_dir, service) = configured_service();
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(5), at(15));
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(10), at(20));

        let ranges = service
            .fetch_recording_ranges("front", at(5), at(30))
            .expect("recording ranges fetched");

        assert_eq!(
            ranges,
            vec![RecordingRange {
                begin_time: at(5),
                end_time: at(20)
            },]
        );
    }

    #[test]
    fn nested_recording_ranges_do_not_shorten_the_outer_range() {
        let (_dir, service) = configured_service();
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(5), at(25));
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(10), at(15));
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(20), at(30));

        let ranges = service
            .fetch_recording_ranges("front", at(5), at(30))
            .expect("recording ranges fetched");

        assert_eq!(
            ranges,
            vec![RecordingRange {
                begin_time: at(5),
                end_time: at(30)
            },]
        );
    }

    #[test]
    fn gaps_between_recording_ranges_are_preserved() {
        let (_dir, service) = configured_service();
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(20), at(25));
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(5), at(10));
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(12), at(15));

        let ranges = service
            .fetch_recording_ranges("front", at(5), at(30))
            .expect("recording ranges fetched");

        assert_eq!(
            ranges,
            vec![
                RecordingRange {
                    begin_time: at(5),
                    end_time: at(10)
                },
                RecordingRange {
                    begin_time: at(12),
                    end_time: at(15)
                },
                RecordingRange {
                    begin_time: at(20),
                    end_time: at(25)
                },
            ]
        );
    }

    #[test]
    fn recording_ranges_are_clipped_at_the_request_begin() {
        let (_dir, service) = configured_service();
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(0), at(15));

        let ranges = service
            .fetch_recording_ranges("front", at(5), at(30))
            .expect("recording ranges fetched");

        assert_eq!(
            ranges,
            vec![RecordingRange {
                begin_time: at(5),
                end_time: at(15)
            },]
        );
    }

    #[test]
    fn recording_ranges_are_clipped_at_the_request_end() {
        let (_dir, service) = configured_service();
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(20), at(35));

        let ranges = service
            .fetch_recording_ranges("front", at(5), at(30))
            .expect("recording ranges fetched");

        assert_eq!(
            ranges,
            vec![RecordingRange {
                begin_time: at(20),
                end_time: at(30)
            },]
        );
    }

    #[test]
    fn recording_ranges_ending_at_the_request_begin_are_excluded() {
        let (_dir, service) = configured_service();
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(0), at(5));

        let ranges = service
            .fetch_recording_ranges("front", at(5), at(30))
            .expect("recording ranges fetched");

        assert!(ranges.is_empty());
    }

    #[test]
    fn recording_ranges_beginning_at_the_request_end_are_excluded() {
        let (_dir, service) = configured_service();
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(30), at(35));

        let ranges = service
            .fetch_recording_ranges("front", at(5), at(30))
            .expect("recording ranges fetched");

        assert!(ranges.is_empty());
    }

    #[test]
    fn recording_ranges_with_zero_duration_are_excluded() {
        let (_dir, service) = configured_service();
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(15), at(15));

        let ranges = service
            .fetch_recording_ranges("front", at(5), at(30))
            .expect("recording ranges fetched");

        assert!(ranges.is_empty());
    }

    #[test]
    fn recording_ranges_with_reversed_timestamps_are_excluded() {
        let (_dir, service) = configured_service();
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(20), at(15));

        let ranges = service
            .fetch_recording_ranges("front", at(5), at(30))
            .expect("recording ranges fetched");

        assert!(ranges.is_empty());
    }

    #[test]
    fn recording_ranges_exclude_other_cameras() {
        let (_dir, service) = configured_service();
        insert_video_unit(&service, "front", VideoUnitState::Finalized, at(5), at(10));
        insert_video_unit(&service, "back", VideoUnitState::Finalized, at(10), at(20));

        let ranges = service
            .fetch_recording_ranges("front", at(5), at(30))
            .expect("recording ranges fetched");

        assert_eq!(
            ranges,
            vec![RecordingRange {
                begin_time: at(5),
                end_time: at(10)
            },]
        );
    }

    #[test]
    fn recording_ranges_are_not_limited_to_999_units() {
        let (_dir, service) = configured_service();
        for index in 0..1_001 {
            insert_video_unit(
                &service,
                "front",
                VideoUnitState::Finalized,
                at(index * 2),
                at(index * 2 + 1),
            );
        }

        let ranges = service
            .fetch_recording_ranges("front", at(0), at(2_002))
            .expect("recording ranges fetched");

        assert_eq!(ranges.len(), 1_001);
        assert_eq!(
            ranges[0],
            RecordingRange {
                begin_time: at(0),
                end_time: at(1)
            }
        );
        assert_eq!(
            ranges[1_000],
            RecordingRange {
                begin_time: at(2_000),
                end_time: at(2_001)
            }
        );
    }

    #[test]
    fn recording_ranges_reject_an_unknown_camera() {
        let (_dir, service) = configured_service();

        let result = service.fetch_recording_ranges("missing", at(0), at(900));

        assert!(matches!(result, Err(super::Error::NotFound)));
    }

    #[test]
    fn deleting_a_video_unit_removes_empty_parent_directories_within_the_storage_root() {
        let (dir, service) = migrated_service();
        let storage_root = dir.path().join("video");
        let mut config = sample_config("secret", vec!["front"]);
        config.storage_groups[0].storage_path = storage_root.display().to_string();
        service.apply_config(&config).expect("config applied");
        let segment_dir = storage_root.join("front/2026/09/22/10");
        std::fs::create_dir_all(&segment_dir).expect("segment directory created");
        let filename = segment_dir.join("segment.mkv");
        std::fs::write(&filename, b"video").expect("video file written");
        let (unit_id, _) = insert_segment(
            &service,
            "front",
            VideoUnitState::Finalized,
            at(0),
            at(5),
            filename.to_str().unwrap(),
            5,
        );

        service
            .delete_video_unit(unit_id)
            .expect("video unit deleted");

        assert!(!filename.exists());
        assert!(!segment_dir.exists());
        assert!(!storage_root.join("front").exists());
        assert!(storage_root.exists());
        assert_eq!(remaining_segment_ids(&service), (vec![], vec![]));
    }

    #[test]
    fn login_accepts_the_configured_password() {
        let (_dir, service) = migrated_service();
        service
            .apply_config(&sample_config("secret", vec![]))
            .expect("config applied");

        let user = service.login("alice", "secret").expect("login succeeds");

        assert_eq!(user.username, "alice");
    }

    #[test]
    fn creating_a_session_returns_and_persists_its_key() {
        use crate::schema::user_sessions;
        let (_dir, service) = configured_service();

        let key = service
            .create_user_session(&CreateUserSession {
                name: "ci-token".to_string(),
                user_name: "alice".to_string(),
                session_key: "token-key".to_string(),
                is_token: true,
                expiration: unexpired(),
            })
            .expect("token created");

        assert_eq!(key, "token-key");
        let stored = user_sessions::table
            .first::<super::auth::UserSession>(&mut service.pool.get().expect("query connection"))
            .expect("session loaded");
        assert_eq!(stored.session_key, "token-key");
        assert_eq!(stored.user_name, "alice");
        assert_eq!(stored.name, "ci-token");
        assert!(stored.is_token);
        assert_eq!(stored.expiration_us, unexpired().timestamp_micros());
    }

    #[test]
    fn unexpired_sessions_validate_as_their_user() {
        let (_dir, service) = configured_service();
        create_session(&service, "alice", "token-key", true, unexpired());

        let user = service
            .validate_user_session("token-key")
            .expect("session validates");

        assert_eq!(user.username, "alice");
    }

    #[test]
    fn token_listing_excludes_regular_sessions() {
        let (_dir, service) = configured_service();
        create_session(&service, "alice", "session-key", false, unexpired());
        create_session(&service, "alice", "token-key", true, unexpired());

        let tokens = service.fetch_users_tokens("alice").expect("tokens fetched");

        assert_eq!(tokens.len(), 1);
        assert!(tokens[0].id > 0);
        assert_eq!(tokens[0].name, "ci-token");
        assert_eq!(tokens[0].user_name, "alice");
        assert_eq!(tokens[0].expiration, unexpired());
    }

    #[test]
    fn deleting_a_session_removes_its_database_record() {
        use crate::schema::user_sessions;
        let (_dir, service) = configured_service();
        create_session(&service, "alice", "token-key", true, unexpired());
        let tokens = service.fetch_users_tokens("alice").expect("tokens fetched");
        let session_id = tokens[0].id;

        service
            .delete_user_session(session_id)
            .expect("token deleted");

        assert!(
            service
                .fetch_users_tokens("alice")
                .expect("tokens fetched")
                .is_empty()
        );
        let count: i64 = user_sessions::table
            .count()
            .get_result(&mut service.pool.get().expect("query connection"))
            .expect("sessions counted");
        assert_eq!(count, 0);
    }

    #[test]
    fn expired_sessions_are_rejected() {
        let (_dir, service) = configured_service();
        create_session(
            &service,
            "alice",
            "expired-key",
            false,
            DateTime::UNIX_EPOCH,
        );

        let result = service.validate_user_session("expired-key");

        assert!(matches!(result, Err(super::Error::NotFound)));
    }

    #[test]
    fn successful_session_validation_deletes_expired_sessions() {
        use crate::schema::user_sessions;
        let (_dir, service) = configured_service();
        create_session(
            &service,
            "alice",
            "expired-key",
            false,
            DateTime::UNIX_EPOCH,
        );
        create_session(&service, "alice", "valid-key", false, unexpired());

        service
            .validate_user_session("valid-key")
            .expect("session validates");

        let remaining: Vec<String> = user_sessions::table
            .select(user_sessions::session_key)
            .load(&mut service.pool.get().expect("query connection"))
            .expect("remaining sessions loaded");
        assert_eq!(remaining, vec!["valid-key"]);
    }

    #[test]
    fn concurrent_segment_writes_persist_every_finalized_segment() {
        let (dir, service) = configured_service();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let mut handles = Vec::new();
        for worker in 0..4 {
            let service = service.clone();
            let barrier = barrier.clone();
            let directory = dir.path().to_path_buf();
            handles.push(thread::spawn(move || {
                barrier.wait();
                for segment in 0..5 {
                    let filename = directory.join(format!("segment-{worker}-{segment}.mkv"));
                    let (unit, file) = service
                        .create_video_segment(
                            &CreateVideoUnit {
                                camera_name: "front".to_string(),
                                begin_time: at(0),
                                end_time: at(0),
                            },
                            CreateVideoFile {
                                filename: filename.display().to_string(),
                                size: 0,
                            },
                        )
                        .expect("video segment created");
                    service
                        .close_video_segment(unit.id, file.id, at(30), 1)
                        .expect("video segment closed");
                }
            }));
        }
        for handle in handles {
            handle.join().expect("worker joined");
        }

        let segments = service
            .fetch_video_units_between("front", at(0), at(30))
            .expect("video segments fetched");

        assert_eq!(segments.len(), 20);
        let filenames: std::collections::HashSet<_> = segments
            .iter()
            .map(|(_, file)| file.filename.clone())
            .collect();
        assert_eq!(filenames.len(), 20);
        for worker in 0..4 {
            for segment in 0..5 {
                assert!(
                    filenames.contains(
                        &dir.path()
                            .join(format!("segment-{worker}-{segment}.mkv"))
                            .display()
                            .to_string()
                    )
                );
            }
        }
        for (unit, file) in segments {
            assert_eq!(unit.camera_name, "front");
            assert_eq!(unit.begin_time, at(0));
            assert_eq!(unit.end_time, at(30));
            assert!(matches!(
                unit.state,
                crate::api::video_units::VideoUnitState::Finalized
            ));
            assert_eq!(file.video_unit_id, unit.id);
            assert_eq!(file.size, 1);
        }
    }
}
