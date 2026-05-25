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
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str;
use std::time::{Duration, Instant};
use sxd_document::dom::{Document, Element};
use sxd_document::parser;
use sxd_xpath::nodeset::Node as XPathNode;
use sxd_xpath::{Value, evaluate_xpath};
use tokio::net::UdpSocket;
use tokio::time::timeout;
use uuid::Uuid;

use crate::error::Error;
use crate::soap::{NS_NETWORK, NS_SOAP12, NS_WSA_DISCOVERY, NS_WSD, NS_XSD, NS_XSI, XmlWriter};

fn build_probe_request(message_id: Uuid) -> Result<String, Error> {
    let mut xml = XmlWriter::new();

    xml.start_attr(
        "Envelope",
        &[("xmlns", NS_SOAP12), ("xmlns:dn", NS_NETWORK)],
    )?;
    xml.start_attr("Header", &[("xmlns:wsa", NS_WSA_DISCOVERY)])?;
    xml.text_element("wsa:MessageID", message_id)?;
    xml.text_element("wsa:To", "urn:schemas-xmlsoap-org:ws:2005:04:discovery")?;
    xml.text_element(
        "wsa:Action",
        "http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe",
    )?;
    xml.end("Header")?;
    xml.start("Body")?;
    xml.start_attr(
        "Probe",
        &[
            ("xmlns", NS_WSD),
            ("xmlns:xsd", NS_XSD),
            ("xmlns:xsi", NS_XSI),
        ],
    )?;
    xml.text_element("Types", "dn:NetworkVideoTransmitter")?;
    xml.empty("Scopes")?;
    xml.end("Probe")?;
    xml.end("Body")?;
    xml.end("Envelope")?;

    xml.finish_string()
}

/// A structured WS-Discovery device result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredDevice {
    /// UDP address that sent the discovery response.
    pub remote_addr: SocketAddr,

    /// Device service addresses advertised by the camera.
    pub xaddrs: Vec<String>,

    /// WS-Discovery types advertised by the device.
    pub types: Vec<String>,

    /// WS-Discovery scopes advertised by the device.
    pub scopes: Vec<String>,

    /// Metadata version advertised by the device.
    pub metadata_version: Option<u32>,
}

/// Struct representing onvif discover probe
pub struct ProbeServer {
    /// multicast socket used for discovery
    socket: UdpSocket,

    /// time duration we should search for devices
    timeout: Duration,

    /// stores uninterpreted discovery results
    buf: Vec<u8>,

    /// Vec of discovery results
    results: Vec<DiscoveredDevice>,

    /// Stores start of discovery probe. This is used to limit
    /// discovery time.
    start: Option<Instant>,

    /// Message ID expected in response `RelatesTo`.
    message_id: Option<String>,
}

impl ProbeServer {
    /// Returns interpretation of discovery probe result
    fn interpret_probe(&mut self, size: usize, addr: SocketAddr) {
        let Some(message_id) = self.message_id.as_deref() else {
            return;
        };
        let Ok(response) = str::from_utf8(&self.buf[..size]) else {
            return;
        };

        match parse_probe_matches(response, message_id, addr) {
            Ok(mut devices) => self.results.append(&mut devices),
            Err(err) => debug!("ignored invalid discovery response from {addr}: {err}"),
        }
    }

    /// Calculates time left in probe interval
    fn time_left(&self) -> Option<Duration> {
        let start = self.start?;

        self.timeout
            .checked_sub(Instant::now().duration_since(start))
    }
    /// Sends a probe request and returns detected cameras
    pub async fn probe(&mut self) -> Result<Vec<DiscoveredDevice>, Error> {
        self.results.clear();
        let message_id = Uuid::new_v4();
        self.message_id = Some(message_id.to_string());
        let request_body = build_probe_request(message_id)?;

        let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(239, 255, 255, 250)), 3702);

        // Send discovery request
        self.socket
            .send_to(request_body.as_bytes(), &remote_addr)
            .await?;

        self.start = Some(Instant::now());

        // Receive responses
        while let Some(time_left) = self.time_left() {
            let rec = self.socket.recv_from(&mut self.buf);
            match timeout(time_left, rec).await {
                Ok(Ok((size, addr))) => self.interpret_probe(size, addr),
                Ok(Err(err)) => return Err(err.into()),
                Err(_) => (), // timeout, leave loop and return from fn
            }
        }

        Ok(std::mem::take(&mut self.results))
    }
}

