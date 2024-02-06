use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use axum::extract::ws::{self, WebSocket};
use str0m::{
    change::SdpOffer,
    format::PayloadParams,
    media::{Frequency, MediaTime, Mid},
    net::Protocol,
    Candidate, Input, Rtc,
};
use tokio::net::UdpSocket;

use crate::super_capture_actor::VideoPacket;

/// Messages from client
#[derive(Deserialize)]
#[serde(tag = "kind")]
#[serde(rename_all = "camelCase")]
pub enum ClientMessage {
    #[serde(rename_all = "camelCase")]
    SubscriptionUpdate { subscribed_camera_ids: Vec<i32> },
    #[serde(rename_all = "camelCase")]
    NegotiationRequest { offer: String },
    /// Maps camera id to Mid
    #[serde(rename_all = "camelCase")]
    StreamMapping { mappings: HashMap<String, i32> },
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
    websocket: WebSocket,
    /// used only to send messages
    udp_socket: Arc<UdpSocket>,
    udp_receiver: tokio::sync::broadcast::Receiver<(usize, SocketAddr, Vec<u8>)>,
    video_receiver: tokio::sync::broadcast::Receiver<VideoPacket>,
    candidate_ips: Vec<IpAddr>,
    rtc: Rtc,
    subscribed_ids: HashSet<i32>,
    camera_mapping: HashMap<i32, Mid>,
}

impl Client {
    pub fn new(
        websocket: WebSocket,
        udp_receiver: tokio::sync::broadcast::Receiver<(usize, SocketAddr, Vec<u8>)>,
        video_receiver: tokio::sync::broadcast::Receiver<VideoPacket>,
        udp_socket: Arc<UdpSocket>,
        candidate_ips: Vec<IpAddr>,
    ) -> Self {
        let rtc = Rtc::builder()
            .set_send_buffer_video(10000)
            .clear_codecs()
            .enable_h264(true)
            .build();

        Self {
            websocket,
            udp_receiver,
            udp_socket,
            video_receiver,
            candidate_ips,
            rtc,
            subscribed_ids: HashSet::new(),
            camera_mapping: HashMap::new(),
        }
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
                subscribed_camera_ids,
            } => {
                self.subscribed_ids = subscribed_camera_ids.into_iter().collect();
            }
            ClientMessage::NegotiationRequest { offer } => {
                let spd_offer =
                    SdpOffer::from_sdp_string(&offer).expect("Failed to deserialized sdp offer");

                // Add candidate ips
                for ip in &self.candidate_ips {
                    let addr = SocketAddr::new(*ip, 4000);
                    let candidate = Candidate::host(addr, Protocol::Udp).unwrap();
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
                for (mid_string, camera_id) in mappings {
                    let m: Mid = Mid::from(mid_string.as_str());
                    self.camera_mapping.insert(camera_id, m);
                }
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
        let destination_port = self.udp_socket.local_addr().unwrap().port();
        let destination_ip = self.candidate_ips.first().unwrap();

        let socket_addr = SocketAddr::new(*destination_ip, destination_port);
        match str0m::net::Receive::new(Protocol::Udp, addr, socket_addr, &buf) {
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
        if true {
            if let Some(mid) = self.camera_mapping.get(&msg.camera_id) {
                let Some(writer) = self.rtc.writer(*mid) else {
                    return;
                };
                let pt = writer.payload_params().collect::<Vec<&PayloadParams>>()[0].pt();
                let rtp_time = MediaTime::new(msg.timestamp, Frequency::NINETY_KHZ);
                // debug!(
                //     "Writing packet for camera id {} to mid {}, time {}",
                //     msg.camera_id, mid, msg.timestamp
                // );
                if let Err(_e) = writer.write(pt, Instant::now(), rtp_time, msg.data) {
                    error!("Error writing video packet! ");
                }
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
        let mut timeout = Duration::from_millis(100);
        loop {
            tokio::select! {
                // websocket control messages
                Some(msg) = self.websocket.recv() => {
                    if self.handle_websocket(msg).await.is_err() {
                        info!("Got websocket error, exiting...");
                        return;
                    }
                },
                // webrtc udp packets
                Ok(udp_msg) = self.udp_receiver.recv() => self.handle_udp(udp_msg),
                // video packets
                Ok(msg) = self.video_receiver.recv() => self.handle_video(msg),
                // timeout
                 () = tokio::time::sleep(timeout) => {
                }
            }

            timeout = match self.process_client_events().await {
                Ok(t) => t,
                Err(()) => return,
            };

            if !self.rtc.is_alive() {
                return;
            }
        }
    }
}
