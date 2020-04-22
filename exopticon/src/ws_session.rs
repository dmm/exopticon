// clippy doesn't like the base64_serde_type macro
#![allow(clippy::empty_enum)]

use actix::{
    Actor, ActorContext, ActorFuture, AsyncContext, Handler, StreamHandler, SystemService,
    WrapFuture,
};
use actix_web_actors::ws;
use futures::future;
use rmp_serde::Serializer;
use serde;
use serde::Serialize;
use serde_json;

use crate::db_registry;
use crate::models::{FetchObservationsByVideoUnit, FetchVideoUnit, Observation};
use crate::playback_actor::PlaybackFrame;
use crate::playback_supervisor::{PlaybackSupervisor, StartPlayback, StopPlayback};
use crate::struct_map_writer::StructMapWriter;
use crate::ws_camera_server::{
    CameraFrame, FrameResolution, FrameSource, Subscribe, SubscriptionSubject, Unsubscribe,
    WsCameraServer,
};

use base64::STANDARD_NO_PAD;

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
    /// Start playback request
    StartPlayback {
        /// playback id, currently supplied by client
        id: u64,
        /// id of video unit id to play
        video_unit_id: i32,
        /// initial offset to begin playback
        offset: i32,
    },
    /// Stop playback request
    StopPlayback {
        /// playback id, supplied by client
        id: u64,
    },
}

/// A frame of video from a camera stream. This struct is used to
/// deliver a frame to the browser over the websocket connection.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawCameraFrame {
    /// id of camera that produced frame
    pub camera_id: i32,
    /// resolution of frame
    pub resolution: FrameResolution,
    /// original width of frame
    pub unscaled_width: i32,
    /// original height of frame
    pub unscaled_height: i32,
    /// source of frame
    pub source: FrameSource,
    /// id of video unit
    pub video_unit_id: i32,
    /// offset from beginning of video unit
    pub offset: i64,
    /// observations associated with frame
    pub observations: Vec<Observation>,
    /// jpeg image data
    #[serde(with = "Base64Standard")]
    pub jpeg: Vec<u8>,
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
    /// Returns new `WsSession` struct initialized with default values
    /// and specified serialization type
    ///
    /// # Arguments
    ///
    /// * `serialization` - Type of serialization to use `MsgPack` or `Json`
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
        if self.live_frames == 0 {
            error!("live frame count should never be zero when acking!");
            return;
        }
        self.live_frames = std::cmp::max(self.live_frames, 1);
        self.live_frames -= 1;

        if self.live_frames < self.window_size && self.window_size < 10 {
            self.window_size += 1;
        }
    }

    /// Examines the current window state and adjusts window size.
    fn adjust_window(&mut self) {
        if self.live_frames == self.window_size {
            self.window_size -= 1;
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

    /// handle frame, potentially sending it to client
    fn handle_frame(
        &mut self,
        msg: CameraFrame,
        observations: Vec<Observation>,
        ctx: &mut <Self as Actor>::Context,
    ) {
        if !self.ready_to_send() {
            self.adjust_window();
            return;
        }
        // wait for buffer to drain before sending another
        //        self.ready = false;
        //        let fut = ctx.drain().map(|_status, actor, _ctx| {
        //            actor.ready = true;
        //        });

        let frame = RawCameraFrame {
            camera_id: msg.camera_id,
            jpeg: msg.jpeg,
            resolution: msg.resolution,
            unscaled_width: msg.unscaled_width,
            unscaled_height: msg.unscaled_height,
            source: msg.source,
            video_unit_id: msg.video_unit_id,
            offset: msg.offset,
            observations,
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
        //        ctx.spawn(fut);
    }
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

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
        self.handle_frame(msg, Vec::new(), ctx);
    }
}

impl Handler<PlaybackFrame> for WsSession {
    type Result = ();

    fn handle(&mut self, msg: PlaybackFrame, ctx: &mut Self::Context) -> Self::Result {
        self.handle_frame(msg.frame, msg.observations, ctx);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                error!("WsSession Stream error! {}", e);
                return;
            }
        };
        match msg {
            ws::Message::Text(text) => {
                let cmd: Result<WsCommand, serde_json::Error> = serde_json::from_str(&text);
                match cmd {
                    Ok(WsCommand::Subscribe(subject)) => {
                        WsCameraServer::from_registry().do_send(Subscribe {
                            subject,
                            client: ctx.address().recipient(),
                        });
                    }

                    Ok(WsCommand::Unsubscribe(subject)) => {
                        WsCameraServer::from_registry().do_send(Unsubscribe {
                            subject,
                            client: ctx.address().recipient(),
                        });
                    }

                    Ok(WsCommand::StartPlayback {
                        id,
                        video_unit_id,
                        offset,
                    }) => {
                        debug!("StartPlayback: {} {}, {}", id, video_unit_id, offset);
                        // Right now we are trusting that the client
                        // is sending a random id.  Eventually we
                        // should stop doing that and generate an id
                        // and map between the client provided and
                        // generated ids.

                        // Ask playback supervisor to begin playback
                        let fetch_observations = db_registry::get_db()
                            .send(FetchObservationsByVideoUnit { video_unit_id });

                        let fetch_video_unit =
                            db_registry::get_db().send(FetchVideoUnit { id: video_unit_id });

                        let create_actor = future::join(fetch_video_unit, fetch_observations)
                            .into_actor(self)
                            .map(move |res, _act, ctx| {
                                if let (Ok(Ok(video_unit)), Ok(Ok(observations))) = res {
                                    info!("Fetched {} observations.", observations.len());
                                    if let Some(video_file) = video_unit.files.first() {
                                        PlaybackSupervisor::from_registry().do_send(
                                            StartPlayback {
                                                id,
                                                video_unit_id,
                                                offset,
                                                video_filename: video_file.filename.clone(),
                                                observations,
                                                address: ctx.address(),
                                            },
                                        );
                                    }
                                }
                            });
                        ctx.spawn(create_actor);

                        // subscribe to playback subject
                    }

                    Ok(WsCommand::StopPlayback { id }) => {
                        debug!("Got unsubscribe message for playback id: {}", id);
                        PlaybackSupervisor::from_registry().do_send(StopPlayback { id });
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
            ws::Message::Nop
            | ws::Message::Continuation(_)
            | ws::Message::Binary(_)
            | ws::Message::Ping(_)
            | ws::Message::Pong(_) => {}
        }
    }
}
