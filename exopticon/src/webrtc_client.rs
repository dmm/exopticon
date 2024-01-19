use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, SocketAddr},
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
    writes: u32,
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
        let mut builder = Rtc::builder()
            .enable_h264(true)
            .enable_vp8(false)
            .enable_vp9(false);

        Self {
            websocket,
            udp_receiver,
            udp_socket,
            video_receiver,
            candidate_ips,
            rtc: builder.build(),
            writes: 0,
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
            ws::Message::Ping(_) => return Ok(()),
            ws::Message::Pong(_) => return Ok(()),
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
                self.subscribed_ids = HashSet::from_iter(subscribed_camera_ids.into_iter());
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
                debug!("OFFER: {}", spd_offer.to_sdp_string());
                let answer = match self.rtc.sdp_api().accept_offer(spd_offer) {
                    Ok(answer) => answer,
                    Err(err) => {
                        error!("accept_offer failed: {:?}", err);
                        return Err(());
                    }
                };
                debug!("ANSWER: {}", answer.to_sdp_string());
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

    async fn handle_udp(&mut self, (len, addr, mut buf): (usize, SocketAddr, Vec<u8>)) {
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

    async fn handle_video(&mut self, msg: VideoPacket) {
        if true {
            //self.subscribed_ids.contains(&msg.camera_id) {
            if let Some(mid) = self.camera_mapping.get(&msg.camera_id) {
                let writer = match self.rtc.writer(*mid) {
                    Some(w) => w,
                    None => {
                        //                        error!("unable for find a writer for {} {}", &msg.camera_id, mid);
                        return;
                    }
                };
                let pt = writer.payload_params().collect::<Vec<&PayloadParams>>()[0].pt();
                let rtp_time = MediaTime::new(msg.timestamp, Frequency::NINETY_KHZ);

                if let Err(e) = writer.write(pt, Instant::now(), rtp_time, msg.data) {
                    error!("Error writing video packet! writes({}) {}", self.writes, e);
                }
                self.writes += 1;
            }
        } else {
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
                str0m::Output::Timeout(_) => return Ok(Duration::from_millis(100)),
                str0m::Output::Transmit(t) => {
                    if let Err(err) = self.udp_socket.send_to(&t.contents, t.destination).await {
                        error!("Error sending udp data! {}", err);
                        return Err(());
                    }
                    self.writes = 0;
                }
                str0m::Output::Event(e) => match e {
                    str0m::Event::Connected => (),
                    str0m::Event::IceConnectionStateChange(_) => (),
                    str0m::Event::MediaAdded(media) => {}
                    str0m::Event::MediaData(_) => (),
                    str0m::Event::MediaChanged(_) => (),
                    str0m::Event::ChannelOpen(_, _) => (),
                    str0m::Event::ChannelData(_) => (),
                    str0m::Event::ChannelClose(_) => (),
                    str0m::Event::PeerStats(_) => (),
                    str0m::Event::MediaIngressStats(_) => (),
                    str0m::Event::MediaEgressStats(_) => (),
                    str0m::Event::EgressBitrateEstimate(_) => (),
                    str0m::Event::KeyframeRequest(_) => (),
                    str0m::Event::StreamPaused(_) => (),
                    str0m::Event::RtpPacket(_) => (),
                    str0m::Event::RawPacket(_) => (),
                    _ => (),
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
                    if let Err(_) = self.handle_websocket(msg).await {
                        info!("Got websocket error, exiting...");
                        return;
                    }
                },
                // webrtc udp packets
                Ok(udp_msg) = self.udp_receiver.recv() => self.handle_udp(udp_msg).await,
                // video packets
                Ok(msg) = self.video_receiver.recv() => self.handle_video(msg).await,
                // timeout
                _ = tokio::time::sleep(timeout) => {
                }
            }

            timeout = match self.process_client_events().await {
                Ok(t) => t,
                Err(_) => return,
            };

            if !self.rtc.is_alive() {
                return;
            }
        }
    }
}
