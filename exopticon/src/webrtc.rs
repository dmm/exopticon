/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2023 David Matthew Mattli <dmm@mattli.us>
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

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};

use axum::extract::ws::{self, WebSocket};
use futures::stream::{FuturesUnordered, Stream};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use str0m::{
    change::SdpOffer, channel::ChannelId, media::Mid, net::Protocol, IceConnectionState, Input, Rtc,
};
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

use crate::super_capture_actor::VideoPacket;

pub enum ClientEvent {
    Noop,
    Timeout(Instant),
    Transmit(str0m::net::Transmit),
}

impl ClientEvent {
    pub fn as_timeout(&self) -> Option<Instant> {
        if let Self::Timeout(inst) = self {
            Some(*inst)
        } else {
            None
        }
    }
}

#[derive(PartialEq)]
pub enum MidStatus {
    //    NotYetValid,
    Valid,
}

pub struct ClientTrack {
    mid: Mid,
    mid_status: MidStatus,
}
/// Messages from client
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientMessage {
    #[serde(rename_all = "camelCase")]
    SubscriptionUpdate { subscribed_camera_ids: Vec<i32> },
    #[serde(rename_all = "camelCase")]
    NegotiationRequest { offer: String },
}

/// Messages from server
#[derive(Serialize)]
#[serde(tag = "kind")]
#[serde(rename_all = "camelCase")]
pub enum ServerMessage {
    /// Maps camera id to Mid
    #[serde(rename_all = "camelCase")]
    StreamMappings(HashMap<i32, String>),
    #[serde(rename_all = "camelCase")]
    NegotiationAnswer { answer: String },
}

pub struct Client {
    socket_sink: SplitSink<WebSocket, axum::extract::ws::Message>,
    rtc: Rtc,
    cid: Option<ChannelId>,
    /// mapping between camera_ids and `ClientTrack` struct
    tracks: HashMap<i32, ClientTrack>,
    subscriptions: HashMap<i32, bool>,
}

impl Client {
    pub fn new(rtc: Rtc, socket_sink: SplitSink<WebSocket, axum::extract::ws::Message>) -> Self {
        Self {
            socket_sink,
            rtc,
            cid: None,
            tracks: HashMap::new(),
            subscriptions: HashMap::new(),
        }
    }

    fn get_stream_mappings(&self) -> HashMap<i32, String> {
        let mut mappings = HashMap::new();
        for (camera_id, client_track) in &self.tracks {
            if client_track.mid_status == MidStatus::Valid {
                mappings.insert(camera_id.clone(), client_track.mid.to_string());
            }
        }
        mappings
    }

    pub fn accepts(&self, input: &Input<'_>) -> bool {
        self.rtc.accepts(input)
    }

    pub fn handle_input(&mut self, input: Input<'_>) {
        if !self.rtc.is_alive() {
            debug!("RTC is dead, not handling input!");
            return;
        }

        if let Err(e) = self.rtc.handle_input(input) {
            warn!("Client ({}) disconnected: {:?}", 3, e);
            self.rtc.disconnect();
        }
    }

    async fn send_server_message(&mut self, message: &ServerMessage) {
        let serialized_message = match serde_json::to_string(&message) {
            Ok(m) => m,
            Err(err) => {
                error!("error serializing client message...{:?}", err);
                return;
            }
        };
        if let Err(err) = self
            .socket_sink
            .send(ws::Message::Text(serialized_message))
            .await
        {
            error!("Error sending websocket: {}", err);
        }
    }

