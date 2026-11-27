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

use std::{
    collections::{BTreeMap, HashMap},
    fmt::Write as _,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use axum_prometheus::{
    EndpointLabel, PrometheusMetricLayer, PrometheusMetricLayerBuilder,
    metrics_exporter_prometheus::{
        Matcher, PrometheusBuilder, PrometheusHandle, PrometheusRecorder,
    },
};
use metrics::{Gauge, Unit, counter, describe_counter, describe_gauge, gauge};
use tokio::sync::RwLock;

use crate::{CameraStatusRegistry, api::cameras::CameraStatus};

pub const HTTP_PREFIX: &str = "exopticon";
pub const HTTP_REQUESTS_DURATION_SECONDS: &str = "exopticon_http_requests_duration_seconds";
pub const WEBRTC_SESSIONS: &str = "exopticon_webrtc_sessions";

pub const CAMERA_UP: &str = "exopticon_camera_up";
pub const CAPTURE_RESTARTS_TOTAL: &str = "exopticon_capture_restarts_total";
pub const RECORDINGS_PUBLISHED_TOTAL: &str = "exopticon_recordings_published_total";
pub const RECORDING_PUBLISH_ERRORS_TOTAL: &str = "exopticon_recording_publish_errors_total";
pub const RECORDING_UNPUBLISHED_BYTES: &str = "exopticon_recording_unpublished_bytes";

pub const STORAGE_GROUP_USED_BYTES: &str = "exopticon_storage_group_used_bytes";
pub const STORAGE_GROUP_MAX_BYTES: &str = "exopticon_storage_group_max_bytes";
pub const RETENTION_ERRORS_TOTAL: &str = "exopticon_retention_errors_total";
pub const RETENTION_LAST_RUN_TIMESTAMP_SECONDS: &str =
    "exopticon_retention_last_run_timestamp_seconds";

const HTTP_DURATION_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
];
const UPKEEP_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageGroupUsage {
    pub used_bytes: u64,
    pub max_bytes: u64,
}

pub type StorageGroupUsageRegistry = Arc<RwLock<BTreeMap<String, StorageGroupUsage>>>;

#[must_use]
pub fn new_storage_group_usage_registry() -> StorageGroupUsageRegistry {
    Arc::new(RwLock::new(BTreeMap::new()))
}

#[derive(Clone)]
pub struct MetricsHandle {
    prometheus: PrometheusHandle,
    cameras: CameraStatusRegistry,
    storage_groups: StorageGroupUsageRegistry,
}

impl MetricsHandle {
    pub async fn render(&self) -> String {
        let mut output = self.prometheus.render();
        {
            let cameras = self.cameras.read().await;
            render_camera_gauges(&cameras, &mut output);
        }
        {
            let storage_groups = self.storage_groups.read().await;
            render_storage_group_gauges(&storage_groups, &mut output);
        }
        output
    }
}

fn write_metric_header(output: &mut String, name: &str, kind: &str, help: &str) {
    let _ = writeln!(output, "# HELP {name} {help}");
    let _ = writeln!(output, "# TYPE {name} {kind}");
}

fn escape_label_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn render_camera_gauges(cameras: &HashMap<String, CameraStatus>, output: &mut String) {
    let mut enabled: Vec<(&str, &CameraStatus)> = cameras
        .iter()
        .filter(|(_, status)| status.phase != "disabled")
        .map(|(name, status)| (name.as_str(), status))
        .collect();
    if enabled.is_empty() {
        return;
    }
    enabled.sort_unstable_by_key(|(name, _)| *name);

    write_metric_header(
        output,
        CAMERA_UP,
        "gauge",
        "Whether each enabled camera is actively streaming (1 when running, 0 otherwise).",
    );
    for &(name, status) in &enabled {
        let escaped = escape_label_value(name);
        let up = u8::from(status.phase == "running");
        let _ = writeln!(output, "{CAMERA_UP}{{camera_name=\"{escaped}\"}} {up}");
    }
    output.push('\n');

    write_metric_header(
        output,
        RECORDING_UNPUBLISHED_BYTES,
        "gauge",
        "Stream payload bytes received since the camera last published a closed recording segment.",
    );
    for &(name, status) in &enabled {
        let escaped = escape_label_value(name);
        let _ = writeln!(
            output,
            "{RECORDING_UNPUBLISHED_BYTES}{{camera_name=\"{escaped}\"}} {}",
            status.unpublished_bytes
        );
    }
    output.push('\n');
}

