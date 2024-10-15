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
};

use bytes::BytesMut;
use chrono::{DateTime, Utc};
use futures::stream::StreamExt;
use tokio::{
    fs,
    process::{self, Child, ChildStdin, ChildStdout},
    sync::{broadcast, mpsc},
    task::spawn_blocking,
};
use tokio_util::codec::{length_delimited, FramedRead, LengthDelimitedCodec};

use crate::{
    api::{
        cameras::Camera,
        storage_groups::StorageGroup,
        video_units::{CreateVideoFile, CreateVideoUnit},
    },
    db::uuid::Uuid,
};
use exserial::models::CaptureMessage;

#[derive(Clone)]
pub struct VideoPacket {
    pub camera_id: Uuid,
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
    video_segment_id: Option<(Uuid, Uuid)>,

    /// Video Packet Sender
    sender: broadcast::Sender<VideoPacket>,
    /// Supervisor command channel
    command_receiver: mpsc::Receiver<Command>,
}

impl CaptureActor {
    pub fn new(
        db: crate::db::Service,
        camera: Camera,
        storage_group: StorageGroup,
        command_receiver: mpsc::Receiver<Command>,
        sender: broadcast::Sender<VideoPacket>,
    ) -> Self {
        Self {
            state: State::Ready,
            db,
            camera,
            storage_group,
            child: None,
            video_segment_id: None,
            command_receiver,
            sender,
        }
    }

    fn start_worker(&mut self) {
        debug!(
            "Starting worker process for camera: {}, id: {}, stream: {}",
            self.camera.common.name, self.camera.id, self.camera.common.rtsp_url
        );
        let storage_path =
            Path::new(&self.storage_group.storage_path).join(self.camera.id.to_string());
        if std::fs::create_dir(&storage_path).is_err() {
            // The error returned by create_dir has no information so
            // we can't really distinguish between failure
            // scenarios. If the directory already exists everything
            // is fine, otherwise we fail later.
        }
        let worker_path = env::var("EXOPTICONWORKERS").unwrap_or_else(|_| "/".to_string());
        let executable_path: PathBuf = [worker_path, "cworkers/captureworker".to_string()]
            .iter()
            .collect();

        let hwaccel_method =
            env::var("EXOPTICON_HWACCEL_METHOD").unwrap_or_else(|_| "none".to_string());
        let mut cmd = process::Command::new(executable_path);
        cmd.arg(&self.camera.common.rtsp_url);
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

    async fn handle_new_file(
        &mut self,
        filename: String,
        begin_time: String,
    ) -> anyhow::Result<()> {
        let new_video_unit_id = Uuid::new_v4();
        let date = begin_time.parse::<DateTime<Utc>>().expect("Parse failure!");

        let create_video_unit = CreateVideoUnit {
            camera_id: self.camera.id,
            monotonic_index: 0,
            begin_time: date,
            end_time: date,
            id: new_video_unit_id,
        };
        let create_video_file = CreateVideoFile {
            filename,
            size: 0,
            video_unit_id: new_video_unit_id,
        };
        let db = self.db.clone();
        let (video_unit, video_file) =
            spawn_blocking(move || db.create_video_segment(&create_video_unit, create_video_file))
                .await??;

        self.video_segment_id = Some((video_unit.id, video_file.id));
        self.state = State::Recording;
        Ok(())
    }

    async fn handle_close_file(
        &mut self,
        filename: &str,
        end_time: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        if let (Some((video_unit_id, video_file_id)), Ok(metadata)) =
            (self.video_segment_id, fs::metadata(filename).await)
        {
            let db = self.db.clone();
            let file_size: i32 = metadata.len().try_into().unwrap_or(-1);
            spawn_blocking(move || {
                db.close_video_segment(video_unit_id, video_file_id, end_time, file_size)
            })
            .await??;
        }
        Ok(())
    }
    fn handle_packet(&mut self, data: Vec<u8>, timestamp: i64, duration: i64) {
        if let Err(_e) = self.sender.send(VideoPacket {
            camera_id: self.camera.id,
            data,
            timestamp,
            duration,
        }) {
            // error!(
            //     "Error sending packet! Camera: {}, {}, {} ",
            //     self.camera.id, self.camera.common.name, e
            // );
        }
    }
    async fn message_to_action(&mut self, msg: CaptureMessage) -> anyhow::Result<()> {
        match msg {
            CaptureMessage::Log { level, message } => {
                log!(
                    level,
                    "capture worker {} {} log: {}",
                    self.camera.id,
                    self.camera.common.name,
                    message
                );
            }
            CaptureMessage::Frame {
                jpeg: _,
                offset: _,
                unscaled_width: _,
                unscaled_height: _,
            }
            | CaptureMessage::ScaledFrame {
                jpeg: _,
                offset: _,
                unscaled_width: _,
                unscaled_height: _,
            } => {}
            CaptureMessage::Packet {
                data,
                timestamp,
                duration,
            } => {
                // TODO handle packets...
                self.handle_packet(data, timestamp, duration);
            }
            CaptureMessage::NewFile {
                filename,
                begin_time,
            } => self.handle_new_file(filename, begin_time).await?,

            CaptureMessage::EndFile { filename, end_time } => {
                let end_time = end_time.parse::<DateTime<Utc>>().expect("Parse failure!");
                self.handle_close_file(&filename, end_time).await?;
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
        if let Some((child, _, framed_stream)) = &mut self.child {
            tokio::select! {
                biased;
                _ = child.wait() => {
                    info!(
                        "Capture process for {} {} died. Restarting...",
                        self.camera.id, self.camera.common.name,
                    );
                    return Ok(false);
                }
                Some(Command::Stop) = self.command_receiver.recv() => {
                    info!("Received stop command for {} {}.",
                          self.camera.id, self.camera.common.name,
                    );
                    return Ok(false)
                }
                Some(msg) = framed_stream.next() => self.stream_handler(msg).await?,
                else => return Ok(false)
            }
        } else {
            tokio::select! {
                Some(Command::Stop) = self.command_receiver.recv() => return Ok(false),
                else => return Ok(false),
            }
        }

        Ok(true)
    }

    pub async fn run(mut self) -> Uuid {
        loop {
            if self.state == State::Ready {
                self.start_worker();
            }
            let res = self.select_next().await;
            match res {
                Ok(true) => {}
                Ok(false) => break,
                Err(e) => {
                    error!("Error {}", e);
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
        self.camera.id
    }
}
