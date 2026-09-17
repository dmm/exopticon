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

//! One sequential ONVIF request task per capture actor, shared by its
//! viewers.

use std::{
    net::Ipv6Addr,
    sync::{Arc, Mutex},
    time::Duration,
};

use oxvif::OnvifSession;
use tokio::{sync::Notify, time::Instant};

use crate::db::cameras::Camera;

const REQUEST_INTERVAL: Duration = Duration::from_secs(1);

pub fn device_url(host: &str, port: u16) -> String {
    let host = if host.parse::<Ipv6Addr>().is_ok() {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    format!("http://{host}:{port}/onvif/device_service")
}

struct RequestState {
    busy: bool,
    next_allowed_at: Instant,
}

/// The gate is shared by all viewers and checked for each key frame
/// request. A request received during the service call or cooldown
/// area dropped, so they don't become stale.
pub struct KeyframeRequestGate {
    notification: Notify,
    state: Mutex<RequestState>,
}

impl Default for KeyframeRequestGate {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyframeRequestGate {
    pub fn new() -> Self {
        Self {
            notification: Notify::new(),
            state: Mutex::new(RequestState {
                busy: false,
                next_allowed_at: Instant::now(),
            }),
        }
    }

    pub fn request(&self) {
        let mut state = self.state.lock().expect("keyframe request state");
        let now = Instant::now();
        if state.busy || now < state.next_allowed_at {
            return;
        }
        state.busy = true;
        state.next_allowed_at = now + REQUEST_INTERVAL;
        drop(state);
        self.notification.notify_one();
    }

    pub async fn wait(&self) {
        self.notification.notified().await;
    }

    fn mark_started(&self) {
        self.state
            .lock()
            .expect("keyframe request state")
            .next_allowed_at = Instant::now() + REQUEST_INTERVAL;
    }

    pub(super) fn finish(&self) {
        self.state.lock().expect("keyframe request state").busy = false;
    }
}

pub async fn run(camera: Camera, requests: Arc<KeyframeRequestGate>) {
    let Some(profile_token) = camera.onvif_profile_token.as_deref() else {
        return;
    };
    let Ok(port) = u16::try_from(camera.onvif_port) else {
        warn!(
            "Cannot request keyframes for {}: invalid ONVIF port",
            camera.name
        );
        return;
    };
    let endpoint = device_url(&camera.ip, port);
    let mut session = None;
    loop {
        requests.wait().await;
        let result = async {
            if session.is_none() {
                session = Some(
                    OnvifSession::builder(&endpoint)
                        .with_credentials(camera.username.clone(), camera.password.clone())
                        .with_clock_sync()
                        .build()
                        .await?,
                );
            }
            // Discovery may have taken longer than the interval.
            // Space actual synchronization requests from their start,
            // not discovery's start.
            requests.mark_started();

            session
                .as_ref()
                .expect("media client discovered")
                .media_set_synchronization_point(profile_token)
                .await
        }
        .await;
        if let Err(error) = result {
            warn!(
                "ONVIF keyframe request failed for {}: {}",
                camera.name, error
            );
            // Rediscover on the next requested attempt, without an
            // automatic retry or bypassing the cooldown after
            // failures.
            session = None;
        }
        requests.finish();
    }
}
