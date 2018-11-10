use actix::prelude::*;
use actix_web::{fs, ws, App, Error, HttpRequest, HttpResponse};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

use app::AppState;
use ws_camera_server::{CameraFrame, Subscribe, Unsubscribe};

#[derive(Default)]
pub struct WsSession {}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self, AppState>;

    fn started(&mut self, ctx: &mut Self::Context) {
        debug!("Starting websocket!");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        debug!("Stopping websocket!");
    }
}

impl Handler<CameraFrame> for WsSession {
    type Result = ();
    fn handle(&mut self, msg: CameraFrame, ctx: &mut Self::Context) -> Self::Result {
        let mut buf = Vec::new();
        msg.serialize(&mut Serializer::new(&mut buf)).unwrap();
        ctx.binary(buf);
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for WsSession {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        debug!("WEBSOCKET MESSAGE: {:?}", msg);
        match msg {
            ws::Message::Text(text) => {
                debug!("Got text {}: ", text);
            }
            ws::Message::Close(_) => {
                ctx.stop();
            }
            _ => {}
        }
    }
}
