/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020-2022 David Matthew Mattli <dmm@mattli.us>
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
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::extract::ws::{self, WebSocket};
use metrics::gauge;
use str0m::{
    Candidate, Input, Rtc,
    change::SdpOffer,
    format::PayloadParams,
    media::{Frequency, MediaTime, Mid},
    net::Protocol,
};
use tokio::{
    net::{UdpSocket, lookup_host},
    sync::mpsc,
};
use uuid::Uuid;

use crate::{capture_actor::VideoPacket, video_router::VideoRouter};

pub type ClientId = Uuid;

/// Messages from client
#[derive(Deserialize)]
#[serde(tag = "kind")]
#[serde(rename_all = "camelCase")]
pub enum ClientMessage {
    #[serde(rename_all = "camelCase")]
    SubscriptionUpdate { _subscribed_camera_ids: Vec<Uuid> },
    #[serde(rename_all = "camelCase")]
    NegotiationRequest { offer: String },
    /// Maps camera id to Mid
    #[serde(rename_all = "camelCase")]
    StreamMapping { mappings: HashMap<String, Uuid> },
}

/// Messages from server
#[derive(Serialize)]
#[serde(tag = "kind")]
#[serde(rename_all = "camelCase")]
pub enum ServerMessage {
    #[serde(rename_all = "camelCase")]
    NegotiationAnswer { answer: String },
}

pub struct Client {
    id: ClientId,
    websocket: WebSocket,
    /// used only to send messages
    udp_socket: Arc<UdpSocket>,
    udp_receiver: tokio::sync::broadcast::Receiver<(usize, SocketAddr, Vec<u8>)>,
    video_receiver: mpsc::Receiver<VideoPacket>,
    video_sender: mpsc::Sender<VideoPacket>,
    video_router: Arc<VideoRouter>,
    candidate_ips: Vec<String>,
    candidate_socketaddrs: Vec<SocketAddr>,
    rtc: Rtc,
    camera_mapping: HashMap<Uuid, Mid>,
}

impl Client {
    pub fn new(
        websocket: WebSocket,
        udp_receiver: tokio::sync::broadcast::Receiver<(usize, SocketAddr, Vec<u8>)>,
        video_receiver: mpsc::Receiver<VideoPacket>,
        video_sender: mpsc::Sender<VideoPacket>,
        video_router: Arc<VideoRouter>,
        udp_socket: Arc<UdpSocket>,
        candidate_ips: Vec<String>,
    ) -> Self {
        let rtc = Rtc::builder()
            .set_send_buffer_video(100_000)
            .set_reordering_size_video(500)
            .clear_codecs()
            .enable_h264(true)
            .enable_experimental_loss_based_bwe(true)
            .build();

        Self {
            id: Uuid::new_v4(),
            websocket,
            udp_socket,
            udp_receiver,
            video_receiver,
            video_sender,
            video_router,
            candidate_ips,
            candidate_socketaddrs: Vec::new(),
            rtc,
            camera_mapping: HashMap::new(),
        }
    }

    /// Parse list of candidates, and populate `candidate_socketaddrs`
    async fn parse_candidates(&mut self) {
        let mut addrs = Vec::new();

        for c in &self.candidate_ips {
            // First try to parse as ip address:port
            if let Ok(ip) = c.parse::<SocketAddr>() {
                addrs.push(ip);
                continue;
            }

            // If that fails, perform dns lookup
            if let Ok(ips) = lookup_host(c).await {
                for ip in ips {
                    if let SocketAddr::V4(_) = ip {
                        addrs.push(ip);
                    }
                }
            }
        }

        self.candidate_socketaddrs = addrs;
    }