fn render_storage_group_gauges(
    storage_groups: &BTreeMap<String, StorageGroupUsage>,
    output: &mut String,
) {
    if storage_groups.is_empty() {
        return;
    }

    write_metric_header(
        output,
        STORAGE_GROUP_USED_BYTES,
        "gauge",
        "Finalized recording bytes currently stored in each storage group.",
    );
    for (name, usage) in storage_groups {
        let escaped = escape_label_value(name);
        let _ = writeln!(
            output,
            "{STORAGE_GROUP_USED_BYTES}{{storage_group=\"{escaped}\"}} {}",
            usage.used_bytes
        );
    }
    output.push('\n');

    write_metric_header(
        output,
        STORAGE_GROUP_MAX_BYTES,
        "gauge",
        "Configured maximum capacity in bytes for each storage group.",
    );
    for (name, usage) in storage_groups {
        let escaped = escape_label_value(name);
        let _ = writeln!(
            output,
            "{STORAGE_GROUP_MAX_BYTES}{{storage_group=\"{escaped}\"}} {}",
            usage.max_bytes
        );
    }
    output.push('\n');
}

pub fn init_camera_counters(camera_name: &str) {
    counter!(CAPTURE_RESTARTS_TOTAL, "camera_name" => camera_name.to_owned()).increment(0);
    counter!(RECORDINGS_PUBLISHED_TOTAL, "camera_name" => camera_name.to_owned()).increment(0);
    counter!(RECORDING_PUBLISH_ERRORS_TOTAL, "camera_name" => camera_name.to_owned()).increment(0);
}

pub fn record_capture_restart(camera_name: &str) {
    counter!(CAPTURE_RESTARTS_TOTAL, "camera_name" => camera_name.to_owned()).increment(1);
}

pub fn record_recording_published(camera_name: &str) {
    counter!(RECORDINGS_PUBLISHED_TOTAL, "camera_name" => camera_name.to_owned()).increment(1);
}

pub fn record_recording_publish_error(camera_name: &str) {
    counter!(RECORDING_PUBLISH_ERRORS_TOTAL, "camera_name" => camera_name.to_owned()).increment(1);
}

pub fn record_retention_error() {
    counter!(RETENTION_ERRORS_TOTAL).increment(1);
}

pub fn record_retention_run_at(now: SystemTime) {
    let timestamp_secs = now
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    gauge!(RETENTION_LAST_RUN_TIMESTAMP_SECONDS).set(timestamp_secs);
}

pub fn build_recorder() -> PrometheusRecorder {
    PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Full(HTTP_REQUESTS_DURATION_SECONDS.to_string()),
            HTTP_DURATION_BUCKETS,
        )
        .expect("valid HTTP duration buckets")
        .build_recorder()
}

pub fn describe_custom_metrics() {
    describe_gauge!(
        WEBRTC_SESSIONS,
        Unit::Count,
        "Current number of active WebRTC viewer sessions."
    );
    describe_counter!(
        CAPTURE_RESTARTS_TOTAL,
        Unit::Count,
        "Total number of unplanned camera capture restarts."
    );
    describe_counter!(
        RECORDINGS_PUBLISHED_TOTAL,
        Unit::Count,
        "Total number of closed MKV recording segments published to archive storage."
    );
    describe_counter!(
        RECORDING_PUBLISH_ERRORS_TOTAL,
        Unit::Count,
        "Total number of errors while publishing closed MKV recording segments."
    );
    describe_counter!(
        RETENTION_ERRORS_TOTAL,
        Unit::Count,
        "Total number of storage retention and deleted-camera purge errors."
    );
    describe_gauge!(
        RETENTION_LAST_RUN_TIMESTAMP_SECONDS,
        Unit::Seconds,
        "Unix timestamp in seconds of the last completed storage retention pass."
    );
}

pub fn init(
    cameras: CameraStatusRegistry,
    storage_groups: StorageGroupUsageRegistry,
) -> (PrometheusMetricLayer<'static>, MetricsHandle) {
    let recorder = build_recorder();
    let prometheus = recorder.handle();
    metrics::set_global_recorder(recorder).expect("failed to install Prometheus metrics recorder");

    let upkeep_handle = prometheus.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(UPKEEP_INTERVAL).await;
            upkeep_handle.run_upkeep();
        }
    });

    // Build the HTTP layer after installing the global recorder so its
    // `describe_*!` calls register `# HELP` metadata on the active recorder.
    let layer = PrometheusMetricLayerBuilder::new()
        .with_prefix(HTTP_PREFIX)
        .with_ignore_pattern("/metrics")
        .with_endpoint_label_type(EndpointLabel::MatchedPathWithFallbackFn(|_| {
            "<unmatched>".to_string()
        }))
        .build();

    describe_custom_metrics();
    counter!(RETENTION_ERRORS_TOTAL).increment(0);

    let handle = MetricsHandle {
        prometheus,
        cameras,
        storage_groups,
    };
    (layer, handle)
}

