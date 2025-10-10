/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2025 David Matthew Mattli <dmm@mattli.us>
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

use std::{collections::HashMap, sync::Arc};

use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use crate::{capture_actor::VideoPacket, webrtc_client::ClientId};
pub struct VideoRouter {
    // camera_id → list of (client_id, sender) pairs
    subscriptions: Arc<RwLock<HashMap<Uuid, Vec<(ClientId, mpsc::Sender<VideoPacket>)>>>>,
}

impl VideoRouter {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update_subscriptions(
        &self,
        client_id: ClientId,
        camera_ids: Vec<Uuid>,
        sender: mpsc::Sender<VideoPacket>,
    ) {
        let mut subs = self.subscriptions.write().await;

        // Remove this client from ALL cameras first
        for clients in subs.values_mut() {
            clients.retain(|(id, _)| *id != client_id);
        }

        // Add this client to subscribed cameras
        for camera_id in camera_ids {
            subs.entry(camera_id)
                .or_insert_with(Vec::new)
                .push((client_id, sender.clone()));
        }
    }

    pub async fn unsubscribe(&self, client_id: ClientId) {
        let mut subs = self.subscriptions.write().await;
        for clients in subs.values_mut() {
            clients.retain(|(id, _)| *id != client_id);
        }
    }

    pub async fn send_video(&self, packet: VideoPacket) {
        let subs = self.subscriptions.read().await;

        if let Some(clients) = subs.get(&packet.camera_id) {
            for (_, tx) in clients {
                // try_send to avoid blocking if client is slow
                let _ = tx.try_send(packet.clone());
            }
        }
    }
}
