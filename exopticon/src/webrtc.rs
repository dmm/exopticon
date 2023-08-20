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
    net::SocketAddr,
    time::{Duration, Instant},
};

use str0m::{Input, Rtc};
use tokio::sync::mpsc::Receiver;

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

pub struct Client {
    rtc: Rtc,
    //cid: Option<String>
}

impl Client {
    pub fn new(rtc: Rtc) -> Self {
        Self { rtc }
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
                    str0m::Event::IceConnectionStateChange(_) => info!("IceConnectionStateChange!"),
                    str0m::Event::MediaAdded(_) => info!("media added!"),
                    str0m::Event::MediaData(_) => info!("MediaData"),
                    str0m::Event::MediaChanged(_) => info!("MediaChanged!"),
                    str0m::Event::KeyframeRequest(_) => info!("Keyframe request!"),
                    str0m::Event::ChannelOpen(_, _) => info!("Channel Open!"),
                    str0m::Event::ChannelData(_) => info!("Channel Data"),
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
    rtc_receiver: Receiver<Rtc>,
    socket: tokio::net::UdpSocket,
    clients: Vec<Client>,
}

impl Server {
    pub fn new(rtc_receiver: Receiver<Rtc>, socket: tokio::net::UdpSocket) -> Server {
        Server {
            rtc_receiver,
            socket,
            clients: Vec::new(),
        }
    }

    pub async fn spawn_new_client(&mut self, rtc: Rtc) {
        debug!("Spawning new RTC client");
        let client = Client::new(rtc);
        self.clients.push(client);
    }

    pub async fn handle_udp(&mut self, len: usize, addr: SocketAddr, buf: &mut Vec<u8>) {
        buf.truncate(len);
        debug!(
            "Handling UDP packet! {}, len: {}, local addr: {}",
            addr,
            buf.len(),
            self.socket.local_addr().unwrap()
        );
        match str0m::net::Receive::new(addr, self.socket.local_addr().unwrap(), buf) {
            Ok(receive_body) => {
                let input = Input::Receive(Instant::now(), receive_body);

                if let Some(client) = self.clients.iter_mut().find(|c| c.accepts(&input)) {
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
            let events: Vec<ClientEvent> = self.clients.iter_mut().map(|c| c.poll_rtc()).collect();
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
               Some(rtc) = self.rtc_receiver.recv() => self.spawn_new_client(rtc).await,
                Ok((len, addr)) = self.socket.recv_from(&mut buf) => self.handle_udp(len, addr, &mut buf).await,
                _ = tokio::time::sleep(timeout) => (),
            }

            timeout = self.process_client_events().await;

            // clean out disconnected clients
            self.clients.retain(|c| c.rtc.is_alive());
        }
    }
}
