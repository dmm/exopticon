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
    extract::{FromRef, Path, Query, State},
    routing::get,
};
use chrono::{DateTime, Utc};
use tokio::task::spawn_blocking;

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
    Router::<AppState>::new().route("/{camera_name}", get(fetch_recording_ranges))
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
    use diesel::{ExpressionMethods, RunQueryDsl, connection::SimpleConnection};
    use diesel_migrations::MigrationHarness;
    use serde_json::{Value, json};
    use tempfile::TempDir;
    use tower::ServiceExt;

    use super::fetch_recording_ranges;
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

    async fn json_body(response: Response) -> Value {
        let body = to_bytes(response.into_body(), 1_024)
            .await
            .expect("response body read");
        serde_json::from_slice(&body).expect("JSON response parsed")
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
