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

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use futures::stream::FuturesUnordered;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::task::{JoinError, spawn_blocking};

use crate::api::cameras::CameraStatus;
use crate::capture_actor;
use crate::video_router::VideoRouter;
use crate::{CameraStatusRegistry, db::cameras::Camera};

pub enum Command {
    RestartAll,
}

#[derive(Debug)]
enum State {
    Ready,
    Running,
    Restarting,
    Draining,
}

pub struct CaptureSupervisor {
    state: State,
    db: crate::db::Service,
    stopped_camera_names: Vec<String>,
    command_sender: mpsc::Sender<Command>,
    command_receiver: mpsc::Receiver<Command>,
    capture_channels: HashMap<String, mpsc::Sender<capture_actor::Command>>,
    capture_handles: FuturesUnordered<JoinHandle<String>>,
    video_router: Arc<VideoRouter>,
    camera_status_registry: CameraStatusRegistry,
}

impl CaptureSupervisor {
    pub fn new(
        db: crate::db::Service,
        video_router: Arc<VideoRouter>,
        camera_status_registry: CameraStatusRegistry,
    ) -> Self {
        let (command_sender, command_receiver) = mpsc::channel(1);

        Self {
            state: State::Ready,
            db,
            stopped_camera_names: Vec::new(),
            command_sender,
            command_receiver,
            capture_channels: HashMap::new(),
            capture_handles: FuturesUnordered::new(),
            video_router,
            camera_status_registry,
        }
    }

    pub fn get_command_channel(&self) -> mpsc::Sender<Command> {
        self.command_sender.clone()
    }

    async fn stop_cameras(&mut self) -> anyhow::Result<()> {
        for (id, ch) in &self.capture_channels {
            info!("Telling camera {} to stop!", id);
            if let Err(_err) = ch.send(capture_actor::Command::Stop).await {
                error!("Failed to send stop command!");
            }
        }

        self.capture_channels.clear();
        Ok(())
    }

    async fn start_camera(&mut self, c: Camera) -> anyhow::Result<()> {
        let db = self.db.clone();
        let storage_group_name = c.storage_group_name.clone();
        let storage_group =
            spawn_blocking(move || db.fetch_storage_group(&storage_group_name)).await??;
        let name = c.name.clone();
        let (command_sender, command_receiver) = mpsc::channel(1);
        let actor = capture_actor::CaptureActor::new(
            self.db.clone(),
            c,
            storage_group,
            command_receiver,
            self.video_router.clone(),
            self.camera_status_registry.clone(),
        );
        self.camera_status_registry
            .write()
            .await
            .insert(name.clone(), CameraStatus::starting());
        self.capture_channels.insert(name, command_sender);
        let fut = tokio::spawn(actor.run());
        self.capture_handles.push(fut);
        Ok(())
    }

    async fn start_cameras(&mut self, camera_name: Option<String>) -> anyhow::Result<()> {
        info!("Starting capture actors...");
        // fetch cameras
        let db = self.db.clone();

        let all_cameras: Vec<Camera> = spawn_blocking(move || db.fetch_all_camera_rows()).await??;
        {
            let mut statuses = self.camera_status_registry.write().await;
            for camera in &all_cameras {
                if !camera.enabled {
                    statuses.insert(camera.name.clone(), CameraStatus::disabled());
                }
            }
        }

        let mut cameras: Vec<Camera> = all_cameras.into_iter().filter(|c| c.enabled).collect();

        if let Some(name) = camera_name {
            cameras.retain(|c| c.name == name);
        }

        for c in cameras {
            self.start_camera(c).await?;
        }

        Ok(())
    }

    fn handle_supervisor_command(&mut self, cmd: &Command) {
        info!("Got supervisor command!");
        match cmd {
            Command::RestartAll => {
                info!("Got capture restart all command!");
                self.state = State::Restarting;
            }
        }
    }

    fn handle_camera_event(&mut self, res: &Result<String, JoinError>) {
        match self.state {
            State::Running => {
                if let Ok(name) = res {
                    error!(
                        "Capture task died but we're supposed to be running. camera name {}",
                        name
                    );
                    self.stopped_camera_names.push(name.clone());
                } else {
                    error!("Capture task died, restart all cameras..");
                    self.state = State::Restarting;
                }
            }
            State::Ready | State::Restarting | State::Draining => {
                // Ready => ignoring
                // Restarting => update task count
            }
        }
    }

    async fn handle_tick(&mut self) {
        debug!(
            "Capture supervisor tick! state: {:?}, # handles: {}",
            self.state,
            self.capture_handles.len()
        );

        match self.state {
            State::Ready => {
                if let Err(e) = self.start_cameras(None).await {
                    error!("Error starting cameras! {}", e);
                    return;
                }
                self.state = State::Running;
            }
            State::Running => {
                // everything is fine

                // check for stopped cameras
                for name in self.stopped_camera_names.clone() {
                    if let Err(e) = self.start_cameras(Some(name.clone())).await {
                        error!("error restarting camera {}, {}. restarting all.", name, e);
                        self.state = State::Restarting;
                    }
                }
                self.stopped_camera_names.clear();
            }
            State::Restarting => {
                if let Err(e) = self.stop_cameras().await {
                    error!("Error stopping cameras! {}", e);
                    return;
                }
                self.state = State::Draining;
            }
            State::Draining => {
                if self.capture_handles.is_empty() {
                    self.state = State::Ready;
                }
            }
        }
    }

    pub async fn supervise(mut self) -> anyhow::Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(15));
        loop {
            let tick = interval.tick();
            tokio::pin!(tick); // required for select()

            tokio::select! {
                Some(cmd) = self.command_receiver.recv()
                    => self.handle_supervisor_command(&cmd),
                Some(camera_id) = self.capture_handles.next() => self.handle_camera_event(&camera_id),
                _inst = tick => self.handle_tick().await,
                else => break
            }
        }
        Ok(())
    }
}
