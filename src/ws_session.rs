// clippy doesn't like the base64_serde_type macro
#![allow(clippy::empty_enum)]

use actix::{
    Actor, ActorContext, ActorFuture, AsyncContext, Handler, StreamHandler, SystemService,
};
use actix_web::ws;
use rmp_serde::Serializer;
use serde;
use serde::Serialize;
use serde_json;

use base64::STANDARD_NO_PAD;

use crate::app::RouteState;
use crate::struct_map_writer::StructMapWriter;
use crate::ws_camera_server::{
    CameraFrame, FrameResolution, FrameSource, Subscribe, SubscriptionSubject, Unsubscribe,
    WsCameraServer,
};

/// Represents different serializations available for communicating
/// over websockets.
pub enum WsSerialization {
    /// MessagePack Serialization
    MsgPack,

    /// Json Serialization
    Json,
}

base64_serde_type!(Base64Standard, STANDARD_NO_PAD);

/// A command from the client, transported over the websocket
/// connection
#[derive(Serialize, Deserialize)]
pub enum WsCommand {
    /// Subscription request
    Subscribe(SubscriptionSubject),
    /// Unsubscription request
    Unsubscribe(SubscriptionSubject),
    /// frame ack response
    Ack,
}

/// A frame of video from a camera stream. This struct is used to
/// deliver a frame to the browser over the websocket connection.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawCameraFrame {
    /// id of camera that produced frame
    pub camera_id: i32,
    /// jpeg image data
    #[serde(with = "Base64Standard")]
    pub jpeg: Vec<u8>,
    /// resolution of frame
    pub resolution: FrameResolution,
    /// source of frame
    pub source: FrameSource,
    /// id of video unit
    pub video_unit_id: i32,
    /// offset from beginning of video unit
    pub offset: i64,
}

/// An actor representing a websocket connection
pub struct WsSession {
    /// True when the websocket is ready to send
    pub ready: bool,

    /// Serialization to use for this socket
    pub serialization: WsSerialization,

    /// Maximum number of frames to have in flight
    pub window_size: u32,

    /// Current number of frames in flight
    pub live_frames: u32,
}
impl WsSession {
    /// Returns new WsSession struct initialized with default values
    /// and specified serialization type
    ///
    /// # Arguments
    ///
    /// * `serialization` - Type of serialization to use MsgPack or Json
    ///
    pub const fn new(serialization: WsSerialization) -> Self {
        Self {
            ready: true,
            serialization,
            window_size: 1,
            live_frames: 0,
        }
    }

    /// Returns true if session can send another frame.
    fn ready_to_send(&self) -> bool {
        self.live_frames < self.window_size && self.ready
    }

    /// Modifies send window, intended to be called when acking a
    /// frame.
    fn ack(&mut self) {
        self.live_frames -= 1;

        self.live_frames = std::cmp::max(self.live_frames, 0);

        if self.live_frames < self.window_size && self.window_size < 10 {
            self.window_size += 1;
        }
    }

    /// Examines the current window state and adjusts window size.
    fn adjust_window(&mut self) {
        if self.live_frames == self.window_size {
            self.window_size /= 2;
        }

        if self.live_frames < self.window_size {
            self.window_size += 1;
        }

        if self.window_size == 0 {
            self.window_size = 1;
        }

        if self.window_size > 10 {
            self.window_size = 10;
        }
    }
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self, RouteState>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("Starting websocket!");
        self.ready = true;
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        debug!("Stopping websocket!");
    }
}

impl Handler<CameraFrame> for WsSession {
    type Result = ();
    fn handle(&mut self, msg: CameraFrame, ctx: &mut Self::Context) -> Self::Result {
        if !self.ready_to_send() {
            self.adjust_window();
            return;
        }
        // wait for buffer to drain before sending another
        self.ready = false;
        let fut = ctx.drain().map(|_status, actor, _ctx| {
            actor.ready = true;
        });

        let frame = RawCameraFrame {
            camera_id: msg.camera_id,
            jpeg: msg.jpeg,
            resolution: msg.resolution,
            source: msg.source,
            video_unit_id: msg.video_unit_id,
            offset: msg.offset,
        };
        match &self.serialization {
            WsSerialization::MsgPack => {
                let mut se = Serializer::with(Vec::new(), StructMapWriter);
                frame
                    .serialize(&mut se)
                    .expect("Messagepack camera frame serialization failed!");
                ctx.binary(se.into_inner());
            }
            WsSerialization::Json => ctx.text(
                serde_json::to_string(&frame).expect("Json camera frame serialization failed!"),
            ),
        };

        // add live frame
        self.live_frames += 1;

        // spawn drain future
        ctx.spawn(fut);
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for WsSession {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Text(text) => {
                let cmd: Result<WsCommand, serde_json::Error> = serde_json::from_str(&text);
                match cmd {
                    Ok(WsCommand::Subscribe(SubscriptionSubject::Camera(id, resolution))) => {
                        WsCameraServer::from_registry().do_send(Subscribe {
                            subject: SubscriptionSubject::Camera(id, resolution),
                            client: ctx.address().recipient(),
                        });
                    }
                    Ok(WsCommand::Unsubscribe(SubscriptionSubject::Camera(id, resolution))) => {
                        WsCameraServer::from_registry().do_send(Unsubscribe {
                            subject: SubscriptionSubject::Camera(id, resolution),
                            client: ctx.address().recipient(),
                        });
                    }
                    Ok(WsCommand::Subscribe(SubscriptionSubject::AnalysisEngine(id))) => {
                        WsCameraServer::from_registry().do_send(Subscribe {
                            subject: SubscriptionSubject::AnalysisEngine(id),
                            client: ctx.address().recipient(),
                        });
                    }
                    Ok(WsCommand::Unsubscribe(SubscriptionSubject::AnalysisEngine(id))) => {
                        WsCameraServer::from_registry().do_send(Unsubscribe {
                            subject: SubscriptionSubject::AnalysisEngine(id),
                            client: ctx.address().recipient(),
                        });
                    }
                    Ok(WsCommand::Ack) => {
                        self.ack();
                    }
                    Err(e) => {
                        error!("Error deserializing message {}. Ignoring...", e);
                    }
                }
            }
            ws::Message::Close(_) => {
                debug!("Stopping WsSession.");
                ctx.stop();
            }
            ws::Message::Binary(_) | ws::Message::Ping(_) | ws::Message::Pong(_) => {}
        }
    }
}