fn xpath_string<'d>(doc: &'d Document<'d>, xpath: &str) -> Result<String, Error> {
    Ok(evaluate_xpath(doc, xpath)?.string())
}

fn xpath_bool<'d>(doc: &'d Document<'d>, xpath: &str) -> Result<bool, Error> {
    Ok(evaluate_xpath(doc, xpath)?.boolean())
}

fn has_element<'d>(doc: &'d Document<'d>, local_name: &str) -> Result<bool, Error> {
    xpath_bool(
        doc,
        &format!("boolean(//*[local-name()='{local_name}'][1])"),
    )
}

fn child_text(element: Element<'_>, local_name: &str) -> Option<String> {
    element.children().into_iter().find_map(|child| {
        let child = child.element()?;
        if child.name().local_part() == local_name {
            Some(XPathNode::from(child).string_value().trim().to_string())
        } else {
            None
        }
    })
}

fn split_tokens(value: Option<String>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

fn parse_probe_matches(
    xml: &str,
    expected_relates_to: &str,
    remote_addr: SocketAddr,
) -> Result<Vec<DiscoveredDevice>, Error> {
    let package = parser::parse(xml)?;
    let doc = package.as_document();

    if !has_element(&doc, "ProbeMatches")? {
        return Ok(Vec::new());
    }

    let relates_to = xpath_string(&doc, "normalize-space(//*[local-name()='RelatesTo'][1])")?;
    if relates_to != expected_relates_to {
        return Ok(Vec::new());
    }

    let nodes = match evaluate_xpath(&doc, "//*[local-name()='ProbeMatch']")? {
        Value::Nodeset(nodes) => nodes.document_order(),
        Value::Boolean(..) | Value::Number(..) | Value::String(..) => {
            return Err(Error::InvalidResponse);
        }
    };

    let mut devices = Vec::new();
    for node in nodes {
        let Some(element) = node.element() else {
            return Err(Error::InvalidResponse);
        };

        let xaddrs = split_tokens(child_text(element, "XAddrs"));
        if xaddrs.is_empty() {
            continue;
        }

        let metadata_version = match child_text(element, "MetadataVersion") {
            Some(value) if !value.is_empty() => Some(value.parse::<u32>()?),
            Some(_) | None => None,
        };

        devices.push(DiscoveredDevice {
            remote_addr,
            xaddrs,
            types: split_tokens(child_text(element, "Types")),
            scopes: split_tokens(child_text(element, "Scopes")),
            metadata_version,
        });
    }

    Ok(devices)
}

/// Returns discovered devices.
pub async fn discover(timeout: Duration) -> Result<Vec<DiscoveredDevice>, Error> {
    let local_addr: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
    let socket = UdpSocket::bind(&local_addr).await?;

    let mut p = ProbeServer {
        socket,
        timeout,
        buf: vec![0; 0xFFFF],
        results: Vec::new(),
        start: None,
        message_id: None,
    };

    p.probe().await
}

/// Returns number of discovered devices.
pub async fn probe(timeout: Duration) -> Result<usize, Error> {
    discover(timeout).await.map(|devices| devices.len())
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::time::Duration;

    use sxd_document::parser;
    use sxd_xpath::evaluate_xpath;
    use tokio::net::UdpSocket;
    use uuid::Uuid;

    use super::{ProbeServer, build_probe_request, parse_probe_matches};

    fn xpath_string(xml: &str, xpath: &str) -> String {
        let doc = parser::parse(xml).unwrap();
        evaluate_xpath(&doc.as_document(), xpath).unwrap().string()
    }

    fn xpath_bool(xml: &str, xpath: &str) -> bool {
        let doc = parser::parse(xml).unwrap();
        evaluate_xpath(&doc.as_document(), xpath).unwrap().boolean()
    }

    fn probe_match_xml(relates_to: &str, xaddrs: &str) -> String {
        format!(
            r"
            <Envelope>
                <Header>
                    <RelatesTo>{relates_to}</RelatesTo>
                </Header>
                <Body>
                    <ProbeMatches>
                        <ProbeMatch>
                            <Types>dn:NetworkVideoTransmitter tds:Device</Types>
                            <Scopes>scope-one scope-two</Scopes>
                            <XAddrs>{xaddrs}</XAddrs>
                            <MetadataVersion>7</MetadataVersion>
                        </ProbeMatch>
                    </ProbeMatches>
                </Body>
            </Envelope>
            "
        )
    }

    #[test]
    fn discovery_probe_xml_contains_ws_discovery_shape() {
        let message_id = Uuid::nil();
        let xml = build_probe_request(message_id).unwrap();

        assert!(xpath_bool(&xml, "boolean(//*[local-name()='Envelope'])"));
        assert!(xpath_bool(&xml, "boolean(//*[local-name()='Header'])"));
        assert!(xpath_bool(&xml, "boolean(//*[local-name()='Probe'])"));
        assert!(xpath_bool(&xml, "boolean(//*[local-name()='Scopes'])"));
        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='MessageID'])"),
            message_id.to_string()
        );
        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='Types'])"),
            "dn:NetworkVideoTransmitter"
        );
    }

    #[test]
    fn parse_probe_matches_returns_structured_devices() {
        let remote_addr: SocketAddr = "192.0.2.10:3702".parse().unwrap();
        let xml = probe_match_xml(
            "message-1",
            "http://192.0.2.10/onvif/device_service https://camera.example/onvif",
        );

        let devices = parse_probe_matches(&xml, "message-1", remote_addr).unwrap();

        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].remote_addr, remote_addr);
        assert_eq!(
            devices[0].xaddrs,
            vec![
                String::from("http://192.0.2.10/onvif/device_service"),
                String::from("https://camera.example/onvif"),
            ]
        );
        assert_eq!(
            devices[0].types,
            vec![
                String::from("dn:NetworkVideoTransmitter"),
                String::from("tds:Device"),
            ]
        );
        assert_eq!(
            devices[0].scopes,
            vec![String::from("scope-one"), String::from("scope-two")]
        );
        assert_eq!(devices[0].metadata_version, Some(7));
    }

    #[test]
    fn parse_probe_matches_ignores_mismatched_relates_to() {
        let remote_addr: SocketAddr = "192.0.2.10:3702".parse().unwrap();
        let xml = probe_match_xml("message-2", "http://192.0.2.10/onvif/device_service");

        let devices = parse_probe_matches(&xml, "message-1", remote_addr).unwrap();

        assert!(devices.is_empty());
    }

    #[test]
    fn parse_probe_matches_skips_matches_without_xaddrs() {
        let remote_addr: SocketAddr = "192.0.2.10:3702".parse().unwrap();
        let xml = probe_match_xml("message-1", "");

        let devices = parse_probe_matches(&xml, "message-1", remote_addr).unwrap();

        assert!(devices.is_empty());
    }

    #[test]
    fn parse_probe_matches_ignores_non_probe_matches_datagrams() {
        let remote_addr: SocketAddr = "192.0.2.10:3702".parse().unwrap();

        let devices = parse_probe_matches(
            "<Envelope><Body><Hello/></Body></Envelope>",
            "message-1",
            remote_addr,
        )
        .unwrap();

        assert!(devices.is_empty());
    }

    #[tokio::test]
    async fn interpret_probe_uses_received_datagram_size() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let remote_addr: SocketAddr = "192.0.2.10:3702".parse().unwrap();
        let xml = probe_match_xml("message-1", "http://192.0.2.10/onvif/device_service");
        let mut server = ProbeServer {
            socket,
            timeout: Duration::from_millis(1),
            buf: vec![0; 0xFFFF],
            results: Vec::new(),
            start: None,
            message_id: Some(String::from("message-1")),
        };
        server.buf[..xml.len()].copy_from_slice(xml.as_bytes());
        server.buf[xml.len()..xml.len() + 4].copy_from_slice(b"\0\0\0\0");

        server.interpret_probe(xml.len(), remote_addr);

        assert_eq!(server.results.len(), 1);
        assert_eq!(
            server.results[0].xaddrs,
            vec![String::from("http://192.0.2.10/onvif/device_service")]
        );
    }

    #[tokio::test]
    async fn interpret_probe_ignores_invalid_utf8() {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let remote_addr: SocketAddr = "192.0.2.10:3702".parse().unwrap();
        let mut server = ProbeServer {
            socket,
            timeout: Duration::from_millis(1),
            buf: vec![0xff, 0xfe, 0xfd],
            results: Vec::new(),
            start: None,
            message_id: Some(String::from("message-1")),
        };

        server.interpret_probe(3, remote_addr);

        assert!(server.results.is_empty());
    }
}
