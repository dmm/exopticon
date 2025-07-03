/*
 * onvif - An onvif client library
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of onvif.
 *
 * onvif is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * onvif is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with onvif.  If not, see <http://www.gnu.org/licenses/>.
 */

//! Onvif device discovery
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::time::timeout;
use uuid::Uuid;

/// Struct representing onvif discover probe
pub struct ProbeServer {
    /// multicast socket used for discovery
    socket: UdpSocket,

    /// time duration we should search for devices
    timeout: Duration,

    /// stores uninterpreted discovery results
    buf: Vec<u8>,

    /// Vec of discovery results
    results: Vec<String>,

    /// Stores start of discovery probe. This is used to limit
    /// discovery time.
    start: Option<Instant>,
}

impl ProbeServer {
    /// Returns interpretation of discovery probe result
    fn interpret_probe(&mut self, _size: usize, _addr: SocketAddr) {
        match String::from_utf8(self.buf.clone()) {
            Ok(res) => {
                debug!("len: {}, {}", &self.buf.len(), &res);
                self.results.push(res);
            }
            Err(_err) => (),
        }
    }

    /// Calculates time left in probe interval
    fn time_left(&self) -> Option<Duration> {
        let start = match self.start {
            None => {
                return None;
            }
            Some(time) => time,
        };
        self.timeout
            .checked_sub(Instant::now().duration_since(start))
    }
    /// Sends a probe request and returns detected cameras
    pub async fn probe(&mut self) -> Result<usize, io::Error> {
        let request_body = format!(
            r#"
            <Envelope xmlns="http://www.w3.org/2003/05/soap-envelope"
                      xmlns:dn="http://www.onvif.org/ver10/network/wsdl">
              <Header>
                <wsa:MessageID xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing">{}</wsa:MessageID>
                <wsa:To xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing">urn:schemas-xmlsoap-org:ws:2005:04:discovery</wsa:To>
                <wsa:Action xmlns:wsa="http://schemas.xmlsoap.org/ws/2004/08/addressing">http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe</wsa:Action>
              </Header>
              <Body>
                <Probe xmlns="http://schemas.xmlsoap.org/ws/2005/04/discovery"
                       xmlns:xsd="http://www.w3.org/2001/XMLSchema"
                         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
                  <Types>dn:NetworkVideoTransmitter</Types>
                  <Scopes />
                </Probe>
              </Body>
            </Envelope>"#,
            Uuid::new_v4()
        );

        let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(239, 255, 255, 250)), 3702);

        // Send discovery request
        match self
            .socket
            .send_to(request_body.as_bytes(), &remote_addr)
            .await
        {
            Ok(_) => (),
            Err(err) => return Err(err),
        }

        self.start = Some(Instant::now());

        // Receive responses
        while let Some(time_left) = self.time_left() {
            let rec = self.socket.recv_from(&mut self.buf);
            match timeout(time_left, rec).await {
                Ok(Ok((size, addr))) => self.interpret_probe(size, addr),
                Ok(Err(err)) => return Err(err),
                Err(_) => (), // timeout, leave loop and return from fn
            }
        }

        Ok(self.results.len())
    }
}

/// Returns number of discovered devices
pub async fn probe(timeout: Duration) -> Result<usize, io::Error> {
    let local_addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0);
    let socket = UdpSocket::bind(&local_addr).await?;

    let mut p = ProbeServer {
        socket,
        timeout,
        buf: vec![0; 0xFFFF],
        results: Vec::new(),
        start: None,
    };

    p.probe().await
}
