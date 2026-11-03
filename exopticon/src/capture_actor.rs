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
use metrics::{Counter, counter};
use regex::Regex;
use tokio::{
    process::{self, Child, ChildStdout},
    sync::mpsc,
    task::{JoinSet, spawn_blocking},
};
use tokio_util::codec::{FramedRead, LengthDelimitedCodec, length_delimited};
use uuid::Uuid;

use crate::{
    CameraStatusRegistry,
    api::{cameras::CameraStatus, storage_groups::StorageGroup},
    db::cameras::Camera,
    video_router::VideoRouter,
};
use exserial::models::{CaptureMessage, PacketEncoding};

const PACKET_STATUS_UPDATE_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone)]
pub struct VideoPacket {
    pub camera_name: String,
    pub encoding: PacketEncoding,
    pub data: Vec<u8>,
    pub timestamp: i64,
    pub duration: i64,
}

pub enum Command {
    Stop,
}

#[derive(Clone, PartialEq, Eq)]
enum State {
    Ready,
    Started,
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

pub struct CaptureActor {
    state: State,
    db: crate::db::Service,
    camera: Camera,
    storage_group: StorageGroup,
    child: Option<(Child, FramedRead<ChildStdout, LengthDelimitedCodec>)>,

    /// Video Packet Router
    video_router: Arc<VideoRouter>,
    /// Supervisor command channel
    command_receiver: mpsc::Receiver<Command>,
    camera_status_registry: CameraStatusRegistry,
    /// counter to track lost packets
    lost_packet_counter: Counter,
    video_codec: Option<String>,
    audio_codec: Option<String>,
    last_packet_status_update_at: Option<Instant>,
}

impl CaptureActor {
    pub fn new(
        db: crate::db::Service,
        camera: Camera,
        storage_group: StorageGroup,
        command_receiver: mpsc::Receiver<Command>,
        video_router: Arc<VideoRouter>,
        camera_status_registry: CameraStatusRegistry,
    ) -> Self {
        let camera_name = camera.name.clone();
        Self {
            state: State::Ready,
            db,
            camera,
            storage_group,
            child: None,
            command_receiver,
            video_router,
            camera_status_registry,
            lost_packet_counter: counter!("lost_packet_count", "camera_name" => camera_name),
            video_codec: None,
            audio_codec: None,
            last_packet_status_update_at: None,
        }
    }

    async fn start_worker(&mut self) -> anyhow::Result<()> {
        debug!(
            "Starting worker process for camera: {}, id: {}, stream: {}",
            self.camera.name, self.camera.name, self.camera.rtsp_url
        );
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
        cmd.arg(&self.camera.rtsp_url);
        cmd.arg(&storage_path);
        cmd.arg(hwaccel_method);
        cmd.stdout(Stdio::piped());
        cmd.stdin(Stdio::null());
        cmd.kill_on_drop(true);

        let mut child = cmd.spawn()?;
        let stdout = child
            .stdout
            .take()
            .expect("Failed to open stdout on worker child");
        let framed_stream = length_delimited::Builder::new().new_read(stdout);

        self.child = Some((child, framed_stream));
        self.state = State::Started;
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
                active: true,
                last_started_at,
                video_codec: self.video_codec.clone(),
                audio_codec: self.audio_codec.clone(),
                average_bitrate,
                error_message: None,
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

    fn check_log_for_lost_packets(log: &str) -> Option<u32> {
        static RE: std::sync::LazyLock<Regex> =
            std::sync::LazyLock::new(|| Regex::new(r"RTP: missed ([0-9]+) packets").unwrap());
        let caps = RE.captures(log)?;

        caps[1]
            .trim()
            .parse::<u32>()
            .as_ref()
            .map_or(None, |num| Some(*num))
    }
    async fn message_to_action(&mut self, msg: CaptureMessage) -> anyhow::Result<()> {
        match msg {
            CaptureMessage::Log { level, message } => {
                log!(
                    level,
                    "capture worker {} {} log: {}",
                    self.camera.name,
                    self.camera.name,
                    message
                );

                if let Some(packet_count) = Self::check_log_for_lost_packets(&message) {
                    self.lost_packet_counter.increment(packet_count.into());
                }
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
                self.handle_close_file(filename, begin_time.parse()?, end_time.parse()?)
                    .await?;
            }
            CaptureMessage::Metric {
                label: _,
                values: _,
            } => {
                debug!("got capture metrics");
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
                    Some(msg) = framed_stream.next() => self.stream_handler(msg).await?,
                    _ = child.wait() => {
                        info!(
                            "Capture process for {} {} died. Restarting...",
                            self.camera.name, self.camera.name,
                        );
                        return Ok(false);
                    }
                    Some(Command::Stop) = self.command_receiver.recv() => {
                        info!("Received stop command for {} {}.",
                              self.camera.name, self.camera.name,
                        );
                        return Ok(false)
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

    pub async fn run(mut self) -> String {
        // JoinSet aborts the ONVIF task if this actor is dropped or panics.
        let mut keyframe_tasks = JoinSet::new();
        if self.camera.onvif_profile_token.is_some() {
            let requests = Arc::new(crate::keyframe_requests::KeyframeRequestGate::new());
            self.video_router
                .register_keyframe_requests(self.camera.name.clone(), Arc::clone(&requests))
                .await;
            keyframe_tasks.spawn(crate::keyframe_requests::run(self.camera.clone(), requests));
        }
        let mut had_error = false;
        loop {
            if self.state == State::Ready {
                if let Err(e) = self.cleanup_unopened_video_segments().await {
                    error!("error cleaning unopened video segments: {}", e);
                }
                if let Err(e) = self.start_worker().await {
                    error!("error starting capture worker: {}", e);
                    had_error = true;
                    break;
                }
            }
            let res = self.select_next().await;
            match res {
                Ok(true) => {}
                Ok(false) => break,
                Err(e) => {
                    error!("Error {}", e);
                    had_error = true;
                    break;
                }
            }
        }

        self.video_router
            .unregister_keyframe_requests(&self.camera.name)
            .await;
        keyframe_tasks.shutdown().await;

        if let Some((mut child, _)) = self.child.take() {
            // wait for child to exit...
            if let Err(e) = child.kill().await {
                error!("error killing child: {}", e);
            }
            if let Err(e) = child.wait().await {
                error!("error waiting for child exit: {}", e);
            }
        }
        if let Err(e) = self.cleanup_unopened_video_segments().await {
            error!("error cleaning unopened video segments after stop: {}", e);
        }
        self.camera_status_registry.write().await.insert(
            self.camera.name.clone(),
            if had_error {
                CameraStatus {
                    phase: "error".to_string(),
                    active: false,
                    last_started_at: None,
                    video_codec: self.video_codec.clone(),
                    audio_codec: self.audio_codec.clone(),
                    average_bitrate: None,
                    error_message: Some("capture actor exited after error".to_string()),
                }
            } else {
                CameraStatus {
                    phase: "stopped".to_string(),
                    active: false,
                    last_started_at: None,
                    video_codec: self.video_codec.clone(),
                    audio_codec: self.audio_codec.clone(),
                    average_bitrate: None,
                    error_message: None,
                }
            },
        );
        self.camera.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
