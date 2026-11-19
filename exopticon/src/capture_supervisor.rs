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
    Shutdown,
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

    async fn stop_cameras(&mut self) {
        for (id, ch) in &self.capture_channels {
            info!("Telling camera {} to stop!", id);
            if let Err(_err) = ch.send(capture_actor::Command::Stop).await {
                error!("Failed to send stop command!");
            }
        }

        self.capture_channels.clear();
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

        let full_reload = camera_name.is_none();
        let all_cameras: Vec<Camera> = spawn_blocking(move || {
            if full_reload {
                // A full reload starts only after every previous worker has joined.
                db.release_pending_camera_deletions()?;
            }
            db.fetch_all_camera_rows()
        })
        .await??;
        {
            let mut statuses = self.camera_status_registry.write().await;
            if full_reload {
                statuses.clear();
            }
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

    async fn start_all(&mut self) {
        self.state = match self.start_cameras(None).await {
            Ok(()) => State::Running,
            Err(err) => {
                error!("Error starting cameras: {err}");
                State::Restarting
            }
        };
    }

    async fn finish_restart_if_drained(&mut self) {
        if matches!(self.state, State::Draining) && self.capture_handles.is_empty() {
            self.start_all().await;
        }
    }

    async fn restart(&mut self) {
        self.stop_cameras().await;
        self.stopped_camera_names.clear();
        self.state = State::Draining;
        self.finish_restart_if_drained().await;
    }

    async fn handle_tick(&mut self) {
        debug!(
            "Capture supervisor tick! state: {:?}, # handles: {}",
            self.state,
            self.capture_handles.len()
        );
        match self.state {
            State::Ready => self.start_all().await,
            State::Running => {
                for name in std::mem::take(&mut self.stopped_camera_names) {
                    if let Err(err) = self.start_cameras(Some(name.clone())).await {
                        error!("Error restarting camera {name}: {err}");
                        self.restart().await;
                        break;
                    }
                }
            }
            State::Restarting => self.restart().await,
            State::Draining => self.finish_restart_if_drained().await,
        }
    }

    pub async fn supervise(mut self) -> anyhow::Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(15));
        loop {
            let tick = interval.tick();
            tokio::pin!(tick); // required for select()

            tokio::select! {
                biased;
                Some(cmd) = self.command_receiver.recv() => match cmd {
                    Command::RestartAll => {
                        info!("Got capture restart all command!");
                        self.restart().await;
                    }
                    Command::Shutdown => {
                        info!("Shutting down capture supervisor...");
                        self.command_receiver.close();
                        self.stop_cameras().await;
                        while let Some(result) = self.capture_handles.next().await {
                            if let Err(error) = result {
                                error!("Capture actor failed during shutdown: {}", error);
                            }
                        }
                        return Ok(());
                    }
                },
                Some(camera_id) = self.capture_handles.next() => {
                    self.handle_camera_event(&camera_id);
                    self.finish_restart_if_drained().await;
                },
                _inst = tick => self.handle_tick().await,
                else => break
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::{RwLock, oneshot};
    use tokio::time::timeout;

    fn supervisor() -> CaptureSupervisor {
        CaptureSupervisor::new(
            crate::db::Service::new(":memory:"),
            Arc::new(VideoRouter::new()),
            Arc::new(RwLock::new(HashMap::new())),
        )
    }

    #[tokio::test]
    async fn shutdown_stops_all_cameras_and_waits_for_cleanup() {
        let mut supervisor = supervisor();
        supervisor.state = State::Running;
        let commands = supervisor.get_command_channel();
        let (stopped_sender, mut stopped_receiver) = mpsc::channel(2);
        let mut finish_senders = Vec::new();

        for name in ["front", "back"] {
            let (sender, mut receiver) = mpsc::channel(1);
            let (finish_sender, finish_receiver) = oneshot::channel();
            let stopped_sender = stopped_sender.clone();
            supervisor.capture_channels.insert(name.to_string(), sender);
            supervisor.capture_handles.push(tokio::spawn(async move {
                assert!(matches!(
                    receiver.recv().await,
                    Some(capture_actor::Command::Stop)
                ));
                stopped_sender.send(name).await.unwrap();
                finish_receiver.await.unwrap();
                name.to_string()
            }));
            finish_senders.push(finish_sender);
        }

        commands.send(Command::Shutdown).await.unwrap();
        let task = tokio::spawn(supervisor.supervise());
        for _ in 0..2 {
            timeout(Duration::from_secs(1), stopped_receiver.recv())
                .await
                .unwrap()
                .unwrap();
        }
        assert!(!task.is_finished());
        assert!(commands.send(Command::RestartAll).await.is_err());

        finish_senders.pop().unwrap().send(()).unwrap();
        assert!(!task.is_finished());
        finish_senders.pop().unwrap().send(()).unwrap();
        timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn shutdown_before_startup_exits_without_starting_cameras() {
        let supervisor = supervisor();
        let commands = supervisor.get_command_channel();
        commands.send(Command::Shutdown).await.unwrap();

        timeout(Duration::from_secs(1), supervisor.supervise())
            .await
            .unwrap()
            .unwrap();
        assert!(commands.is_closed());
    }
}
