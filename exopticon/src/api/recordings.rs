/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2026 David Matthew Mattli <dmm@mattli.us>
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

use axum::{
    Json, Router,
    body::Body,
    extract::{FromRef, Path, Query, RawQuery, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, Utc};
use std::io;
use tokio::task::spawn_blocking;
use tokio_util::io::ReaderStream;

use crate::AppState;

impl FromRef<AppState> for crate::db::Service {
    fn from_ref(state: &AppState) -> Self {
        state.db_service.clone()
    }
}

/// The half-open window in which to report recodings.
#[derive(Debug, Deserialize)]
pub struct Interval {
    begin_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

/// Merged, helf-open interval backed by closed recordings.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingRange {
    begin_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
}

impl From<crate::db::video_units::RecordingRange> for RecordingRange {
    fn from(range: crate::db::video_units::RecordingRange) -> Self {
        Self {
            begin_time: range.begin_time,
            end_time: range.end_time,
        }
    }
}

/// Recoding availability
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RecordingRanges {
    ranges: Vec<RecordingRange>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingDescriptor {
    file_id: i64,
    begin_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    byte_length: i64,
    content_url: String,
    next_url: String,
}

impl RecordingDescriptor {
    fn new(camera_name: &str, file: &crate::db::video_units::RecordingFile) -> Self {
        let mut url = url::Url::parse("http://localhost/v1/recordings").expect("static URL");
        url.path_segments_mut()
            .expect("URL has path segments")
            .push(camera_name)
            .push("files")
            .push(&file.id.to_string());
        let base = url.path().to_string();
        Self {
            file_id: file.id,
            begin_time: file.begin_time,
            end_time: file.end_time,
            byte_length: file.byte_length,
            content_url: format!("{base}/content"),
            next_url: format!("{base}/next"),
        }
    }
}

#[derive(Debug, Serialize)]
struct NextRecording {
    recording: Option<RecordingDescriptor>,
}

#[derive(Debug)]
enum RecordingError {
    InvalidTimestamp,
    InvalidFileId,
    UnknownCamera,
    UnavailableTime,
    UnavailableFile,
    Internal(crate::api::UserError),
}

impl IntoResponse for RecordingError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::InvalidTimestamp => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_timestamp"),
            Self::InvalidFileId => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_file_id"),
            Self::UnknownCamera => (StatusCode::NOT_FOUND, "unknown_camera"),
            Self::UnavailableTime => (StatusCode::NOT_FOUND, "unavailable_time"),
            Self::UnavailableFile => (StatusCode::GONE, "unavailable_file"),
            Self::Internal(err) => {
                log::error!("Recording API internal error: {err:?}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
            }
        };
        (status, Json(serde_json::json!({ "error": code }))).into_response()
    }
}

fn parse_at(raw: Option<String>) -> Result<DateTime<Utc>, RecordingError> {
    let raw = raw.ok_or(RecordingError::InvalidTimestamp)?;
    let mut values = url::form_urlencoded::parse(raw.as_bytes())
        .filter(|(key, _)| key == "at")
        .map(|(_, value)| value.into_owned());
    let at = values.next().ok_or(RecordingError::InvalidTimestamp)?;
    if values.next().is_some() {
        return Err(RecordingError::InvalidTimestamp);
    }
    DateTime::parse_from_rfc3339(&at)
        .map(|at| at.with_timezone(&Utc))
        .map_err(|_| RecordingError::InvalidTimestamp)
}

fn parse_file_id(file_id: &str) -> Result<i64, RecordingError> {
    file_id
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or(RecordingError::InvalidFileId)
}

async fn resolve_recording(
    Path(camera_name): Path<String>,
    RawQuery(raw_query): RawQuery,
    State(db): State<crate::db::Service>,
) -> Result<Json<RecordingDescriptor>, RecordingError> {
    let at = parse_at(raw_query)?;
    let name = camera_name.clone();
    let file = spawn_blocking(move || db.resolve_recording_file(&name, at))
        .await
        .map_err(|err| RecordingError::Internal(err.into()))?
        .map_err(|err| match err {
            crate::db::Error::NotFound => RecordingError::UnknownCamera,
            err @ crate::db::Error::Other(_) => RecordingError::Internal(err.into()),
        })?
        .ok_or(RecordingError::UnavailableTime)?;
    Ok(Json(RecordingDescriptor::new(&camera_name, &file)))
}

