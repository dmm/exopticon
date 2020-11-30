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

use std::time::Duration;

use actix::prelude::*;
use actix_interop::{critical_section, with_ctx, FutureInterop};
use log::error;
use mqtt_async_client::client::{Client, Publish as PublishOpts, QoS};

/// Actor message requesting to send an mqtt alert
pub struct SendMqttMessage {
    /// message topic
    pub topic: String,
    /// message payload, probably json
    pub payload: String,
}

impl Message for SendMqttMessage {
    type Result = ();
}

/// Actor implementing alert signalling over mqtt
pub struct MqttActor {
    /// hostname of mqtt server
    pub host: String,
    /// port of mqtt server
    pub port: u16,
    /// username or None to authenticate with mqtt server
    pub username: Option<String>,
    /// password or None to authenticate with mqtt server
    pub password: Option<String>,
    /// mqtt client
    pub client: Option<Client>,
}

impl Actor for MqttActor {
    type Context = Context<Self>;
}

impl MqttActor {
    /// Constructor for `MqttActor`
    pub fn new(
        host: String,
        port: u16,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        let mut b = Client::builder();

        b.set_host(host.clone());
        b.set_port(port);
        b.set_username(username.clone());
        if let Some(pass) = password.clone() {
            b.set_password(Some(pass.into_bytes()));
        }
        b.set_connect_retry_delay(Duration::from_secs(1));
        b.set_automatic_connect(true);

        Self {
            host,
            port,
            username,
            password,
            client: Some(b.build().expect("Invalid mqtt client parameters")),
        }
    }
}

impl Handler<SendMqttMessage> for MqttActor {
    type Result = ();

    fn handle(&mut self, msg: SendMqttMessage, ctx: &mut Context<Self>) -> Self::Result {
        let fut = async move {
            critical_section::<Self, _>(async move {
                let mut client = with_ctx(|actor: &mut Self, _| actor.client.take())
                    .expect("client not present in critical section.");

                match client.connect().await {
                    Ok(_) => (),
                    Err(err) => error!("Error connecting to mqtt server: {}", err),
                }

                let mut p = PublishOpts::new(msg.topic.clone(), msg.payload.as_bytes().to_vec());
                p.set_qos(QoS::AtMostOnce);
                p.set_retain(false);

                match client.publish(&p).await {
                    Ok(_) => {}
                    Err(err) => {
                        error!("Error sending mqtt message! {}", err);
                    }
                }

                // put the client back
                with_ctx(|actor: &mut Self, _| {
                    actor.client = Some(client);
                });
            })
            .await;
        }
        .interop_actor_boxed(self);
        ctx.spawn(fut);
    }
}
