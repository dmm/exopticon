use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::anyhow;
use axum::extract::ws::{Message, WebSocket};
use futures_util::StreamExt as _;
use tokio::{pin, sync::broadcast::Receiver, time::interval};
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::{MediaEngine, MIME_TYPE_H264},
        setting_engine::SettingEngine,
        APIBuilder,
    },
    data_channel::{data_channel_message::DataChannelMessage, RTCDataChannel},
    ice::udp_network::UDPNetwork,
    ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit},
    interceptor::registry::Registry,
    media::Sample,
    peer_connection::{
        configuration::RTCConfiguration, math_rand_alpha,
        peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::{track_local_static_sample::TrackLocalStaticSample, TrackLocal},
};

use crate::super_capture_actor::VideoPacket;

/// How often heartbeat pings are sent.
///
/// Should be half (or less) of the acceptable client timeout.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionCommand {
    camera_id: i32,
    track_id: uuid::Uuid,
}

/// A command from the client, transported over the websocket
/// connection
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "kind")]
pub enum SignalCommand {
    Offer {
        sdp: String,
    },
    Answer {
        sdp: String,
    },
    Candidate {
        candidate: Option<RTCIceCandidateInit>,
    },
    UpdateSubscriptions {
        subscriptions: Vec<SubscriptionCommand>,
    },
    CameraStatus {
        id: i32,
        status: bool,
    },
}

async fn build_peer_connection(udp_network: UDPNetwork) -> anyhow::Result<RTCPeerConnection> {
    let mut m = MediaEngine::default();
    m.register_default_codecs()?;

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut m)?;

    let mut setting_engine = SettingEngine::default();

    // This is just a token to be replaced later...

    setting_engine.set_udp_network(udp_network);
    let api = APIBuilder::new()
        .with_setting_engine(setting_engine)
        .with_media_engine(m)
        .with_interceptor_registry(registry)
        .build();

    let config = RTCConfiguration {
        ..Default::default()
    };

    // Create a new RTCPeerConnection
    Ok(api.new_peer_connection(config).await?)
}

struct Subscription {
    pub camera_id: i32,
    pub track_id: uuid::Uuid,
    pub track: Arc<TrackLocalStaticSample>,
    pub active: bool,
}

struct SignalChannel {
    last_heartbeat: Instant,
    pub peer_connection: RTCPeerConnection,
    pub subscriptions: HashMap<i32, Subscription>,
    session: WebSocket,
}

impl SignalChannel {
    pub fn new(session: WebSocket, peer_connection: RTCPeerConnection) -> Self {
        Self {
            last_heartbeat: Instant::now(),
            peer_connection,
            subscriptions: HashMap::new(),
            session,
        }
    }

    async fn close(self) -> anyhow::Result<()> {
        self.peer_connection.close().await?;
        self.session.close().await?;
        Ok(())
    }

    async fn ping(&mut self) -> anyhow::Result<()> {
        self.session.send(Message::Ping(Vec::new())).await?;
        Ok(())
    }

    fn timeout(&self) -> bool {
        if Instant::now().duration_since(self.last_heartbeat) > CLIENT_TIMEOUT {
            return true;
        }

        false
    }

    pub fn get_subscription(&self, camera_id: i32) -> Option<&Subscription> {
        self.subscriptions.get(&camera_id)
    }

    pub async fn send_subscriptions(&mut self) -> anyhow::Result<()> {
        let subscriptions_json = serde_json::to_string(&SignalCommand::UpdateSubscriptions {
            subscriptions: self
                .subscriptions
                .values()
                .map(|s| SubscriptionCommand {
                    camera_id: s.camera_id,
                    track_id: s.track_id,
                })
                .collect(),
        })?;

        self.session.send(Message::Text(subscriptions_json)).await?;

        Ok(())
    }

    async fn send_all_candidates(&mut self) -> anyhow::Result<()> {
        let candidates = vec![
            "dev.exopticon.org",
            "192.168.5.110",
            "fd5a:64a9:8631:4202:5054:ff:fe6a:2420",
            "2604:2d80:5f83:4202:5054:ff:fe6a:2420",
        ];
        for c in candidates {
            let candidate = RTCIceCandidateInit {
                candidate: format!("candidate:2388685802 1 udp 2130706431 {} 4000 typ host", c),
                sdp_mid: Some(String::new()),
                sdp_mline_index: Some(0),
                username_fragment: None,
            };
            let candidate_json = serde_json::to_string(&SignalCommand::Candidate {
                candidate: Some(candidate),
            })?;
            log::info!("Sending candidate! {}", &candidate_json);
            self.session.send(Message::Text(candidate_json)).await?;
        }

        Ok(())
    }

