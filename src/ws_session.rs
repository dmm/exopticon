use std::io::Write;

use crate::actix::prelude::*;
use actix_web::ws;
use rmp::encode::{write_map_len, write_str, ValueWriteError};
use rmp::Marker;
use rmp_serde::encode::VariantWriter;
use rmp_serde::Serializer;
use serde;
use serde::Serialize;
use serde_bytes::ByteBuf;
use serde_json;

use base64::STANDARD_NO_PAD;

use crate::app::AppState;
use crate::ws_camera_server::{
    CameraFrame, FrameResolution, Subscribe, Unsubscribe, WsCameraServer,
};

pub enum WsSerialization {
    MsgPack,
    Json,
}

base64_serde_type!(Base64Standard, STANDARD_NO_PAD);

/// A command from the client, transported over the websocket
/// connection
#[derive(Serialize, Deserialize)]
struct WsCommand {
    /// command type
    command: String,
    /// selected frame resolution
    resolution: FrameResolution,
    /// affected camera ids
    #[serde(rename = "cameraIds")]
    camera_ids: Vec<i32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RawCameraFrame {
    pub camera_id: i32,
    pub resolution: FrameResolution,
    #[serde(with = "Base64Standard")]
    pub jpeg: ByteBuf,
}

/// An actor representing a websocket connection
pub struct WsSession {
    /// True when the websocket is ready to send
    pub ready: bool,
    pub serialization: WsSerialization,
    pub window_size: u32,
    pub live_frames: u32,
}
impl WsSession {
    pub fn new(serialization: WsSerialization) -> WsSession {
        WsSession {
            ready: true,
            serialization: serialization,
            window_size: 1,
            live_frames: 0,
        }
    }

    fn ready_to_send(&self) -> bool {
        return self.live_frames < self.window_size && self.ready;
    }

    fn ack(&mut self) {
        self.live_frames -= 1;

        if self.live_frames < self.window_size && self.window_size < 10 {
            self.window_size += 1;
        }
    }

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
    type Context = ws::WebsocketContext<Self, AppState>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("Starting websocket!");
        self.ready = true;
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        debug!("Stopping websocket!");
    }
}

/// An empty struct to implement VariantWriter so we can serialize frames as structs/maps.
struct StructMapWriter;

impl VariantWriter for StructMapWriter {
    fn write_struct_len<W: Write>(&self, wr: &mut W, len: u32) -> Result<Marker, ValueWriteError> {
        write_map_len(wr, len)
    }

    fn write_field_name<W: Write>(&self, wr: &mut W, key: &str) -> Result<(), ValueWriteError> {
        write_str(wr, key)
    }
}

impl Handler<CameraFrame> for WsSession {
    type Result = ();
    fn handle(&mut self, msg: CameraFrame, ctx: &mut Self::Context) -> Self::Result {
        if !self.ready_to_send() {
            debug!("Dropping frame!");
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
            jpeg: ByteBuf::from(msg.jpeg),
            resolution: msg.resolution,
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
                    Ok(c) => match c.command.as_ref() {
                        "subscribe" => {
                            for id in c.camera_ids {
                                WsCameraServer::from_registry().do_send(Subscribe {
                                    camera_id: id,
                                    client: ctx.address().recipient(),
                                    resolution: c.resolution.clone(),
                                });
                            }
                        }
                        "unsubscribe" => {
                            for id in c.camera_ids {
                                WsCameraServer::from_registry().do_send(Unsubscribe {
                                    camera_id: id,
                                    client: ctx.address().recipient(),
                                    resolution: c.resolution.clone(),
                                });
                            }
                        }
                        "ack" => {
                            debug!(
                                "window_size: {}, live_size: {}",
                                self.window_size, self.live_frames
                            );
                            self.ack();
                        }
                        _ => {}
                    },
                    Err(e) => {
                        error!("Error deserializing message {}. Ignoring...", e);
                    }
                }
            }
            ws::Message::Close(_) => {
                debug!("Stopping WsSession.");
                ctx.stop();
            }
            _ => {}
        }
    }
}