pub struct WebRtcSessionGuard {
    gauge: Gauge,
}

impl WebRtcSessionGuard {
    #[must_use]
    pub fn new() -> Self {
        let gauge = gauge!(WEBRTC_SESSIONS);
        gauge.increment(1);
        Self { gauge }
    }
}

impl Drop for WebRtcSessionGuard {
    fn drop(&mut self) {
        self.gauge.decrement(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_metrics_and_dynamic_gauges_render_expected_series() {
        let recorder = build_recorder();
        let cameras: CameraStatusRegistry = Arc::new(RwLock::new(HashMap::from([
            (
                "front".to_string(),
                CameraStatus {
                    phase: "running".to_string(),
                    last_started_at: None,
                    video_codec: Some("h264".to_string()),
                    audio_codec: None,
                    average_bitrate: None,
                    error_message: None,
                    unpublished_bytes: 4_096,
                },
            ),
            (
                "porch".to_string(),
                CameraStatus {
                    phase: "error".to_string(),
                    last_started_at: None,
                    video_codec: None,
                    audio_codec: None,
                    average_bitrate: None,
                    error_message: Some("Could not start the capture worker".to_string()),
                    unpublished_bytes: 0,
                },
            ),
            ("disabled-cam".to_string(), CameraStatus::disabled()),
        ])));
        let storage_groups: StorageGroupUsageRegistry = Arc::new(RwLock::new(BTreeMap::from([(
            "primary".to_string(),
            StorageGroupUsage {
                used_bytes: 524_288_000,
                max_bytes: 1_073_741_824,
            },
        )])));
        let handle = MetricsHandle {
            prometheus: recorder.handle(),
            cameras: Arc::clone(&cameras),
            storage_groups: Arc::clone(&storage_groups),
        };

        metrics::with_local_recorder(&recorder, || {
            describe_custom_metrics();
            init_camera_counters("front");
            record_capture_restart("front");
            record_recording_published("front");
            record_recording_publish_error("porch");
            record_retention_error();
            record_retention_run_at(UNIX_EPOCH + Duration::from_secs(1_700_000_000));
            let _session = WebRtcSessionGuard::new();
        });

        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let output = runtime.block_on(handle.render());

        assert!(output.contains("# TYPE exopticon_camera_up gauge"));
        assert!(output.contains(r#"exopticon_camera_up{camera_name="front"} 1"#));
        assert!(output.contains(r#"exopticon_camera_up{camera_name="porch"} 0"#));
        assert!(!output.contains(r#"camera_name="disabled-cam""#));

        assert!(output.contains("# TYPE exopticon_recording_unpublished_bytes gauge"));
        assert!(
            output.contains(r#"exopticon_recording_unpublished_bytes{camera_name="front"} 4096"#)
        );
        assert!(output.contains(r#"exopticon_recording_unpublished_bytes{camera_name="porch"} 0"#));

        assert!(output.contains(r#"exopticon_capture_restarts_total{camera_name="front"} 1"#));
        assert!(output.contains(r#"exopticon_recordings_published_total{camera_name="front"} 1"#));
        assert!(
            output.contains(r#"exopticon_recording_publish_errors_total{camera_name="front"} 0"#)
        );
        assert!(
            output.contains(r#"exopticon_recording_publish_errors_total{camera_name="porch"} 1"#)
        );

        assert!(
            output.contains(
                r#"exopticon_storage_group_used_bytes{storage_group="primary"} 524288000"#
            )
        );
        assert!(
            output.contains(
                r#"exopticon_storage_group_max_bytes{storage_group="primary"} 1073741824"#
            )
        );
        assert!(output.contains("exopticon_retention_errors_total 1"));
        assert!(output.contains("exopticon_retention_last_run_timestamp_seconds 1700000000"));
        assert!(output.contains("exopticon_webrtc_sessions 0"));

        // Removing a camera or storage group from the live registries drops its gauges on scrape.
        runtime.block_on(async {
            cameras.write().await.remove("porch");
            storage_groups.write().await.clear();
        });
        let updated = runtime.block_on(handle.render());
        assert!(!updated.contains(r#"exopticon_camera_up{camera_name="porch"}"#));
        assert!(!updated.contains(r#"exopticon_recording_unpublished_bytes{camera_name="porch"}"#));
        assert!(!updated.contains(STORAGE_GROUP_USED_BYTES));
        assert!(!updated.contains(STORAGE_GROUP_MAX_BYTES));
    }
}