    async fn next(&mut self) -> Option<Result<Message, axum::Error>> {
        self.session.next().await
    }

    async fn handle_message(
        &mut self,
        next: Option<Result<Message, axum::Error>>,
    ) -> anyhow::Result<()> {
        match next {
            Some(Ok(msg)) => {
                match msg {
                    Message::Text(text_body) => {
                        //                        session.text(text.to_string()).await.unwrap();
                        let cmd: Result<SignalCommand, serde_json::Error> =
                            serde_json::from_str(&text_body);
                        match cmd {
                            Ok(SignalCommand::Offer { sdp }) => {
                                debug!("Got offer!!");
                                self.send_all_candidates().await?;
                                if let Ok(session_desc) = RTCSessionDescription::offer(sdp) {
                                    if let Err(e) = self
                                        .peer_connection
                                        .set_remote_description(session_desc)
                                        .await
                                    {
                                        error!("Failed to set local description! {}", e);
                                    } else {
                                        let answer =
                                            self.peer_connection.create_answer(None).await.unwrap();
                                        self.peer_connection
                                            .set_local_description(answer.clone())
                                            .await
                                            .unwrap();
                                        let answer_json =
                                            serde_json::to_string(&SignalCommand::Answer {
                                                sdp: answer.sdp,
                                            })
                                            .expect("Json camera frame serialization failed!");

                                        self.session
                                            .send(Message::Text(answer_json))
                                            .await
                                            .expect("Sending answer failed");
                                    }
                                } else {
                                    warn!("Failed to parse offer!");
                                }
                            }
                            Ok(
                                SignalCommand::Answer { sdp: _ }
                                | SignalCommand::Candidate { candidate: _ },
                            ) => {
                                // ignore for now
                            }
                            Ok(SignalCommand::UpdateSubscriptions { subscriptions }) => {
                                debug!("Got new subscriptions!: {:?}", subscriptions);
                            }
                            Ok(SignalCommand::CameraStatus { id, status }) => {
                                if let Some(mut sub) = self.subscriptions.get_mut(&id) {
                                    sub.active = status;
                                }
                            }
                            Err(e) => error!("Error webrtc! {}", e),
                        }
                    }

                    Message::Binary(_bin) => {}

                    Message::Close(_reason) => {
                        return Err(anyhow!("socket closed"));
                    }

                    Message::Ping(bytes) => {
                        self.last_heartbeat = Instant::now();
                        self.session.send(Message::Pong(bytes)).await?;
                    }

                    Message::Pong(_) => {
                        self.last_heartbeat = Instant::now();
                    }
                }
            }
            Some(Err(e)) => return Err(anyhow!("socket error {}", e)),
            // client WebSocket stream ended
            None => return Err(anyhow!("closed")),
        }

        Ok(())
    }
}