async fn next_recording(
    Path((camera_name, raw_file_id)): Path<(String, String)>,
    State(db): State<crate::db::Service>,
) -> Result<Json<NextRecording>, RecordingError> {
    let file_id = parse_file_id(&raw_file_id)?;
    let name = camera_name.clone();
    let camera_db = db.clone();
    let next = spawn_blocking(move || db.next_recording_file(&name, file_id))
        .await
        .map_err(|err| RecordingError::Internal(err.into()))?;
    let next = match next {
        Ok(next) => next,
        Err(crate::db::Error::NotFound) => {
            let check_name = camera_name.clone();
            let exists = spawn_blocking(move || camera_db.recording_camera_exists(&check_name))
                .await
                .map_err(|err| RecordingError::Internal(err.into()))?
                .map_err(|err| RecordingError::Internal(err.into()))?;
            return Err(if exists {
                RecordingError::UnavailableFile
            } else {
                RecordingError::UnknownCamera
            });
        }
        Err(err @ crate::db::Error::Other(_)) => {
            return Err(RecordingError::Internal(err.into()));
        }
    };
    Ok(Json(NextRecording {
        recording: next.map(|file| RecordingDescriptor::new(&camera_name, &file)),
    }))
}

async fn recording_content(
    Path((camera_name, raw_file_id)): Path<(String, String)>,
    State(db): State<crate::db::Service>,
) -> Result<Response, RecordingError> {
    let file_id = parse_file_id(&raw_file_id)?;
    let file = spawn_blocking(move || db.get_recording_file(&camera_name, file_id))
        .await
        .map_err(|err| RecordingError::Internal(err.into()))?
        .map_err(|err| match err {
            crate::db::Error::NotFound => RecordingError::UnknownCamera,
            err @ crate::db::Error::Other(_) => RecordingError::Internal(err.into()),
        })?
        .ok_or(RecordingError::UnavailableFile)?;

    let handle = tokio::fs::File::open(&file.filename)
        .await
        .map_err(|err| recording_file_open_error(&err))?;
    let metadata = handle.metadata().await.map_err(|err| {
        RecordingError::Internal(crate::api::UserError::InternalError(err.to_string()))
    })?;
    if !metadata.is_file() || i64::try_from(metadata.len()).ok() != Some(file.byte_length) {
        return Err(RecordingError::UnavailableFile);
    }

    let body = Body::from_stream(ReaderStream::with_capacity(handle, 64 * 1024));
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "video/x-matroska")
        .header(header::CONTENT_LENGTH, metadata.len())
        .header(header::CACHE_CONTROL, "private, no-transform")
        .body(body)
        .expect("static response headers"))
}

fn recording_file_open_error(err: &io::Error) -> RecordingError {
    if err.kind() == io::ErrorKind::NotFound {
        RecordingError::UnavailableFile
    } else {
        RecordingError::Internal(crate::api::UserError::InternalError(err.to_string()))
    }
}

/// Returns sorted, merged recording ranges clipped to the requested
/// half-open window.
pub async fn fetch_recording_ranges(
    Query(interval): Query<Interval>,
    Path(camera_name): Path<String>,
    State(db): State<crate::db::Service>,
) -> Result<Json<RecordingRanges>, super::UserError> {
    if interval.begin_time >= interval.end_time {
        return Err(super::UserError::Validation(
            "begin_time must be before end_time".to_string(),
        ));
    }

    let ranges = spawn_blocking(move || {
        db.fetch_recording_ranges(&camera_name, interval.begin_time, interval.end_time)
    })
    .await??;

    Ok(Json(RecordingRanges {
        ranges: ranges.into_iter().map(Into::into).collect(),
    }))
}