    pub async fn handle_message(&mut self, message_string: &str) {
        // Parse json into `ClientMessage'
        debug!("got client message: {}", message_string);
        let message: ClientMessage = match serde_json::from_str(message_string) {
            Ok(msg) => msg,
            Err(err) => {
                error!("Error parsing client message: {}", err);
                return;
            }
        };

        match message {
            ClientMessage::SubscriptionUpdate {
                subscribed_camera_ids,
            } => {
                self.subscriptions.clear();
                for id in subscribed_camera_ids {
                    self.subscriptions.insert(id, true);
                }

                // respond with current stream mappings
                let mappings: ServerMessage =
                    ServerMessage::StreamMappings(self.get_stream_mappings());
                self.send_server_message(&mappings).await;
            }
            ClientMessage::NegotiationRequest { offer } => {
                let spd_offer =
                    SdpOffer::from_sdp_string(&offer).expect("Failed to deserialized sdp offer");
                let answer = match self.rtc.sdp_api().accept_offer(spd_offer) {
                    Ok(answer) => answer,
                    Err(err) => {
                        error!("accept_offer failed: {:?}", err);
                        return;
                    }
                };
                self.send_server_message(&ServerMessage::NegotiationAnswer {
                    answer: answer.to_sdp_string(),
                })
                .await;
            }
        }
    }

    pub fn poll_rtc(&mut self) -> ClientEvent {
        let out = match self.rtc.poll_output() {
            Ok(o) => o,
            Err(err) => {
                warn!("Poll output failed: {}", err);
                self.rtc.disconnect();
                return ClientEvent::Noop;
            }
        };

        match out {
            str0m::Output::Timeout(inst) => ClientEvent::Timeout(inst),
            str0m::Output::Transmit(t) => ClientEvent::Transmit(t),
            str0m::Output::Event(e) => {
                match e {
                    str0m::Event::Connected => info!("Connected!"),
                    str0m::Event::IceConnectionStateChange(v) => {
                        info!("IceConnectionStateChange!");
                        if v == IceConnectionState::Disconnected {
                            self.rtc.disconnect();
                        }
                    }
                    str0m::Event::MediaAdded(added) => {
                        info!("media added!");
                        if let Some((_cid, track)) = self
                            .tracks
                            .iter_mut()
                            .find(|(_cid, track)| track.mid == added.mid)
                        {
                            track.mid_status = MidStatus::Valid;
                        }
                    }
                    str0m::Event::MediaData(_) => info!("MediaData"),
                    str0m::Event::MediaChanged(_) => info!("MediaChanged!"),
                    str0m::Event::KeyframeRequest(_) => info!("Keyframe request!"),
                    str0m::Event::ChannelOpen(cid, _) => {
                        info!("Channel Open!");
                        self.cid = Some(cid);
                    }
                    str0m::Event::ChannelData(d) => {
                        info!("Channel Data");
                        if !d.binary {
                            let message_string = String::from_utf8(d.data).unwrap();
                            info!("Got channel message: {}", &message_string);
                        }
                    }
                    str0m::Event::ChannelClose(_) => info!("Channel close!"),
                    str0m::Event::PeerStats(_) => info!("PeerStats"),
                    str0m::Event::MediaIngressStats(_) => info!("MediaIngressStats"),
                    str0m::Event::MediaEgressStats(_) => info!("MediaEgressStats"),
                    str0m::Event::EgressBitrateEstimate(_) => info!("EgressBitrateEstimate"),
                    _ => info!("unknown event!"),
                }
                ClientEvent::Noop
            }
        }
    }
}

pub struct Server {
    packet_receiver: tokio::sync::broadcast::Receiver<VideoPacket>,
    ws_receiver: Receiver<WebSocket>,
    websocket_streams:
        FuturesUnordered<Box<dyn Stream<Item = (Uuid, Result<ws::Message, axum::Error>)> + Send>>,
    socket: tokio::net::UdpSocket,
    candidate_ips: Vec<IpAddr>,
    clients: HashMap<Uuid, Client>,
}

impl Server {
    pub fn new(
        packet_receiver: tokio::sync::broadcast::Receiver<VideoPacket>,
        ws_receiver: Receiver<WebSocket>,
        socket: tokio::net::UdpSocket,
        candidate_ips: Vec<IpAddr>,
    ) -> Self {
        Self {
            packet_receiver,
            ws_receiver,
            websocket_streams: FuturesUnordered::new(),
            socket,
            candidate_ips,
            clients: HashMap::new(),
        }
    }

