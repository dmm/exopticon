//! Onvif device discovery

use futures::{Async, Future, Poll};
use std::io;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
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
}

impl Future for ProbeServer {
    type Item = usize;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<usize, io::Error> {
        let request_body =
            format!(
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
                Uuid::new_v4());

        let remote_addr = "239.255.255.250:3702"
            .parse::<SocketAddr>()
            .expect("Invalid probe address");
        match self.start {
            None => {
                match self
                    .socket
                    .poll_send_to(request_body.as_bytes(), &remote_addr)
                {
                    Ok(Async::Ready(_)) => {
                        self.start = Some(Instant::now());
                    }
                    Ok(Async::NotReady) => {}
                    Err(err) => {
                        return Err(err);
                    }
                }

                Ok(Async::NotReady)
            }
            Some(start) => {
                debug!("polling...");
                let since = Instant::now().duration_since(start);
                debug!(
                    "timeout: {}, Duration since: {}",
                    self.timeout.as_secs(),
                    since.as_secs()
                );
                if self.timeout <= since {
                    return Ok(Async::Ready(self.results.len()));
                }

                match self.socket.poll_recv_from(&mut self.buf) {
                    Ok(Async::Ready((size, addr))) => {
                        self.interpret_probe(size, addr);
                        Ok(Async::NotReady)
                    }
                    Ok(Async::NotReady) => Ok(Async::NotReady),
                    Err(err) => Err(err),
                }
            }
        }
    }
}

/// Returns number of discovered devices
pub fn probe(timeout: Duration) -> impl Future<Item = usize, Error = io::Error> {
    let local_addr: SocketAddr = "0.0.0.0:0".parse().expect("Invalid local address");
    let socket = UdpSocket::bind(&local_addr).expect("Create socket failed");

    ProbeServer {
        socket,
        timeout,
        buf: vec![0; 0xFFFF],
        results: Vec::new(),
        start: None,
    }
}