pub fn router() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/{camera_name}", get(fetch_recording_ranges))
        .route("/{camera_name}/samples", get(resolve_recording))
        .route("/{camera_name}/files/{file_id}/next", get(next_recording))
        .route(
            "/{camera_name}/files/{file_id}/content",
            get(recording_content)
                .head(|| async { (StatusCode::METHOD_NOT_ALLOWED, [(header::ALLOW, "GET")]) }),
        )
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        response::Response,
    };
    use chrono::DateTime;
    use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, connection::SimpleConnection};
    use diesel_migrations::MigrationHarness;
    use serde_json::{Value, json};
    use tempfile::TempDir;
    use tower::ServiceExt;

    use super::{
        fetch_recording_ranges, next_recording, recording_content, recording_file_open_error,
        resolve_recording,
    };
    use crate::db::Service;

    fn test_app() -> (TempDir, Service, Router) {
        let temp_dir = TempDir::new().expect("temp dir created");
        let database_path = temp_dir.path().join("exopticon.sqlite");
        let service = Service::new(&database_path.display().to_string());

        let mut conn = service.pool.get().expect("migration connection");
        conn.run_pending_migrations(crate::MIGRATIONS)
            .expect("migrations run");
        conn.batch_execute(
            "
            INSERT INTO storage_groups
                (name, display_name, storage_path, max_storage_size)
            VALUES ('primary', 'Primary', '/video', 1024);

            INSERT INTO cameras
                (name, display_name, storage_group_name, ip, onvif_port, mac,
                 username, password, rtsp_url, ptz_type, onvif_profile_token,
                 enabled, ptz_x_step_size, ptz_y_step_size)
            VALUES
                ('front', 'Front', 'primary', '192.0.2.10', 80,
                 '00:00:00:00:00:01', 'camera-user', 'camera-password',
                 'rtsp://front.example/stream', 'none', NULL, 1, 1, 1);
            ",
        )
        .expect("camera inserted");
        drop(conn);

        let app = Router::new()
            .route("/{camera_name}", axum::routing::get(fetch_recording_ranges))
            .route(
                "/{camera_name}/samples",
                axum::routing::get(resolve_recording),
            )
            .route(
                "/{camera_name}/files/{file_id}/next",
                axum::routing::get(next_recording),
            )
            .route(
                "/{camera_name}/files/{file_id}/content",
                axum::routing::get(recording_content).head(|| async {
                    (
                        StatusCode::METHOD_NOT_ALLOWED,
                        [(axum::http::header::ALLOW, "GET")],
                    )
                }),
            )
            .with_state(service.clone());

        (temp_dir, service, app)
    }

    async fn get(app: &Router, uri: &str) -> Response {
        app.clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request built"),
            )
            .await
            .expect("request completed")
    }

    async fn get_with_headers(
        app: &Router,
        uri: &str,
        method: &str,
        headers: &[(&str, &str)],
    ) -> Response {
        let mut request = Request::builder().method(method).uri(uri);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        app.clone()
            .oneshot(request.body(Body::empty()).expect("request built"))
            .await
            .expect("request completed")
    }

    async fn json_body(response: Response) -> Value {
        let body = to_bytes(response.into_body(), 1_024)
            .await
            .expect("response body read");
        serde_json::from_slice(&body).expect("JSON response parsed")
    }

    #[tokio::test]
    async fn file_open_errors_keep_missing_and_server_failures_distinct() {
        use axum::response::IntoResponse;

        for (kind, status, code) in [
            (
                std::io::ErrorKind::NotFound,
                StatusCode::GONE,
                "unavailable_file",
            ),
            (
                std::io::ErrorKind::PermissionDenied,
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
            ),
            (
                std::io::ErrorKind::Other,
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
            ),
        ] {
            let response = recording_file_open_error(&std::io::Error::from(kind)).into_response();
            assert_eq!(response.status(), status);
            assert_eq!(json_body(response).await, json!({"error": code}));
        }
    }

    fn add_file(
        service: &Service,
        camera: &str,
        begin: &str,
        end: &str,
        filename: &str,
        size: i32,
    ) -> i64 {
        use crate::schema::{video_files, video_units};
        let begin_us = DateTime::parse_from_rfc3339(begin)
            .unwrap()
            .timestamp_micros();
        let end_us = DateTime::parse_from_rfc3339(end)
            .unwrap()
            .timestamp_micros();
        let mut conn = service.pool.get().unwrap();
        let unit_id: i64 = diesel::insert_into(video_units::table)
            .values((
                video_units::camera_name.eq(camera),
                video_units::begin_time_us.eq(begin_us),
                video_units::end_time_us.eq(end_us),
            ))
            .returning(video_units::id)
            .get_result(&mut conn)
            .unwrap();
        diesel::insert_into(video_files::table)
            .values((
                video_files::filename.eq(filename),
                video_files::size.eq(size),
                video_files::video_unit_id.eq(unit_id),
            ))
            .returning(video_files::id)
            .get_result(&mut conn)
            .unwrap()
    }

    #[tokio::test]
    async fn resolves_half_open_intervals_overlaps_and_gaps() {
        let (dir, service, app) = test_app();
        let path = dir.path().join("one.mkv");
        std::fs::write(&path, b"mkv!").unwrap();
        let name = path.to_str().unwrap();
        let first = add_file(
            &service,
            "front",
            "2026-09-23T10:00:00Z",
            "2026-09-23T10:00:10Z",
            name,
            4,
        );
        let second = add_file(
            &service,
            "front",
            "2026-09-23T10:00:05Z",
            "2026-09-23T10:00:12Z",
            name,
            4,
        );
        let tie = add_file(
            &service,
            "front",
            "2026-09-23T10:00:05Z",
            "2026-09-23T10:00:09Z",
            name,
            4,
        );
        add_file(
            &service,
            "front",
            "2026-09-23T10:00:12Z",
            "2026-09-23T10:00:20Z",
            name,
            -1,
        );
        assert_eq!(
            json_body(get(&app, "/front/samples?at=2026-09-23T10%3A00%3A00Z").await).await,
            json!({
                "fileId": first,
                "beginTime": "2026-09-23T10:00:00Z",
                "endTime": "2026-09-23T10:00:10Z",
                "byteLength": 4,
                "contentUrl": format!("/v1/recordings/front/files/{first}/content"),
                "nextUrl": format!("/v1/recordings/front/files/{first}/next")
            })
        );
        assert_eq!(
            json_body(get(&app, "/front/samples?at=2026-09-23T10%3A00%3A07Z").await).await["fileId"],
            tie
        );
        assert_eq!(
            json_body(get(&app, "/front/samples?at=2026-09-23T10%3A00%3A09Z").await).await["fileId"],
            second
        );
        assert_eq!(
            json_body(get(&app, "/front/samples?at=2026-09-23T10%3A00%3A12Z").await).await,
            json!({"error":"unavailable_time"})
        );
        assert_eq!(
            get(&app, "/front/samples").await.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            json_body(get(&app, "/front/samples?at=oops").await).await,
            json!({"error":"invalid_timestamp"})
        );
        assert_eq!(
            json_body(get(&app, "/front/files/not-an-id/content").await).await,
            json!({"error":"invalid_file_id"})
        );
        assert_eq!(
            get(&app, "/front/files/not-an-id/content").await.status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            json_body(get(&app, "/missing/samples?at=2026-09-23T10%3A00%3A00Z").await).await,
            json!({"error":"unknown_camera"})
        );
    }

    #[tokio::test]
    async fn internal_recording_errors_use_json() {
        use crate::schema::{video_files, video_units};

        let (_dir, service, app) = test_app();
        let file_id = add_file(
            &service,
            "front",
            "2026-09-23T10:00:00Z",
            "2026-09-23T10:00:10Z",
            "/unused.mkv",
            10,
        );
        let mut conn = service.pool.get().unwrap();
        let unit_id: i64 = video_files::table
            .filter(video_files::id.eq(file_id))
            .select(video_files::video_unit_id)
            .first(&mut conn)
            .unwrap();
        diesel::update(video_units::table.filter(video_units::id.eq(unit_id)))
            .set(video_units::end_time_us.eq(i64::MAX))
            .execute(&mut conn)
            .unwrap();
        drop(conn);

        let response = get(&app, "/front/samples?at=2026-09-23T10%3A00%3A05Z").await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(json_body(response).await, json!({"error":"internal_error"}));
    }

    #[tokio::test]
    async fn successor_uses_begin_time_then_id_even_across_gaps() {
        let (dir, service, app) = test_app();
        let name = dir.path().join("one.mkv").to_string_lossy().to_string();
        let first = add_file(
            &service,
            "front",
            "2026-09-23T10:00:00Z",
            "2026-09-23T10:00:05Z",
            &name,
            4,
        );
        let second = add_file(
            &service,
            "front",
            "2026-09-23T10:00:20Z",
            "2026-09-23T10:00:25Z",
            &name,
            4,
        );
        let third = add_file(
            &service,
            "front",
            "2026-09-23T10:00:20Z",
            "2026-09-23T10:00:26Z",
            &name,
            4,
        );
        let pending = add_file(
            &service,
            "front",
            "2026-09-23T10:00:30Z",
            "2026-09-23T10:00:31Z",
            &name,
            -1,
        );
        assert_eq!(
            json_body(get(&app, &format!("/front/files/{first}/next")).await).await["recording"]["fileId"],
            second
        );
        assert_eq!(
            json_body(get(&app, &format!("/front/files/{second}/next")).await).await["recording"]["fileId"],
            third
        );
        assert_eq!(
            json_body(get(&app, &format!("/front/files/{third}/next")).await).await,
            json!({"recording":null})
        );
        let mut conn = service.pool.get().unwrap();
        diesel::delete(
            crate::schema::video_files::table.filter(crate::schema::video_files::id.eq(second)),
        )
        .execute(&mut conn)
        .unwrap();
        drop(conn);
        assert_eq!(
            json_body(get(&app, &format!("/front/files/{second}/next")).await).await,
            json!({"error":"unavailable_file"})
        );
        assert_eq!(
            json_body(get(&app, &format!("/missing/files/{first}/next")).await).await,
            json!({"error":"unknown_camera"})
        );
        let mut conn = service.pool.get().unwrap();
        diesel::delete(
            crate::schema::video_files::table.filter(crate::schema::video_files::id.eq(third)),
        )
        .execute(&mut conn)
        .unwrap();
        diesel::delete(
            crate::schema::video_files::table.filter(crate::schema::video_files::id.eq(pending)),
        )
        .execute(&mut conn)
        .unwrap();
        drop(conn);
        let replacement = add_file(
            &service,
            "front",
            "2026-09-23T10:01:00Z",
            "2026-09-23T10:01:05Z",
            &name,
            4,
        );
        assert!(replacement > pending, "deleted IDs must never be reused");
    }

    #[tokio::test]
    async fn content_serves_only_full_gets_and_checks_camera_and_retention() {
        let (dir, service, app) = test_app();
        let path = dir.path().join("one.mkv");
        std::fs::write(&path, b"0123456789").unwrap();
        let file = add_file(
            &service,
            "front",
            "2026-09-23T10:00:00Z",
            "2026-09-23T10:00:10Z",
            path.to_str().unwrap(),
            10,
        );
        let uri = format!("/front/files/{file}/content");
        let response = get(&app, &uri).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "video/x-matroska");
        assert_eq!(response.headers()["content-length"], "10");
        assert!(!response.headers().contains_key("etag"));
        assert!(!response.headers().contains_key("accept-ranges"));
        assert_eq!(
            to_bytes(response.into_body(), 20).await.unwrap(),
            "0123456789"
        );

        // Range and validator headers do not change the full-download contract.
        for headers in [
            vec![("range", "bytes=2-5")],
            vec![("range", "bytes=0-1,8-9")],
            vec![("if-none-match", "*")],
            vec![("if-match", "\"stale\"")],
            vec![("if-modified-since", "Fri, 01 Jan 2100 00:00:00 GMT")],
        ] {
            let response = get_with_headers(&app, &uri, "GET", &headers).await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()["content-length"], "10");
            assert_eq!(
                to_bytes(response.into_body(), 20).await.unwrap(),
                "0123456789"
            );
        }
        let head = get_with_headers(&app, &uri, "HEAD", &[]).await;
        assert_eq!(head.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(head.headers()["allow"], "GET");

        assert_eq!(
            json_body(get(&app, &format!("/missing/files/{file}/content")).await).await,
            json!({"error":"unknown_camera"})
        );
        let mut conn = service.pool.get().unwrap();
        conn.batch_execute("INSERT INTO cameras (name, display_name, storage_group_name, ip, onvif_port, mac, username, password, rtsp_url, ptz_type, onvif_profile_token, enabled, ptz_x_step_size, ptz_y_step_size) VALUES ('rear', 'Rear', 'primary', '192.0.2.11', 80, '00:00:00:00:00:02', 'u', 'p', 'rtsp://rear.example/stream', 'none', NULL, 1, 1, 1)").unwrap();
        drop(conn);
        assert_eq!(
            json_body(get(&app, &format!("/rear/files/{file}/content")).await).await,
            json!({"error":"unavailable_file"})
        );
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            json_body(get(&app, &uri).await).await,
            json!({"error":"unavailable_file"})
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unreadable_existing_file_returns_json_server_error() {
        use std::os::unix::fs::PermissionsExt;

        let (dir, service, app) = test_app();
        let path = dir.path().join("unreadable.mkv");
        std::fs::write(&path, b"0123456789").unwrap();
        let file = add_file(
            &service,
            "front",
            "2026-09-23T10:00:00Z",
            "2026-09-23T10:00:10Z",
            path.to_str().unwrap(),
            10,
        );
        let original_permissions = std::fs::metadata(&path).unwrap().permissions();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        // Privileged test processes may still be able to open mode-000 files.
        if std::fs::File::open(&path).is_err() {
            let response = get(&app, &format!("/front/files/{file}/content")).await;
            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(json_body(response).await, json!({"error":"internal_error"}));
        }
        std::fs::set_permissions(path, original_permissions).unwrap();
    }

    #[tokio::test]
    async fn validates_requests_and_serializes_utc_ranges() {
        let (_temp_dir, service, app) = test_app();

        assert_eq!(get(&app, "/front").await.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            get(
                &app,
                "/front?begin_time=invalid&end_time=2026-09-22T10%3A15%3A00Z",
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            get(
                &app,
                "/front?begin_time=2026-09-22T10%3A00%3A00Z&end_time=2026-09-22T10%3A00%3A00Z",
            )
            .await
            .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            get(
                &app,
                "/missing?begin_time=2026-09-22T10%3A00%3A00Z&end_time=2026-09-22T10%3A15%3A00Z",
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );

        let response = get(
            &app,
            "/front?begin_time=2026-09-22T10%3A00%3A00Z&end_time=2026-09-22T10%3A15%3A00Z",
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(json_body(response).await, json!({ "ranges": [] }));

        let mut conn = service.pool.get().expect("insert connection");
        let begin_time_us = DateTime::parse_from_rfc3339("2026-09-22T10:00:00Z")
            .expect("timestamp parsed")
            .timestamp_micros();
        let end_time_us = DateTime::parse_from_rfc3339("2026-09-22T10:15:00Z")
            .expect("timestamp parsed")
            .timestamp_micros();
        diesel::insert_into(crate::schema::video_units::table)
            .values((
                crate::schema::video_units::camera_name.eq("front"),
                crate::schema::video_units::begin_time_us.eq(begin_time_us),
                crate::schema::video_units::end_time_us.eq(end_time_us),
            ))
            .execute(&mut conn)
            .expect("video unit inserted");
        drop(conn);

        let response = get(
            &app,
            "/front?begin_time=2026-09-22T12%3A00%3A00%2B02%3A00&end_time=2026-09-22T12%3A20%3A00%2B02%3A00",
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            json_body(response).await,
            json!({
                "ranges": [
                    {
                        "beginTime": "2026-09-22T10:00:00Z",
                        "endTime": "2026-09-22T10:15:00Z"
                    }
                ]
            })
        );
    }
}