    pub async fn spawn_new_client(&mut self, ws: WebSocket) -> Result<(), ()> {
        debug!("Spawning new RTC client");

        let (sink, source) = ws.split();
        let id = Uuid::new_v4();
        let rtc = Rtc::builder().build();
        let client = Client::new(rtc, sink);

        self.clients.insert(id, client);
        let id_stream = source.map(move |msg| (id, msg));
        self.websocket_streams.push(Box::new(id_stream));

        Ok(())
    }

    pub async fn handle_control_message(
        &mut self,
        (id, msg): (Uuid, Result<ws::Message, axum::Error>),
    ) {
        let client = self.clients.get(&id);
        match msg {
            Ok(msg) => {}
            Err(_) => todo!(),
        }
    }

    /// handle video packet from capture workers
    async fn handle_packet(&mut self, _pkt: VideoPacket) {
        //        for client in &self.clients {}
    }

    /// Handle udp packet received by server
    pub async fn handle_udp(&mut self, len: usize, addr: SocketAddr, buf: &mut Vec<u8>) {
        buf.truncate(len);
        // debug!(
        //     "Handling UDP packet! {}, len: {}, local addr: {}ma",
        //     addr,
        //     buf.len(),
        //     self.socket.local_addr().unwrap()
        // );
        // str0m doesn't like it when we give it a destination address
        // not in the webrc ips. This can happend when we are behind
        // NAT. Replace the destination ip with the first one in the
        // candidate ip set.
        let destination_port = self.socket.local_addr().unwrap().port();
        let destination_ip = self.candidate_ips.first().unwrap();

        let socket_addr = SocketAddr::new(*destination_ip, destination_port);
        match str0m::net::Receive::new(Protocol::Udp, addr, socket_addr, buf) {
            Ok(receive_body) => {
                let input = Input::Receive(Instant::now(), receive_body);

                if let Some(client) = self.clients.values_mut().find(|c| c.accepts(&input)) {
                    client.handle_input(input);
                } else {
                    // Invalid packet?
                    debug!("No client accepts UDP packet: {:?}", input);
                }
            }
            Err(e) => {
                error!("Error parsing packet: {:?}", e);
            }
        }
    }

    pub async fn process_event(&mut self, event: &ClientEvent) {
        match event {
            ClientEvent::Noop => {}
            ClientEvent::Timeout(_) => {}
            ClientEvent::Transmit(t) => {
                if let Err(e) = self.socket.send_to(&t.contents, t.destination).await {
                    error!("Error sending udp data! {}", e);
                }
            }
        };
    }

    pub async fn process_client_events(&mut self) -> Duration {
        loop {
            // Poll all clients for events
            let events: Vec<ClientEvent> =
                self.clients.values_mut().map(|c| c.poll_rtc()).collect();
            let timeouts: Vec<_> = events.iter().filter_map(|e| e.as_timeout()).collect();
            // handle events until they are all timeouts
            if events.len() == timeouts.len() {
                break;
            }
            for e in events {
                self.process_event(&e).await;
            }
        }

        Duration::from_millis(100)
    }

    pub async fn run(mut self) {
        let mut buf = vec![0; 2000];
        let mut timeout = Duration::from_millis(100);
        loop {
            buf.resize(2000, 0);
            tokio::select! {
                Ok(pkt) = self.packet_receiver.recv() => self.handle_packet(pkt).await,

                Some(ws) = self.ws_receiver.recv() => self.spawn_new_client(ws).await.unwrap(),
                Ok((len, addr)) = self.socket.recv_from(&mut buf) => self.handle_udp(len, addr, &mut buf).await,
                control_message = self.websocket_streams.select_next_some() => self.handle_control_message(control_message).await,
                _ = tokio::time::sleep(timeout) => (),
            }

            timeout = self.process_client_events().await;

            // clean out disconnected clients
            self.clients.retain(|_id, c| c.rtc.is_alive());
        }
    }
}
