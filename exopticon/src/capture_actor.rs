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

use bytes::BytesMut;
use chrono::{DateTime, Utc};
use futures::stream::StreamExt;
use metrics::{Counter, counter};
use regex::Regex;
use tokio::{
    fs,
    io::AsyncWriteExt,
    process::{self, Child, ChildStdin, ChildStdout},
    sync::mpsc,
    task::spawn_blocking,
};
use tokio_util::codec::{FramedRead, LengthDelimitedCodec, length_delimited};
use uuid::Uuid;

use crate::{
    CameraStatusRegistry,
    api::{cameras::CameraStatus, storage_groups::StorageGroup},
    db::cameras::Camera,
    video_router::VideoRouter,
};
use exserial::models::{CaptureCommand, CaptureMessage, PacketEncoding};

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
    Recording,
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

pub struct CaptureActor {
    state: State,
    db: crate::db::Service,
    camera: Camera,
    storage_group: StorageGroup,
    child: Option<(
        Child,
        ChildStdin,
        FramedRead<ChildStdout, LengthDelimitedCodec>,
    )>,

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

    fn start_worker(&mut self) {
        debug!(
            "Starting worker process for camera: {}, id: {}, stream: {}",
            self.camera.name, self.camera.name, self.camera.rtsp_url
        );
        let storage_path =
            Path::new(&self.storage_group.spec.storage_path).join(self.camera.name.clone());
        if std::fs::create_dir(&storage_path).is_err() {
            // The error returned by create_dir has no information so
            // we can't really distinguish between failure
            // scenarios. If the directory already exists everything
            // is fine, otherwise we fail later.
        }
        let worker_path = env::var("EXOPTICONWORKERS").unwrap_or_else(|_| "/".to_string());
        let executable_path: PathBuf = [worker_path, "capture_worker".to_string()].iter().collect();

        let hwaccel_method =
            env::var("EXOPTICON_HWACCEL_METHOD").unwrap_or_else(|_| "none".to_string());
        let mut cmd = process::Command::new(executable_path);
        cmd.arg(&self.camera.rtsp_url);
        cmd.arg(&storage_path);
        cmd.arg(hwaccel_method);
        cmd.stdout(Stdio::piped());
        cmd.stdin(Stdio::piped());

        let mut child = cmd.spawn().expect("Failed to launch");
        let stdout = child
            .stdout
            .take()
            .expect("Failed to open stdout on worker child");
        let stdin = child.stdin.take().expect("Failed to open stdin");
        let framed_stream = length_delimited::Builder::new().new_read(stdout);

        self.child = Some((child, stdin, framed_stream));
        self.state = State::Started;
    }

    fn worker_storage_path(&self) -> PathBuf {
        Path::new(&self.storage_group.spec.storage_path).join(self.camera.name.clone())
    }

    async fn reserve_file(&self) -> anyhow::Result<(i64, i64, String)> {
        let path = uuid_to_filename(&self.worker_storage_path(), Uuid::now_v7());
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("reserved filename has no parent directory"))?;
        fs::create_dir_all(parent).await?;

        let filename = path.to_string_lossy().to_string();
        let db_filename = filename.clone();
        let db = self.db.clone();
        let camera_name = self.camera.name.clone();
        let reserved_at = Utc::now();
        let (video_unit, video_file) = spawn_blocking(move || {
            db.reserve_video_segment(&camera_name, db_filename, reserved_at)
        })
        .await??;

        Ok((video_unit.id, video_file.id, filename))
    }

    async fn send_worker_command(&mut self, command: CaptureCommand) -> anyhow::Result<()> {
        let Some((_, stdin, _)) = &mut self.child else {
            anyhow::bail!("capture worker stdin is not available");
        };

        let frame = exserial::serialize_frame(&command)?;
        stdin.write_all(&frame).await?;
        stdin.flush().await?;
        Ok(())
    }

    async fn handle_reserve_file(&mut self) -> anyhow::Result<()> {
        let command = match self.reserve_file().await {
            Ok((video_unit_id, video_file_id, filename)) => CaptureCommand::FileReserved {
                video_unit_id,
                video_file_id,
                filename,
            },
            Err(err) => CaptureCommand::FileReservationFailed {
                message: err.to_string(),
            },
        };

        self.send_worker_command(command).await
    }

    async fn handle_file_opened(
        &mut self,
        video_unit_id: i64,
        video_file_id: i64,
        filename: String,
        begin_time: String,
    ) -> anyhow::Result<()> {
        let begin_time = begin_time.parse::<DateTime<Utc>>().expect("Parse failure!");
        debug!("Reserved file opened: {filename}");

        let db = self.db.clone();
        spawn_blocking(move || db.open_video_segment(video_unit_id, video_file_id, begin_time))
            .await??;

        self.state = State::Recording;
        Ok(())
    }

    async fn handle_close_file(
        &self,
        video_unit_id: i64,
        video_file_id: i64,
        filename: &str,
        end_time: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        if let Ok(metadata) = fs::metadata(filename).await {
            let db = self.db.clone();
            let file_size: i32 = metadata.len().try_into().unwrap_or(-1);
            spawn_blocking(move || {
                db.close_video_segment(video_unit_id, video_file_id, end_time, file_size)
            })
            .await??;
        }
        Ok(())
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
            CaptureMessage::ReserveFile => {
                self.handle_reserve_file().await?;
            }
            CaptureMessage::FileOpened {
                video_unit_id,
                video_file_id,
                filename,
                begin_time,
            } => {
                self.handle_file_opened(video_unit_id, video_file_id, filename, begin_time)
                    .await?;
            }

            CaptureMessage::EndFile {
                video_unit_id,
                video_file_id,
                filename,
                end_time,
            } => {
                let end_time = end_time.parse::<DateTime<Utc>>().expect("Parse failure!");
                self.handle_close_file(video_unit_id, video_file_id, &filename, end_time)
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

        let frame: CaptureMessage = bincode::deserialize(&item[..])?;

        self.message_to_action(frame).await?;

        Ok(())
    }

    async fn select_next(&mut self) -> anyhow::Result<bool> {
        match &mut self.child {
            Some((child, _, framed_stream)) => {
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
        let mut had_error = false;
        loop {
            if self.state == State::Ready {
                if let Err(e) = self.cleanup_unopened_video_segments().await {
                    error!("error cleaning unopened video segments: {}", e);
                }
                self.start_worker();
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

        if let Some((mut child, stdin, _)) = self.child.take() {
            drop(stdin);
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
