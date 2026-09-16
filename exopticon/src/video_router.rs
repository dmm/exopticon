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

use crate::keyframe_requests::KeyframeRequestGate;
use tokio::sync::{RwLock, mpsc};

use crate::{capture_actor::VideoPacket, webrtc_client::ClientId};

type VideoPacketVec = Vec<(ClientId, mpsc::Sender<VideoPacket>)>;

pub struct VideoRouter {
    // camera name → list of (client_id, sender) pairs
    subscriptions: Arc<RwLock<HashMap<String, VideoPacketVec>>>,
    keyframe_requests: RwLock<HashMap<String, Arc<KeyframeRequestGate>>>,
}

impl VideoRouter {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            keyframe_requests: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register_keyframe_requests(
        &self,
        camera_name: String,
        requests: Arc<KeyframeRequestGate>,
    ) {
        self.keyframe_requests
            .write()
            .await
            .insert(camera_name, requests);
    }

    pub async fn unregister_keyframe_requests(&self, camera_name: &str) {
        self.keyframe_requests.write().await.remove(camera_name);
    }

    pub async fn request_keyframe(&self, client_id: ClientId, camera_name: &str) {
        let subscribed = self
            .subscriptions
            .read()
            .await
            .get(camera_name)
            .is_some_and(|clients| clients.iter().any(|(id, _)| *id == client_id));
        if subscribed && let Some(requests) = self.keyframe_requests.read().await.get(camera_name) {
            requests.request();
        }
    }

    pub async fn update_subscriptions(
        &self,
        client_id: ClientId,
        camera_names: Vec<String>,
        sender: mpsc::Sender<VideoPacket>,
    ) {
        let mut subs = self.subscriptions.write().await;

        // Remove this client from ALL cameras first
        for clients in subs.values_mut() {
            clients.retain(|(id, _)| *id != client_id);
        }

        // Add this client to subscribed cameras
        for camera_name in camera_names {
            subs.entry(camera_name)
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

        if let Some(clients) = subs.get(&packet.camera_name) {
            for (_, tx) in clients {
                // try_send to avoid blocking if client is slow
                let _ = tx.try_send(packet.clone());
            }
        }
    }
}

impl Default for VideoRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::FutureExt;
    use uuid::Uuid;

    #[tokio::test(start_paused = true)]
    async fn viewers_share_camera_notifications_and_lifecycle_is_respected() {
        let router = VideoRouter::new();
        let first = Arc::new(KeyframeRequestGate::new());
        let second = Arc::new(KeyframeRequestGate::new());
        router
            .register_keyframe_requests("front".to_string(), Arc::clone(&first))
            .await;
        router
            .register_keyframe_requests("back".to_string(), Arc::clone(&second))
            .await;
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let (tx, _rx) = mpsc::channel(1);
        router
            .update_subscriptions(a, vec!["front".to_string(), "back".to_string()], tx.clone())
            .await;
        router
            .update_subscriptions(b, vec!["front".to_string()], tx)
            .await;
        for client in [a, b, a, b] {
            router.request_keyframe(client, "front").await;
        }
        assert!(first.wait().now_or_never().is_some());
        assert!(first.wait().now_or_never().is_none());
        assert!(second.wait().now_or_never().is_none());
        router.request_keyframe(b, "back").await;
        assert!(second.wait().now_or_never().is_none());
        router.request_keyframe(a, "back").await;
        assert!(second.wait().now_or_never().is_some());
        first.finish();
        tokio::time::advance(std::time::Duration::from_secs(1)).await;
        router.unsubscribe(a).await;
        router.request_keyframe(a, "front").await;
        assert!(first.wait().now_or_never().is_none());
        router.unregister_keyframe_requests("front").await;
        router.request_keyframe(b, "front").await;
        assert!(first.wait().now_or_never().is_none());
        let replacement = Arc::new(KeyframeRequestGate::new());
        router
            .register_keyframe_requests("front".to_string(), Arc::clone(&replacement))
            .await;
        router.request_keyframe(b, "front").await;
        assert!(replacement.wait().now_or_never().is_some());
        assert!(first.wait().now_or_never().is_none());
    }
}
