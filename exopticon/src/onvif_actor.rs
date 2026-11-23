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

//! Serializes ONVIF requests for one enabled camera.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, anyhow, ensure};
use oxvif::OnvifSession;
use tokio::{
    sync::{Notify, mpsc, oneshot},
    time::{Instant, timeout},
};
use tracing::instrument::WithSubscriber;
use url::Url;

use crate::{db::cameras::Camera, onvif_client::Target};

const REQUEST_INTERVAL: Duration = Duration::from_secs(1);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

struct RequestState {
    busy: bool,
    next_allowed_at: Instant,
}

/// The gate is shared by all viewers and checked for each key frame
/// request. Requests during the service call or cooldown are dropped.
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

type StreamUriReply = oneshot::Sender<anyhow::Result<Url>>;

#[derive(Clone)]
pub struct Handle {
    stream_requests: mpsc::Sender<StreamUriReply>,
    pub keyframe_requests: Arc<KeyframeRequestGate>,
}

impl Handle {
    pub async fn stream_uri(&self) -> anyhow::Result<Url> {
        let (reply, response) = oneshot::channel();
        self.stream_requests
            .send(reply)
            .await
            .map_err(|_| anyhow!("ONVIF actor stopped"))?;
        response.await.context("ONVIF actor stopped")?
    }
}

pub fn actor(
    camera: Camera,
    device_url: String,
    profile_token: String,
) -> (Handle, impl Future<Output = ()> + Send + 'static) {
    let (stream_requests, receiver) = mpsc::channel(1);
    let keyframe_requests = Arc::new(KeyframeRequestGate::new());
    let task = run(
        camera,
        device_url,
        profile_token,
        receiver,
        Arc::clone(&keyframe_requests),
    )
    .with_subscriber(tracing::subscriber::NoSubscriber::default());
    (
        Handle {
            stream_requests,
            keyframe_requests,
        },
        task,
    )
}

async fn run(
    camera: Camera,
    endpoint: String,
    token: String,
    mut stream_requests: mpsc::Receiver<StreamUriReply>,
    keyframe_requests: Arc<KeyframeRequestGate>,
) {
    // Reuse service discovery while fetching a new stream URI for every request.
    let mut session = None;
    loop {
        tokio::select! {
            biased;
            reply = stream_requests.recv() => {
                let Some(reply) = reply else { break };
                if reply.is_closed() {
                    continue;
                }
                let result = timeout(REQUEST_TIMEOUT, async {
                    let (client, version) = match session.take() {
                        Some((client, version)) => (client, Some(version)),
                        None => (connect(&camera, &endpoint).await?, None),
                    };
                    let (uri, version) = get_stream_uri(&client, &token, version).await?;
                    let url = authenticated_stream_url(&uri, &camera.username, &camera.password)?;
                    session = Some((client, version));
                    Ok(url)
                }).await.context("device request timed out").and_then(|result| result);
                let _ = reply.send(result);
            }
            () = keyframe_requests.wait() => {
                if let Some((client, version)) = &session {
                    keyframe_requests.mark_started();
                    let request = async {
                        match version {
                            MediaVersion::Media1 => client.media_set_synchronization_point(&token).await,
                            MediaVersion::Media2 => client.set_synchronization_point_media2(&token).await,
                        }
                    };
                    if !matches!(timeout(REQUEST_TIMEOUT, request).await, Ok(Ok(()))) {
                        warn!("ONVIF keyframe request failed for {}", camera.name);
                    }
                }
                keyframe_requests.finish();
            }
        }
    }
}

async fn connect(camera: &Camera, endpoint: &str) -> anyhow::Result<OnvifSession> {
    let endpoint = Url::parse(endpoint)?;
    let host = endpoint.host_str().context("missing ONVIF device host")?;
    let addresses = tokio::net::lookup_host((host.trim_matches(['[', ']']), 0))
        .await?
        .collect();
    Target {
        host: host.to_owned(),
        addresses,
    }
    .session(&endpoint, &camera.username, &camera.password)?
    .with_clock_sync()
    .build()
    .await
    .context("could not discover ONVIF services")
}

#[derive(Clone, Copy)]
enum MediaVersion {
    Media1,
    Media2,
}

async fn get_stream_uri(
    session: &OnvifSession,
    token: &str,
    version: Option<MediaVersion>,
) -> anyhow::Result<(String, MediaVersion)> {
    if !matches!(version, Some(MediaVersion::Media2)) && session.capabilities().media.url.is_some()
    {
        match session.get_stream_uri(token).await {
            Ok(uri) => return Ok((uri.uri, MediaVersion::Media1)),
            Err(error) if version.is_some() || session.capabilities().media2.url.is_none() => {
                return Err(error.into());
            }
            Err(_) => {}
        }
    }
    Ok((
        session.get_stream_uri_media2(token).await?,
        MediaVersion::Media2,
    ))
}

fn authenticated_stream_url(uri: &str, username: &str, password: &str) -> anyhow::Result<Url> {
    let mut url = Url::parse(uri)?;
    ensure!(
        matches!(url.scheme(), "rtsp" | "rtsps") && url.has_host(),
        "ONVIF did not return an RTSP URL with a host"
    );
    if !username.is_empty() || !password.is_empty() {
        // URL setters escape userinfo delimiters. Escape literal percent signs as well.
        url.set_username(&username.replace('%', "%25"))
            .map_err(|()| anyhow!("cannot set RTSP username"))?;
        let password = password.replace('%', "%25");
        url.set_password((!password.is_empty()).then_some(password.as_str()))
            .map_err(|()| anyhow!("cannot set RTSP password"))?;
    }
    Ok(url)
}
