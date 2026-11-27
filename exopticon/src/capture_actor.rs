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

use std::{
    env,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use bytes::BytesMut;
use chrono::{DateTime, Utc};
use futures::stream::StreamExt;
use tokio::{
    process::{self, Child, ChildStdin, ChildStdout},
    sync::mpsc,
    task::spawn_blocking,
    time::{self, sleep_until, timeout_at},
};
use tokio_util::codec::{FramedRead, LengthDelimitedCodec, length_delimited};
use uuid::Uuid;

use crate::{
    CameraStatusRegistry,
    api::{cameras::CameraStatus, storage_groups::StorageGroup},
    db::cameras::Camera,
    metrics::{record_recording_publish_error, record_recording_published},
    onvif_actor,
    video_router::VideoRouter,
};
use exserial::models::{CaptureMessage, PacketEncoding};

const PACKET_STATUS_UPDATE_INTERVAL: Duration = Duration::from_secs(1);
const CAPTURE_WORKER_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub struct VideoPacket {
    pub camera_name: String,
    pub encoding: PacketEncoding,
    pub data: Vec<u8>,
    pub timestamp: i64,
    pub duration: i64,
}

pub enum Stream {
    StaticRtsp(String),
    Onvif(onvif_actor::Handle),
}

pub enum Command {
    Stop,
}

fn uuid_to_filename(parent_path: &Path, uuid: Uuid) -> PathBuf {
    let uuid_ts = uuid.get_timestamp().expect("failed to get uuid timestamp");
    let (secs, nsecs) = uuid_ts.to_unix();

    let secs = i64::try_from(secs).expect("overflow converting secs timestamp");

    let ts =
        chrono::DateTime::from_timestamp(secs, nsecs).expect("failed to build chrono datetime");

    let path = format!("{}/{}.mkv", ts.format("%Y/%m/%d/%H"), uuid);

    parent_path.join(&path)
}

fn publish_recording(
    db: &crate::db::Service,
    camera_name: &str,
    storage_root: &Path,
    filename: &str,
    begin_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> anyhow::Result<()> {
    use crate::api::video_units::{CreateVideoFile, CreateVideoUnit};

    let uuid = filename
        .strip_suffix(".mkv")
        .ok_or_else(|| anyhow::anyhow!("staged recording must have an .mkv extension"))?
        .parse::<Uuid>()?;

    let source = storage_root
        .join(".staging")
        .join(camera_name)
        .join(filename);
    let metadata = std::fs::symlink_metadata(&source)?;
    let destination = uuid_to_filename(&storage_root.join(camera_name), uuid);
    std::fs::create_dir_all(destination.parent().expect("archive path has a parent"))?;

    let (unit, file) = db.create_video_segment(
        &CreateVideoUnit {
            camera_name: camera_name.to_string(),
            begin_time,
            end_time,
        },
        CreateVideoFile {
            filename: destination.to_string_lossy().into_owned(),
            size: 0,
        },
    )?;

    // Same-filesystem publication, with no copy fallback.
    if let Err(err) = std::fs::rename(&source, &destination) {
        db.discard_unpublished_video_segment(unit.id)?;
        return Err(err.into());
    }
    let size = i32::try_from(metadata.len()).unwrap_or(i32::MAX);
    db.close_video_segment(unit.id, file.id, end_time, size)?;
    Ok(())
}

fn cleanup_staged_recordings(staging_path: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(staging_path).with_context(|| {
        format!(
            "failed to create staging directory {}",
            staging_path.display()
        )
    })?;
    let entries = std::fs::read_dir(staging_path).with_context(|| {
        format!(
            "failed to read staging directory {}",
            staging_path.display()
        )
    })?;
    let mut removed_count = 0;
    let mut removed_bytes = 0u64;

    for entry in entries {
        let entry = entry.with_context(|| {
            format!("failed to read staging entry in {}", staging_path.display())
        })?;
        let filename = entry.file_name();
        let Some(filename) = filename.to_str() else {
            continue;
        };
        let Some(uuid) = filename
            .strip_suffix(".mkv")
            .and_then(|stem| Uuid::parse_str(stem).ok())
        else {
            continue;
        };
        if uuid.get_version_num() != 7 || filename != format!("{uuid}.mkv") {
            continue;
        }

        let path = entry.path();
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(err) => {
                warn!(
                    "Failed to inspect staged recording {}: {}",
                    path.display(),
                    err
                );
                continue;
            }
        };
        if !metadata.is_file() {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => {
                removed_count += 1;
                removed_bytes += metadata.len();
            }
            Err(err) => {
                warn!(
                    "Failed to remove abandoned staged recording {}: {}",
                    path.display(),
                    err
                );
            }
        }
    }

    if removed_count > 0 {
        info!(
            "Removed {} abandoned staged recordings ({} bytes) from {}",
            removed_count,
            removed_bytes,
            staging_path.display()
        );
    }
    Ok(())
}

async fn wait_for_capture_worker(child: &mut Child, deadline: time::Instant) -> anyhow::Result<()> {
    if let Ok(result) = timeout_at(deadline, child.wait()).await {
        result.context("waiting for capture worker exit")?;
        return Ok(());
    }

    child
        .kill()
        .await
        .context("killing and reaping capture worker after shutdown timed out")?;
    anyhow::bail!("capture worker exceeded its shutdown deadline and was killed");
}

pub struct CaptureActor {
    db: crate::db::Service,
    camera: Camera,
    storage_group: StorageGroup,
    child: Option<(Child, FramedRead<ChildStdout, LengthDelimitedCodec>)>,
    // Keep stdin separate because Child::wait closes any stdin it
    // still owns.
    child_stdin: Option<ChildStdin>,

    /// Video Packet Router
    video_router: Arc<VideoRouter>,
    /// Supervisor command channel
    command_receiver: mpsc::Receiver<Command>,
    camera_status_registry: CameraStatusRegistry,
    unpublished_bytes: u64,
    video_codec: Option<String>,
    audio_codec: Option<String>,
    last_packet_status_update_at: Option<Instant>,
}

impl CaptureActor {
    pub const fn new(
        db: crate::db::Service,
        camera: Camera,
        storage_group: StorageGroup,
        command_receiver: mpsc::Receiver<Command>,
        video_router: Arc<VideoRouter>,
        camera_status_registry: CameraStatusRegistry,
    ) -> Self {
        Self {
            db,
            camera,
            storage_group,
            child: None,
            child_stdin: None,
            command_receiver,
            video_router,
            camera_status_registry,
            unpublished_bytes: 0,
            video_codec: None,
            audio_codec: None,
            last_packet_status_update_at: None,
        }
    }

    async fn start_worker(&mut self, stream_uri: &str) -> anyhow::Result<()> {
        debug!("Starting worker process for camera {}", self.camera.name);
        let storage_path = Path::new(&self.storage_group.spec.storage_path)
            .join(".staging")
            .join(&self.camera.name);
        let staging_path = storage_path.clone();
        spawn_blocking(move || cleanup_staged_recordings(&staging_path)).await??;
        let worker_path = env::var("EXOPTICONWORKERS").unwrap_or_else(|_| "/".to_string());
        let executable_path: PathBuf = [worker_path, "capture_worker".to_string()].iter().collect();

        let hwaccel_method =
            env::var("EXOPTICON_HWACCEL_METHOD").unwrap_or_else(|_| "none".to_string());
        let mut cmd = process::Command::new(executable_path);
        cmd.arg(stream_uri);
        cmd.arg(&storage_path);
        cmd.arg(hwaccel_method);
        cmd.stdout(Stdio::piped());
        cmd.stdin(Stdio::piped());
        cmd.kill_on_drop(true);
        // Keep terminal Ctrl-C from killing the worker before stdin EOF can
        // request a graceful shutdown and finalize the recording.
        cmd.process_group(0);

        let mut child = cmd.spawn()?;
        self.child_stdin = Some(
            child
                .stdin
                .take()
                .expect("Failed to open stdin on worker child"),
        );
        let stdout = child
            .stdout
            .take()
            .expect("Failed to open stdout on worker child");
        let framed_stream = length_delimited::Builder::new().new_read(stdout);

        self.child = Some((child, framed_stream));
        Ok(())
    }

    async fn handle_close_file(
        &self,
        filename: String,
        begin_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let db = self.db.clone();
        let camera_name = self.camera.name.clone();
        let storage_root = PathBuf::from(&self.storage_group.spec.storage_path);
        spawn_blocking(move || {
            publish_recording(
                &db,
                &camera_name,
                &storage_root,
                &filename,
                begin_time,
                end_time,
            )
        })
        .await?
    }

    async fn cleanup_unopened_video_segments(&self) -> anyhow::Result<()> {
        let db = self.db.clone();
        let camera_name = self.camera.name.clone();
        let deleted_count =
            spawn_blocking(move || db.delete_unopened_video_segments(&camera_name)).await??;

        if deleted_count > 0 {
            debug!(
                "Deleted {} unopened video segments for {}",
                deleted_count, self.camera.name
            );
        }

        Ok(())
    }

    async fn update_packet_status(&mut self, encoding: &PacketEncoding) {
        let codec = encoding.codec_name();
        let codec_changed = match encoding {
            PacketEncoding::H264 => self.video_codec.as_deref() != Some(codec),
            PacketEncoding::PCMU => self.audio_codec.as_deref() != Some(codec),
        };

        match encoding {
            PacketEncoding::H264 => self.video_codec = Some(codec.to_string()),
            PacketEncoding::PCMU => self.audio_codec = Some(codec.to_string()),
        }

        let now = Instant::now();
        let should_update = codec_changed
            || self
                .last_packet_status_update_at
                .is_none_or(|last_update| last_update.elapsed() >= PACKET_STATUS_UPDATE_INTERVAL);

        if !should_update {
            return;
        }

        let mut statuses = self.camera_status_registry.write().await;
        let (last_started_at, average_bitrate) = statuses
            .get(&self.camera.name)
            .map_or((None, None), |status| {
                (status.last_started_at, status.average_bitrate)
            });

        statuses.insert(
            self.camera.name.clone(),
            CameraStatus {
                phase: "running".to_string(),
                last_started_at,
                video_codec: self.video_codec.clone(),
                audio_codec: self.audio_codec.clone(),
                average_bitrate,
                error_message: None,
                unpublished_bytes: self.unpublished_bytes,
            },
        );
        drop(statuses);
        self.last_packet_status_update_at = Some(now);
    }

    async fn handle_packet(
        &mut self,
        encoding: PacketEncoding,
        data: Vec<u8>,
        timestamp: i64,
        duration: i64,
    ) {
        let packet_bytes = u64::try_from(data.len()).unwrap_or(u64::MAX);
        self.unpublished_bytes = self.unpublished_bytes.saturating_add(packet_bytes);
        let packet = VideoPacket {
            camera_name: self.camera.name.clone(),
            encoding,
            data,
            timestamp,
            duration,
        };
        self.update_packet_status(&packet.encoding).await;
        self.video_router.send_video(packet).await;
    }

    async fn message_to_action(&mut self, msg: CaptureMessage) -> anyhow::Result<()> {
        match msg {
            CaptureMessage::Log { level, message } => {
                log!(
                    level,
                    "capture worker {} log: {}",
                    self.camera.name,
                    message
                );
            }
            CaptureMessage::Packet {
                encoding,
                data,
                timestamp,
                duration,
            } => {
                self.handle_packet(encoding, data, timestamp, duration)
                    .await;
            }
            CaptureMessage::EndFile {
                filename,
                begin_time,
                end_time,
            } => {
                let close_result = async {
                    let begin = begin_time.parse()?;
                    let end = end_time.parse()?;
                    self.handle_close_file(filename, begin, end).await
                }
                .await;
                match close_result {
                    Ok(()) => {
                        self.unpublished_bytes = 0;
                        record_recording_published(&self.camera.name);
                        if let Some(status) = self
                            .camera_status_registry
                            .write()
                            .await
                            .get_mut(&self.camera.name)
                        {
                            status.unpublished_bytes = 0;
                        }
                    }
                    Err(err) => {
                        record_recording_publish_error(&self.camera.name);
                        return Err(err);
                    }
                }
            }
        }
        Ok(())
    }

    async fn stream_handler(
        &mut self,
        msg: Result<BytesMut, std::io::Error>,
    ) -> anyhow::Result<()> {
        let item = msg?;

        let frame: CaptureMessage = exserial::deserialize_message(&item[..])?;

        self.message_to_action(frame).await?;

        Ok(())
    }

    async fn select_next(&mut self) -> anyhow::Result<bool> {
        match &mut self.child {
            Some((child, framed_stream)) => {
                tokio::select! {
                    biased;
                    Some(Command::Stop) = self.command_receiver.recv() => {
                        info!("Received stop command for {}.", self.camera.name);
                        return Ok(false)
                    }
                    msg = framed_stream.next() => match msg {
                        Some(msg) => self.stream_handler(msg).await?,
                        None => anyhow::bail!("Capture worker closed its output"),
                    },
                    _ = child.wait() => {
                        anyhow::bail!("Capture worker exited");
                    }
                    else => return Ok(false)
                }
            }
            _ => {
                tokio::select! {
                    Some(Command::Stop) = self.command_receiver.recv() => return Ok(false),
                    else => return Ok(false),
                }
            }
        }

        Ok(true)
    }

    async fn start(&mut self, stream: Stream) -> Result<bool, &'static str> {
        if let Err(e) = self.cleanup_unopened_video_segments().await {
            error!("error cleaning unopened video segments: {}", e);
        }
        let stream_uri = match stream {
            Stream::StaticRtsp(url) => url,
            Stream::Onvif(onvif) => tokio::select! {
                biased;
                _ = self.command_receiver.recv() => return Ok(false),
                result = onvif.stream_uri() => {
                    result.map_err(|_| "Could not resolve the ONVIF stream URI")?.into()
                }
            },
        };
        if let Err(error) = self.start_worker(&stream_uri).await {
            error!("error starting capture worker: {}", error);
            return Err("Could not start the capture worker");
        }
        Ok(true)
    }

    pub async fn run(mut self, stream: Stream) -> String {
        let mut error_message = match self.start(stream).await {
            Ok(true) => loop {
                match self.select_next().await {
                    Ok(true) => {}
                    Ok(false) => break None,
                    Err(e) => {
                        error!("Error {}", e);
                        break Some("capture actor exited after error");
                    }
                }
            },
            Ok(false) => None,
            Err(message) => {
                error!("{} for {}", message, self.camera.name);
                Some(message)
            }
        };

        // EOF signals a shutdown. Wait for final messages to arrive
        // until the shutdown deadline.
        let deadline = time::Instant::now() + CAPTURE_WORKER_SHUTDOWN_TIMEOUT;
        drop(self.child_stdin.take());

        if let Some((mut child, mut framed_stream)) = self.child.take() {
            let pid = child.id();
            let drain = async {
                let deadline = sleep_until(deadline);
                tokio::pin!(deadline);
                loop {
                    let message = tokio::select! {
                        biased;
                        () = &mut deadline => break,
                        message = framed_stream.next() => message,
                    };
                    let Some(message) = message else { break };
                    // Allow publication to finish, even if the worker
                    // is killed.
                    if let Err(e) = self.stream_handler(message).await {
                        error!(
                            "Error handling capture worker message during shutdown: {}",
                            e
                        );
                        error_message = Some("capture actor exited after error");
                    }
                }
            };
            let ((), exit_result) =
                tokio::join!(drain, wait_for_capture_worker(&mut child, deadline));
            if let Err(error) = exit_result {
                error!(
                    "Capture worker shutdown failed for {} (PID {pid:?}): {error:#}",
                    self.camera.name,
                );
                error_message = Some("capture worker shutdown failed");
            }
        }

        if let Err(e) = self.cleanup_unopened_video_segments().await {
            error!("error cleaning unopened video segments after stop: {}", e);
        }
        self.camera_status_registry.write().await.insert(
            self.camera.name.clone(),
            CameraStatus {
                phase: if error_message.is_some() {
                    "error"
                } else {
                    "stopped"
                }
                .to_string(),
                last_started_at: None,
                video_codec: self.video_codec.clone(),
                audio_codec: self.audio_codec.clone(),
                average_bitrate: None,
                error_message: error_message.map(str::to_owned),
                unpublished_bytes: 0,
            },
        );
        self.camera.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn worker_past_shutdown_deadline_is_killed_and_reaped() {
        use std::os::unix::process::ExitStatusExt;

        let mut child = process::Command::new("sleep")
            .arg("60")
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let deadline = time::Instant::now() + Duration::from_millis(25);
        let error = time::timeout(
            Duration::from_secs(5),
            wait_for_capture_worker(&mut child, deadline),
        )
        .await
        .expect("shutdown deadline did not stop the worker")
        .unwrap_err();

        assert!(error.to_string().contains("exceeded its shutdown deadline"));
        assert_eq!(child.try_wait().unwrap().unwrap().signal(), Some(9));
        assert_eq!(child.id(), None);
    }

    #[test]
    fn staging_cleanup_removes_only_this_cameras_abandoned_recordings() {
        let directory = tempfile::tempdir().unwrap();
        let staging = directory.path().join(".staging/front");
        std::fs::create_dir_all(&staging).unwrap();
        let filename = "019a2c8e-7200-7000-8000-000000000001.mkv";
        let abandoned = staging.join(filename);
        std::fs::write(&abandoned, b"interrupted recording").unwrap();

        let preserved = [
            staging.join("notes.mkv"),
            staging.join("019A2C8E-7200-7000-8000-000000000001.mkv"),
            staging.join("019a2c8e-7200-4000-8000-000000000001.mkv"),
            staging.join("019a2c8e-7200-7000-8000-000000000001.tmp"),
            staging.join("nested").join(filename),
            staging.join("019a2c8e-7200-7000-8000-000000000002.mkv/keep"),
            directory.path().join(".staging/back").join(filename),
            directory.path().join("front/2026/11/03/21").join(filename),
        ];
        for path in &preserved {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"keep").unwrap();
        }

        cleanup_staged_recordings(&staging).unwrap();
        cleanup_staged_recordings(&staging).unwrap();

        assert!(!abandoned.exists());
        for path in &preserved {
            assert_eq!(std::fs::read(path).unwrap(), b"keep", "{}", path.display());
        }
    }

    #[cfg(unix)]
    #[test]
    fn staging_cleanup_preserves_symlinks_and_their_targets() {
        let directory = tempfile::tempdir().unwrap();
        let staging = directory.path().join(".staging/front");
        std::fs::create_dir_all(&staging).unwrap();
        let target = directory.path().join("recording.mkv");
        std::fs::write(&target, b"keep").unwrap();
        let link = staging.join("019a2c8e-7200-7000-8000-000000000001.mkv");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        cleanup_staged_recordings(&staging).unwrap();

        assert!(std::fs::symlink_metadata(link).unwrap().is_symlink());
        assert_eq!(std::fs::read(target).unwrap(), b"keep");
    }

    #[test]
    fn unpublished_bytes_reset_on_publish_and_errors_increment_counter() {
        use std::collections::HashMap;
        use tokio::sync::RwLock;

        let (dir, db) = crate::db::tests::configured_service();
        let camera = db.fetch_camera_row("front").unwrap();
        let storage_group = crate::api::storage_groups::StorageGroup {
            metadata: crate::api::ResourceMetadata {
                name: "primary".to_string(),
                display_name: "Primary".to_string(),
            },
            spec: crate::api::storage_groups::StorageGroupSpec {
                storage_path: dir.path().display().to_string(),
                max_storage_size: 1024,
            },
            status: serde_json::json!({}),
        };
        let staging = dir.path().join(".staging/front");
        std::fs::create_dir_all(&staging).unwrap();
        let filename = "019a2c8e-7200-7000-8000-000000000001.mkv";
        std::fs::write(staging.join(filename), b"mkvdata").unwrap();

        let statuses: CameraStatusRegistry = Arc::new(RwLock::new(HashMap::new()));
        let (_tx, rx) = mpsc::channel(1);
        let mut actor = CaptureActor::new(
            db,
            camera,
            storage_group,
            rx,
            Arc::new(VideoRouter::new()),
            Arc::clone(&statuses),
        );

        let recorder = crate::metrics::build_recorder();
        let handle = recorder.handle();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        metrics::with_local_recorder(&recorder, || {
            runtime.block_on(async {
                actor
                    .message_to_action(CaptureMessage::Packet {
                        encoding: PacketEncoding::H264,
                        data: vec![1; 2048],
                        timestamp: 90_000,
                        duration: 33_333,
                    })
                    .await
                    .unwrap();
                assert_eq!(statuses.read().await["front"].unpublished_bytes, 2048);

                actor
                    .message_to_action(CaptureMessage::EndFile {
                        filename: filename.to_string(),
                        begin_time: "2026-11-03T21:00:00Z".to_string(),
                        end_time: "2026-11-03T21:00:10Z".to_string(),
                    })
                    .await
                    .unwrap();
                assert_eq!(statuses.read().await["front"].unpublished_bytes, 0);

                // Missing staged file fails publication and increments error counter.
                let err = actor
                    .message_to_action(CaptureMessage::EndFile {
                        filename: "019a2c8e-7200-7000-8000-000000000099.mkv".to_string(),
                        begin_time: "2026-11-03T21:00:10Z".to_string(),
                        end_time: "2026-11-03T21:00:20Z".to_string(),
                    })
                    .await;
                assert!(err.is_err());
            });
        });

        let output = handle.render();
        assert!(output.contains(r#"exopticon_recordings_published_total{camera_name="front"} 1"#));
        assert!(
            output.contains(r#"exopticon_recording_publish_errors_total{camera_name="front"} 1"#)
        );
    }
}