/// Echo text & binary messages received from the client, respond to ping messages, and monitor
/// connection health to detect network issues and free up resources.
#[allow(clippy::too_many_lines)]
pub async fn echo_heartbeat_ws(
    session: WebSocket,
    //    session: actix_ws::Session,
    //    mut msg_stream: actix_ws::MessageStream,
    udp_network: UDPNetwork,
    mut video_receiver: Receiver<VideoPacket>,
) {
    log::info!("connected");

    let pc = build_peer_connection(udp_network).await.unwrap();
    let (ice_tx, _ice_rx) = tokio::sync::mpsc::channel::<RTCIceCandidateInit>(1);
    pc.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
        log::debug!("Peer Connection State has changed: {}", s);

        if s == RTCPeerConnectionState::Failed {
            // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
            // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
            // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
            println!("Peer Connection has gone to failed exiting");
            //            let _ = done_tx.try_send(());
        }

        Box::pin(async {})
    }));

    pc.on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
        let ice_tx = ice_tx.clone();
        Box::pin(async move {
            if let Some(candidate) = candidate {
                if let Ok(candidate) = candidate.to_json() {
                    debug!("Got candidate: {:?}", &candidate);
                    ice_tx.send(candidate).await.unwrap();
                }
            }
        })
    }));

    pc.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
        let d_label = d.label().to_owned();

        // Register channel opening handling
        Box::pin(async move {
            let d2 = Arc::clone(&d);
            d.on_open(Box::new(move || {
                Box::pin(async move {
                    let mut result = anyhow::Result::<usize>::Ok(0);
                    while result.is_ok() {
                        let timeout = tokio::time::sleep(Duration::from_secs(5));
                        tokio::pin!(timeout);

                        tokio::select! {
                            _ = timeout.as_mut() =>{
                                let message = math_rand_alpha(15);
                                result = d2.send_text(message).await.map_err(Into::into);
                            }
                        };
                    }
                })
            }));

            // Register text message handling
            d.on_message(Box::new(move |msg: DataChannelMessage| {
                let msg_str = String::from_utf8(msg.data.to_vec()).unwrap();
                log::debug!("Message from DataChannel '{d_label}': '{msg_str}'");
                Box::pin(async {})
            }));
        })
    }));

    pc.on_track(Box::new(move |track, _| {
        tokio::spawn(async move {
            if let Some(track) = track {
                debug!("ON TRAAAAAAAAAAAAAAAACK {}", track.id().await);
            }
        });
        Box::pin(async {})
    }));

    let mut interval = interval(HEARTBEAT_INTERVAL);
    let mut signal_channel = SignalChannel::new(session, pc);
    loop {
        // create "next client timeout check" future
        let tick = interval.tick();
        // required for select()
        pin!(tick);

        // waits for either `msg_stream` to receive a message from the client or the heartbeat
        // interval timer to tick, yielding the value of whichever one is ready first
        //        match future::select(msg_stream.next(), tick).await {
        tokio::select! {
                    msg = signal_channel.next() => {
                        if let Err(e) = signal_channel.handle_message(msg).await {
                            error!("Handle socket error {}", e);
                            break
                        }
                    },

                    // heartbeat interval ticked
                    _inst = tick => {
                        // if no heartbeat ping/pong received recently, close the connection
                        if signal_channel.timeout() {
                            error!("signalling channel timeout");
                            break;
                        }

                        // send heartbeat ping
                        let _ping_result = signal_channel.ping().await;
                     },
                    packet_result = video_receiver.recv() => {
                        match packet_result {
                            Ok(packet) => {
                                let sub = signal_channel.get_subscription(packet.camera_id);
                                match sub {
                                    None => {
                                        let track_id = uuid::Uuid::new_v4();
                                        let track = Arc::new(TrackLocalStaticSample::new(
                                            RTCRtpCodecCapability {
                                                mime_type: MIME_TYPE_H264.to_owned(),
                                                ..Default::default()
                                            },
                                            "video".to_owned(),
                                            track_id.clone().to_string()
                                        ));

                                        debug!("Created new track: {:?}", track);

                                         let rtp_sender = signal_channel.peer_connection.add_track(Arc::clone(&track) as Arc<dyn TrackLocal + Send + Sync>).await.unwrap();
                                        let sender = rtp_sender.clone();
                                        tokio::spawn(async move {
                                            let mut rtcp_buf = vec![0u8; 1500];
                                            while let Ok((_, _)) = sender.read(&mut rtcp_buf).await {}
                                            anyhow::Result::<()>::Ok(())
                                        });

                                 signal_channel.subscriptions.insert(packet.camera_id, Subscription {camera_id: packet.camera_id, track_id,  track, active: false});
                                        signal_channel.send_subscriptions().await.unwrap();
                                    },
                                    Some(Subscription { camera_id: _, track_id: _, track, active }) => {
                                        if *active {
                                            let packet_timestamp = if let Ok(t) = u32::try_from(packet.timestamp % 4_294_967_295i64) { // u32::MAX
                                                t
                                            } else {
                                                log::error!("Invalid timestamp: {}", packet.timestamp);
                                                0
                                            };
                                        track.write_sample(&Sample {
                                            data: packet.data.into(),
                                            packet_timestamp,
        //                                    duration: Duration::from_micros(packet.duration as u64),
                                            duration: Duration::from_secs(1),
                                            ..Default::default()
                                        }).await.unwrap();
                                        }

                                    },
                                }
                            },
                            Err(e) => error!("Error receiving packet! {}", e),
                        }
                    }
                }
    }

    // attempt to close connection gracefully
    let _channel = signal_channel.close().await;

    log::info!("disconnected");
}
