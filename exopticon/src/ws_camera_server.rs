/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
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

#![allow(clippy::empty_enum)]
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use actix::{Actor, AsyncContext, Context, Handler, Message, Recipient, Supervised, SystemService};
use base64::STANDARD;
use uuid::Uuid;

use crate::models::Observation;

base64_serde_type!(Base64Standard, STANDARD);

/// Available frame types
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Hash, Serialize)]
pub enum FrameResolution {
    /// Standard definition frame, 480p
    SD,
    /// High definition frame, camera native resolution
    HD,
}

/// Description of source that produced a `CameraFrame`
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "kind")]
pub enum FrameSource {
    /// Camera with camera id
    Camera {
        /// id of camera
        #[serde(rename = "cameraId")]
        camera_id: i32,
        analysis_offset: Duration,
    },
    /// Analysis Engine, with engine id
    AnalysisEngine {
        /// id of source analysis engine
        #[serde(rename = "analysisEngineId")]
        analysis_engine_id: i32,
        analysis_offset: Duration,
    },
    /// Video Playback
    Playback {
        /// Playback id, must be unique per socket
        id: u64,
    },
}

/// An actor address that can receive `CameraFrame` messages
type Client = Recipient<CameraFrame>;

// MESSAGES
/// Represents a frame of video
#[derive(Clone, Message, Serialize, Deserialize, Eq, PartialEq)]
#[rtype(result = "()")]
pub struct CameraFrame {
    /// id of camera that produced frame
    pub camera_id: i32,
    /// jpeg image data
    #[serde(with = "Base64Standard")]
    pub jpeg: Vec<u8>,
    /// observations
    pub observations: Vec<Observation>,
    /// resolution of frame
    pub resolution: FrameResolution,
    /// source of frame
    pub source: FrameSource,
    /// id of video unit
    pub video_unit_id: Uuid,
    /// offset from beginning of video unit
    pub offset: i64,
    /// original width of image
    pub unscaled_width: i32,
    /// original height of image,
    pub unscaled_height: i32,
}

/// Subscription subject, used to subscribe and unsubscribe
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Hash, Serialize)]
pub enum SubscriptionSubject {
    /// A camera id and frame resolution
    Camera(i32, FrameResolution),
    /// Analysis engine id
    AnalysisEngine(i32),
    /// Playback id, name, initial video unit id, initial offset
    Playback(u64, i32, i64),
}

impl From<&CameraFrame> for SubscriptionSubject {
    fn from(item: &CameraFrame) -> Self {
        match item.source {
            FrameSource::AnalysisEngine {
                analysis_engine_id,
                analysis_offset: _,
            } => Self::AnalysisEngine(analysis_engine_id),
            FrameSource::Camera {
                camera_id,
                analysis_offset: _,
            } => Self::Camera(camera_id, item.resolution.clone()),
            FrameSource::Playback { id } => Self::Playback(id, 0, 0),
        }
    }
}

/// subscribe message
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub struct Subscribe {
    /// subscription subject
    pub subject: SubscriptionSubject,
    /// address of WsSession to send frames
    pub client: Client,
}

/// Unsubscribe message
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub struct Unsubscribe {
    /// unsubscription subject
    pub subject: SubscriptionSubject,
    /// Address of WsSession to stop sending frames
    pub client: Client,
}

// Server definitions

/// Type of camera frames
#[derive(Clone, PartialEq, Eq, Hash)]
struct FrameType {
    /// Camera id, identifies source of frame
    pub camera_id: i32,
    /// Represents resolution of frame
    pub resolution: FrameResolution,
}

/// Represents the `WsCameraServer` actor
#[derive(Default)]
pub struct WsCameraServer {
    /// Collection of Client's subscribed to a particular FrameType
    subscriptions: HashMap<SubscriptionSubject, HashSet<Client>>,
}

impl WsCameraServer {
    /// Add `WsSession` subscriber
    #[allow(clippy::option_if_let_else)]
    fn add_subscriber(&mut self, subject: &SubscriptionSubject, client: Client) -> bool {
        if let Some(subscription) = self.subscriptions.get_mut(subject) {
            subscription.insert(client)
        } else {
            self.subscriptions.insert(subject.clone(), HashSet::new());
            self.subscriptions
                .get_mut(subject)
                .expect("The subscriptions HashSet we just inserted is missing.")
                .insert(client)
        }
    }

    /// Removes `WsSession` as subscriber
    fn remove_subscriber(&mut self, subject: &SubscriptionSubject, client: &Client) {
        if let Some(subscription) = self.subscriptions.get_mut(subject) {
            if subscription.remove(client) {
                debug!("Removing subscriber...",);
            } else {
                debug!("Couldn't find the subscription camera...");
            }
        }
    }

    /// Sends frame to subscribed `WsSession`s
    ///
    /// # Arguments
    ///
    /// * `frame` - `CameraFram` to send
    /// * `ctx` - `WsCameraServer` Context
    ///
    fn send_frame(&mut self, frame: &CameraFrame, ctx: &<Self as Actor>::Context) {
        let subject = match &frame.source {
            FrameSource::Camera { camera_id, .. } => {
                SubscriptionSubject::Camera(*camera_id, frame.resolution.clone())
            }

            FrameSource::AnalysisEngine {
                analysis_engine_id, ..
            } => SubscriptionSubject::AnalysisEngine(*analysis_engine_id),
            // FrameSource::Playback is not really used... probably remove this...
            FrameSource::Playback { id, .. } => SubscriptionSubject::Playback(*id, 0, 0),
        };
        if let Some(subscription) = self.subscriptions.get(&subject) {
            let sub_count = subscription.len();
            for client in subscription.iter() {
                if client.do_send(frame.clone()).is_err() {
                    debug!(
                        "Send failed for {} {:?}, {} subscribers",
                        &frame.camera_id, &frame.resolution, sub_count
                    );
                    ctx.address().do_send(Unsubscribe {
                        subject: subject.clone(),
                        client: client.clone(),
                    });
                }
            }
        }
    }
}

impl Actor for WsCameraServer {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("Starting ws_camera_server...");
    }
}

impl Handler<Subscribe> for WsCameraServer {
    type Result = ();

    fn handle(&mut self, msg: Subscribe, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Adding subscriber: {:?}", &msg.subject);
        self.add_subscriber(&msg.subject, msg.client);
    }
}

impl Handler<Unsubscribe> for WsCameraServer {
    type Result = ();

    fn handle(&mut self, msg: Unsubscribe, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Removing subscriber {:?}", &msg.subject);
        self.remove_subscriber(&msg.subject, &msg.client);
    }
}

impl Handler<CameraFrame> for WsCameraServer {
    type Result = ();

    fn handle(&mut self, msg: CameraFrame, ctx: &mut Self::Context) -> Self::Result {
        self.send_frame(&msg, ctx);
    }
}

impl SystemService for WsCameraServer {}
impl Supervised for WsCameraServer {}
