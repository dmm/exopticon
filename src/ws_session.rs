use actix::prelude::*;
use actix_web::ws;
use rmp::encode::{write_map_len, write_str, ValueWriteError};
use rmp::Marker;
use rmp_serde::encode::VariantWriter;
use rmp_serde::Serializer;
use serde::Serialize;
use serde_bytes::ByteBuf;
use serde_json;
use std::io::Write;

use crate::app::AppState;
use crate::ws_camera_server::{CameraFrame, FrameResolution, Subscribe, Unsubscribe, WsCameraServer};

#[derive(Serialize, Deserialize)]
struct WsCommand {
    command: String,
    resolution: FrameResolution,
    #[serde(rename = "cameraIds")]
    camera_ids: Vec<i32>,
}

#[derive(Serialize)]
struct RawCameraFrame {
    pub camera_id: i32,
    pub resolution: FrameResolution,
    pub jpeg: ByteBuf,
}

#[derive(Default)]
pub struct WsSession {
    pub ready: bool,
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
        if !self.ready {
            debug!("Dropping frame!");
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
        let mut se = Serializer::with(Vec::new(), StructMapWriter);
        frame.serialize(&mut se).unwrap();
        ctx.binary(se.into_inner());

        // spawn drain future
        ctx.spawn(fut);
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for WsSession {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Text(text) => {
                debug!("Got text {}: ", text);
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