    async fn handle_websocket(&mut self, msg: Result<ws::Message, axum::Error>) -> Result<(), ()> {
        let msg = match msg {
            Ok(msg) => msg,
            Err(err) => {
                info!("got websocket error! {}", err);
                return Err(());
            }
        };

        let txt_msg = match msg {
            ws::Message::Text(msg) => msg,
            ws::Message::Binary(_) => {
                info!("got invalid binary message");
                return Err(());
            }
            ws::Message::Ping(_) | ws::Message::Pong(_) => return Ok(()),
            ws::Message::Close(_) => return Err(()),
        };

        let message: ClientMessage = match serde_json::from_str(&txt_msg) {
            Ok(m) => m,
            Err(err) => {
                info!("error parsing client message {}: {}", &txt_msg, err);
                return Err(());
            }
        };

        match message {
            ClientMessage::SubscriptionUpdate {
                _subscribed_camera_ids: _,
            } => {
                // NOT USED
                // TODO REMOVE ME
                error!("ClientMessage::SubscriptionUpdate, we shouldn't get this...");
            }
            ClientMessage::NegotiationRequest { offer } => {
                let spd_offer =
                    SdpOffer::from_sdp_string(&offer).expect("Failed to deserialized sdp offer");

                // Add candidate ips
                self.parse_candidates().await;
                for ip in &self.candidate_socketaddrs {
                    let candidate = Candidate::host(*ip, Protocol::Udp).unwrap();
                    self.rtc.add_local_candidate(candidate);
                }

                // Create an SDP Answer.
                //                debug!("OFFER: {}", spd_offer.to_sdp_string());
                let answer = match self.rtc.sdp_api().accept_offer(spd_offer) {
                    Ok(answer) => answer,
                    Err(err) => {
                        error!("accept_offer failed: {:?}", err);
                        return Err(());
                    }
                };
                //                debug!("ANSWER: {}", answer.to_sdp_string());
                let answer_text = serde_json::to_string(&ServerMessage::NegotiationAnswer {
                    answer: answer.to_sdp_string(),
                })
                .unwrap();

                self.websocket
                    .send(ws::Message::Text(answer_text))
                    .await
                    .unwrap();
            }
            ClientMessage::StreamMapping { mappings } => {
                self.camera_mapping.clear();
                for (mid_string, camera_id) in &mappings {
                    let m: Mid = Mid::from(mid_string.as_str());
                    self.camera_mapping.insert(*camera_id, m);
                }

                error!("SENDING UPDATE SUBSCRIPTION!");
                self.video_router
                    .update_subscriptions(
                        self.id,
                        mappings.into_values().collect(),
                        self.video_sender.clone(),
                    )
                    .await;
                error!("DOOOOOONE UPDATE SUBSCRIPTION!");
            }
        }
        Ok(())
    }

    fn handle_udp(&mut self, (len, addr, mut buf): (usize, SocketAddr, Vec<u8>)) {
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
        let Some(destination_ip) = self.candidate_socketaddrs.first() else {
            error!("NO candidate ips");
            return;
        };

        match str0m::net::Receive::new(Protocol::Udp, addr, *destination_ip, &buf) {
            Ok(receive_body) => {
                let input = Input::Receive(Instant::now(), receive_body);

                if self.rtc.accepts(&input) {
                    self.rtc.handle_input(input).unwrap();
                }
            }
            Err(e) => {
                error!("Error parsing packet: {:?}", e);
            }
        }
    }

    fn handle_video(&mut self, msg: VideoPacket) {
        if let Some(mid) = self.camera_mapping.get(&msg.camera_id) {
            let Some(writer) = self.rtc.writer(*mid) else {
                return;
            };
            let pt = writer.payload_params().collect::<Vec<&PayloadParams>>()[0].pt();
            let timestamp: u64 = msg.timestamp.try_into().unwrap_or(0);
            let rtp_time = MediaTime::new(timestamp, Frequency::NINETY_KHZ);
            // debug!(
            //     "Writing packet for camera id {} to mid {}, time {}",
            //     msg.camera_id, mid, msg.timestamp
            // );
            if let Err(_e) = writer.write(pt, Instant::now(), rtp_time, msg.data) {
                error!("Error writing video packet! ");
            }
        }
    }

