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

//! SOAP and XML request generation utilities.

use std::fmt::Display;

use chrono::{Duration, SecondsFormat, Utc};
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use rand::Rng;

use crate::error::Error;

pub const NS_ADDRESSING: &str = "http://www.w3.org/2005/08/addressing";
pub const NS_DEVICE: &str = "http://www.onvif.org/ver10/device/wsdl";
pub const NS_NETWORK: &str = "http://www.onvif.org/ver10/network/wsdl";
pub const NS_PTZ: &str = "http://www.onvif.org/ver20/ptz/wsdl";
pub const NS_SCHEMA: &str = "http://www.onvif.org/ver10/schema";
pub const NS_SOAP12: &str = "http://www.w3.org/2003/05/soap-envelope";
pub const NS_WSA_DISCOVERY: &str = "http://schemas.xmlsoap.org/ws/2004/08/addressing";
pub const NS_WSD: &str = "http://schemas.xmlsoap.org/ws/2005/04/discovery";
pub const NS_WSSE: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd";
pub const NS_WSU: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd";
pub const NS_XSD: &str = "http://www.w3.org/2001/XMLSchema";
pub const NS_XSI: &str = "http://www.w3.org/2001/XMLSchema-instance";
pub const PASSWORD_DIGEST_TYPE: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordDigest";
pub const NONCE_ENCODING_TYPE: &str = "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-soap-message-security-1.0#Base64Binary";

/// A thin XML writer wrapper for ONVIF request generation.
pub struct XmlWriter {
    inner: Writer<Vec<u8>>,
}

impl XmlWriter {
    /// Returns an XML writer backed by an in-memory buffer.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: Writer::new(Vec::new()),
        }
    }

    /// Writes a start element.
    pub fn start(&mut self, name: &str) -> Result<&mut Self, Error> {
        self.inner
            .write_event(Event::Start(BytesStart::new(name)))?;
        Ok(self)
    }

    /// Writes a start element with attributes.
    pub fn start_attr(&mut self, name: &str, attrs: &[(&str, &str)]) -> Result<&mut Self, Error> {
        let mut event = BytesStart::new(name);

        for (key, value) in attrs {
            event.push_attribute((*key, *value));
        }

        self.inner.write_event(Event::Start(event))?;
        Ok(self)
    }

    /// Writes an empty element.
    pub fn empty(&mut self, name: &str) -> Result<&mut Self, Error> {
        self.inner
            .write_event(Event::Empty(BytesStart::new(name)))?;
        Ok(self)
    }

    /// Writes an empty element with attributes.
    pub fn empty_attr(&mut self, name: &str, attrs: &[(&str, &str)]) -> Result<&mut Self, Error> {
        let mut event = BytesStart::new(name);

        for (key, value) in attrs {
            event.push_attribute((*key, *value));
        }

        self.inner.write_event(Event::Empty(event))?;
        Ok(self)
    }

    /// Writes escaped text.
    pub fn text(&mut self, value: &str) -> Result<&mut Self, Error> {
        self.inner.write_event(Event::Text(BytesText::new(value)))?;
        Ok(self)
    }

    /// Writes an element containing escaped text.
    pub fn text_element(&mut self, name: &str, value: impl Display) -> Result<&mut Self, Error> {
        let text = value.to_string();
        self.start(name)?;
        self.text(&text)?;
        self.end(name)
    }

    /// Writes an element with attributes containing escaped text.
    pub fn text_element_attr(
        &mut self,
        name: &str,
        attrs: &[(&str, &str)],
        value: impl Display,
    ) -> Result<&mut Self, Error> {
        let text = value.to_string();
        self.start_attr(name, attrs)?;
        self.text(&text)?;
        self.end(name)
    }

    /// Writes an end element.
    pub fn end(&mut self, name: &str) -> Result<&mut Self, Error> {
        self.inner.write_event(Event::End(BytesEnd::new(name)))?;
        Ok(self)
    }

    /// Returns the generated XML as a String.
    pub fn finish_string(self) -> Result<String, Error> {
        Ok(String::from_utf8(self.inner.into_inner())?)
    }
}

impl Default for XmlWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// A SOAP request body payload.
pub trait SoapBody {
    /// Writes the request-specific SOAP body child.
    fn write_body(&self, xml: &mut XmlWriter) -> Result<(), Error>;
}

/// Credentials used for WS-Security `UsernameToken` authentication.
#[derive(Clone, Copy)]
pub struct Credentials<'a> {
    username: &'a str,
    password: &'a str,
}

impl<'a> Credentials<'a> {
    /// Returns a credentials wrapper borrowing the username and password.
    pub const fn new(username: &'a str, password: &'a str) -> Self {
        Self { username, password }
    }

    const fn has_security(self) -> bool {
        !(self.username.is_empty() && self.password.is_empty())
    }
}

