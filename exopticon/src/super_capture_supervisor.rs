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
use std::time::Duration;

use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio::task::{spawn_blocking, JoinError};

use crate::api::cameras::Camera;
use crate::super_capture_actor::VideoPacket;
use crate::super_capture_actor::{CaptureActor, CaptureActorCommands};

pub enum CaptureSupervisorCommand {
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
    command_sender: mpsc::Sender<CaptureSupervisorCommand>,
    command_receiver: mpsc::Receiver<CaptureSupervisorCommand>,
    capture_channels: HashMap<i32, mpsc::Sender<CaptureActorCommands>>,
    capture_handles: FuturesUnordered<JoinHandle<i32>>,
    packet_sender: broadcast::Sender<VideoPacket>,
}

impl CaptureSupervisor {
    pub fn new(db: crate::db::Service) -> Self {
        let (command_sender, command_receiver) = mpsc::channel(1);
        let (packet_sender, _packet_receiver) = broadcast::channel(10);

        Self {
            state: State::Ready,
            db,
            command_sender,
            command_receiver,
            capture_channels: HashMap::new(),
            capture_handles: FuturesUnordered::new(),
            packet_sender,
        }
    }

    pub fn get_command_channel(&self) -> mpsc::Sender<CaptureSupervisorCommand> {
        self.command_sender.clone()
    }

    pub fn get_packet_sender(&self) -> broadcast::Sender<VideoPacket> {
        self.packet_sender.clone()
    }

    async fn stop_cameras(&mut self) -> anyhow::Result<()> {
        for (id, ch) in &self.capture_channels {
            info!("Telling camera {} to stop!", id);
            if let Err(_err) = ch.send(CaptureActorCommands::Stop).await {
                error!("Failed to send stop command!");
            }
        }

        self.capture_channels.clear();
        Ok(())
    }

    async fn start_cameras(&mut self) -> anyhow::Result<()> {
        info!("Starting capture actors...");
        // fetch cameras
        let db = self.db.clone();
        let cameras: Vec<Camera> = spawn_blocking(move || db.fetch_all_cameras())
            .await??
            .into_iter()
            .filter(|c| c.common.enabled)
            .collect();

        for c in cameras {
            let db = self.db.clone();
            let storage_group =
                spawn_blocking(move || db.fetch_storage_group(c.common.storage_group_id)).await??;
            let id = c.id;
            let (command_sender, command_receiver) = mpsc::channel(1);
            let actor = CaptureActor::new(
                self.db.clone(),
                c,
                storage_group,
                command_receiver,
                self.packet_sender.clone(),
            );
            self.capture_channels.insert(id, command_sender);
            let fut = tokio::spawn(actor.run());
            self.capture_handles.push(fut);
        }

        Ok(())
    }

    fn handle_supervisor_command(&mut self, cmd: &CaptureSupervisorCommand) {
        info!("Got supervisor command!");
        match cmd {
            CaptureSupervisorCommand::RestartAll => {
                info!("Got capture restart all command!");
                self.state = State::Restarting;
            }
        }
    }

    fn handle_camera_event(&mut self, _res: &Result<i32, JoinError>) {
        match self.state {
            State::Running => {
                error!("Capture task died but we're supposed to be running. Restart all cameras");
                self.state = State::Restarting;
            }
            State::Ready | State::Restarting | State::Draining => {
                // Ready => ignoring
                // Restarting => update task count
            }
        }
    }

    async fn handle_tick(&mut self) {
        info!(
            "Capture supervisor tick! state: {:?}, # handles: {}",
            self.state,
            self.capture_handles.len()
        );

        match self.state {
            State::Ready => {
                if let Err(e) = self.start_cameras().await {
                    error!("Error starting cameras! {}", e);
                    return;
                }
                self.state = State::Running;
            }
            State::Running => {
                // everything is fine
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
        let mut interval = tokio::time::interval(Duration::from_secs(5));
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