    async fn process_client_events(&mut self) -> Result<Duration, ()> {
        loop {
            let event = match self.rtc.poll_output() {
                Ok(event) => event,
                Err(err) => {
                    debug!("received rtc error: {}", err);
                    return Err(());
                }
            };

            match event {
                str0m::Output::Timeout(timeout) => {
                    // Drive time forward in client
                    let now = Instant::now();
                    if let Err(err) = self.rtc.handle_input(Input::Timeout(now)) {
                        error!("Error driving client forward: {}", err);
                    }

                    let duration = (timeout - Instant::now()).max(Duration::from_millis(1));

                    return Ok(duration);
                }
                str0m::Output::Transmit(t) => {
                    if t.contents.len() > 1400 {
                        debug!("It's a big boy! {}", t.contents.len());
                    }
                    if let Err(err) = self.udp_socket.send_to(&t.contents, t.destination).await {
                        error!("Error sending udp data to ! {}: {}", t.destination, err);
                        return Err(());
                    }
                }
                str0m::Output::Event(e) => match e {
                    str0m::Event::PeerStats(s) => {
                        debug!(
                            "Peer stats loss {:?}, bwe {:?}",
                            s.egress_loss_fraction, s.bwe_tx
                        );
                    }

                    str0m::Event::MediaEgressStats(s) => {
                        debug!("Media egress stats loss {:?}, nacks {:?}", s.loss, s.nacks);
                    }
                    str0m::Event::MediaAdded(_media) => {}
                    _ => {}
                },
            }
        }
    }

    pub async fn run(mut self) {
        let gauge = gauge!("webrtc_sessions");
        gauge.increment(1);
        let mut timeout = Duration::from_millis(200);
        self.parse_candidates().await;

        loop {
            // Process up to N events before checking RTC state
            const MAX_EVENTS_PER_BATCH: usize = 16;
            let mut events_processed = 0;
            let mut should_exit = false;

            // Process events in batch
            while events_processed < MAX_EVENTS_PER_BATCH {
                tokio::select! {
                    biased; // Process in order of priority

                    // High priority: websocket control messages
                    Some(msg) = self.websocket.recv() => {
                        if self.handle_websocket(msg).await.is_err() {
                            should_exit = true;
                            break;
                        }
                        events_processed += 1;
                    },

                    // Medium priority: UDP packets (time-sensitive)
                    Ok(udp_msg) = self.udp_receiver.recv() => {
                        self.handle_udp(udp_msg);
                        events_processed += 1;

                        // Try to drain more UDP packets without yielding
                        while let Ok(msg) = self.udp_receiver.try_recv() {
                            self.handle_udp(msg);
                            events_processed += 1;
                            if events_processed >= MAX_EVENTS_PER_BATCH {
                                break;
                            }
                        }
                    },

                    // Lower priority: video packets (buffer exists)
                    Some(msg) = self.video_receiver.recv() => {
                        self.handle_video(msg);
                        events_processed += 1;

                        // Try to drain more video packets without yielding
                        while let Ok(msg) = self.video_receiver.try_recv() {
                            self.handle_video(msg);
                            events_processed += 1;
                            if events_processed >= MAX_EVENTS_PER_BATCH {
                                break;
                            }
                        }

                        // Immediately flush to network. This prevents
                        // event batching from introducing video
                        // latency. Otherwise, video would buffer
                        // until =process_client_events()= is called
                        // once the event limit is reached.
                        timeout = match self.process_client_events().await {
                            Ok(t) => t,
                            Err(()) => break,
                        };

                    },

                    // Timeout: process RTC state even if no events
                    () = tokio::time::sleep(timeout) => {
                        break;
                     }
                }
            }

            if should_exit {
                break;
            }

            // Process RTC state machine after batch
            timeout = match self.process_client_events().await {
                Ok(t) => t,
                Err(()) => break,
            };

            if !self.rtc.is_alive() {
                break;
            }
        }

        self.video_router.unsubscribe(self.id).await;

        gauge.decrement(1);
    }
}