/// Builds a complete SOAP 1.2 envelope around an ONVIF request body.
pub fn build_envelope<B: SoapBody>(
    body: &B,
    credentials: Option<Credentials<'_>>,
) -> Result<String, Error> {
    let mut xml = XmlWriter::new();

    xml.start_attr(
        "s:Envelope",
        &[("xmlns:s", NS_SOAP12), ("xmlns:a", NS_ADDRESSING)],
    )?;

    if let Some(credentials) = credentials.filter(|credentials| credentials.has_security()) {
        xml.start("s:Header")?;
        write_ws_security(&mut xml, credentials)?;
        xml.end("s:Header")?;
    }

    xml.start_attr("s:Body", &[("xmlns:xsi", NS_XSI), ("xmlns:xsd", NS_XSD)])?;
    body.write_body(&mut xml)?;
    xml.end("s:Body")?;
    xml.end("s:Envelope")?;

    xml.finish_string()
}

struct PasswordDigest {
    digest: String,
    nonce: String,
    timestamp: String,
}

fn generate_password_digest(password: &str, offset: Duration) -> Result<PasswordDigest, Error> {
    let timestamp = match Utc::now().checked_add_signed(offset) {
        Some(datetime) => datetime.to_rfc3339_opts(SecondsFormat::Millis, true),
        None => return Err(Error::InvalidArgument),
    };

    let mut rng = rand::thread_rng();
    let nonce: [u8; 16] = rng.r#gen();

    let mut hasher = Sha1::new();
    hasher.input(&nonce);
    hasher.input(timestamp.as_bytes());
    hasher.input(password.as_bytes());
    let mut hash_bytes = vec![0; hasher.output_bytes()];
    hasher.result(&mut hash_bytes);

    Ok(PasswordDigest {
        digest: base64::encode(hash_bytes.as_slice()),
        nonce: base64::encode(&nonce),
        timestamp,
    })
}

fn write_ws_security(xml: &mut XmlWriter, credentials: Credentials<'_>) -> Result<(), Error> {
    let digest = generate_password_digest(credentials.password, Duration::zero())?;

    xml.start_attr("Security", &[("s:mustUnderstand", "1"), ("xmlns", NS_WSSE)])?;
    xml.start("UsernameToken")?;
    xml.text_element("Username", credentials.username)?;
    xml.text_element_attr("Password", &[("Type", PASSWORD_DIGEST_TYPE)], digest.digest)?;
    xml.text_element_attr(
        "Nonce",
        &[("EncodingType", NONCE_ENCODING_TYPE)],
        digest.nonce,
    )?;
    xml.text_element_attr("Created", &[("xmlns", NS_WSU)], digest.timestamp)?;
    xml.end("UsernameToken")?;
    xml.end("Security")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use sxd_document::parser;
    use sxd_xpath::evaluate_xpath;

    use super::{Credentials, SoapBody, XmlWriter, build_envelope};
    use crate::error::Error;

    struct TestBody;

    impl SoapBody for TestBody {
        fn write_body(&self, xml: &mut XmlWriter) -> Result<(), Error> {
            xml.text_element("Ping", "pong")?;
            Ok(())
        }
    }

    fn xpath_string(xml: &str, xpath: &str) -> String {
        let doc = parser::parse(xml).unwrap();
        evaluate_xpath(&doc.as_document(), xpath).unwrap().string()
    }

    fn xpath_bool(xml: &str, xpath: &str) -> bool {
        let doc = parser::parse(xml).unwrap();
        evaluate_xpath(&doc.as_document(), xpath).unwrap().boolean()
    }

    #[test]
    fn xml_writer_escapes_text_and_attributes() {
        let mut xml = XmlWriter::new();
        xml.text_element_attr("Value", &[("attr", "1&2\"<")], "Tom & <tag> \" '")
            .unwrap();
        let xml = xml.finish_string().unwrap();

        assert_eq!(
            xpath_string(&xml, "string(/*[local-name()='Value'])"),
            "Tom & <tag> \" '"
        );
        assert_eq!(
            xpath_string(&xml, "string(/*[local-name()='Value']/@attr)"),
            "1&2\"<"
        );
    }

    #[test]
    fn envelope_without_credentials_omits_header() {
        let xml = build_envelope(&TestBody, None).unwrap();

        assert!(xpath_bool(&xml, "boolean(/*[local-name()='Envelope'])"));
        assert!(xpath_bool(&xml, "boolean(//*[local-name()='Body'])"));
        assert!(xpath_bool(&xml, "boolean(//*[local-name()='Ping'])"));
        assert!(!xpath_bool(&xml, "boolean(//*[local-name()='Header'])"));
    }

    #[test]
    fn envelope_with_credentials_adds_ws_security() {
        let xml = build_envelope(&TestBody, Some(Credentials::new("user&name", "pass"))).unwrap();

        assert!(xpath_bool(&xml, "boolean(//*[local-name()='Header'])"));
        assert!(xpath_bool(&xml, "boolean(//*[local-name()='Security'])"));
        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='Username'])"),
            "user&name"
        );
        assert!(xpath_bool(&xml, "boolean(//*[local-name()='Password'])"));
        assert!(xpath_bool(&xml, "boolean(//*[local-name()='Nonce'])"));
        assert!(xpath_bool(&xml, "boolean(//*[local-name()='Created'])"));
    }
}
