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

//! Onvif camera api client

use chrono::offset::TimeZone;
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::time::Duration;
use sxd_document::dom::{Document, Element};
use sxd_document::parser;
use sxd_xpath::nodeset::Node as XPathNode;
use sxd_xpath::{Value, evaluate_xpath};

use crate::error::Error;
use crate::soap::{Credentials, NS_DEVICE, NS_PTZ, NS_SCHEMA, SoapBody, XmlWriter, build_envelope};
use crate::util::{SoapClient, build_soap_client, soap_request};

const DEFAULT_CAMERA_PATH: &str = "/onvif/device_service";
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// An Onvif Camera is represented here
pub struct Camera {
    endpoint: hyper::Uri,
    username: String,
    password: String,
    timeout: Duration,
    client: SoapClient,
}

/// Ntp setting for device
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeType {
    /// indicates device should use manual configuration for clock
    Manual,

    /// indicates device should use ntp server to configure clock
    Ntp,
}

impl std::fmt::Display for TimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Self::Manual => write!(f, "Manual"),
            Self::Ntp => write!(f, "NTP"),
        }
    }
}

impl Serialize for TimeType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            Self::Manual => serializer.serialize_str("Manual"),
            Self::Ntp => serializer.serialize_str("NTP"),
        }
    }
}

impl<'de> Deserialize<'de> for TimeType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        match s.as_ref() {
            "Manual" => Ok(Self::Manual),
            "NTP" => Ok(Self::Ntp),
            _ => Err(serde::de::Error::custom("invalid TimeType specified")),
        }
    }
}

/// Struct representing device date and time settings
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDateAndTime {
    /// specifies whether ntp is enabled for device
    pub time_type: TimeType,

    /// specified whether daylight saving time is enabled for device
    pub daylight_savings: bool,

    /// time for device in POSIX format
    pub timezone: String,

    /// utc time for device
    pub utc_datetime: Option<DateTime<Utc>>,
}

impl Default for DeviceDateAndTime {
    /// Returns new `DeviceDateAndTime` struct
    fn default() -> Self {
        Self {
            time_type: TimeType::Manual,
            daylight_savings: false,
            timezone: String::from("UTC"),
            utc_datetime: None,
        }
    }
}

/// Specifies format of ntp server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NtpType {
    /// an ipv4 address
    #[serde(rename = "IPv4")]
    Ipv4,
    /// an ipv6 address
    #[serde(rename = "IPv6")]
    Ipv6,
    /// a dns hostname
    #[serde(rename = "DNS")]
    Dns,
}

impl std::fmt::Display for NtpType {
    /// Implementing display format for the `TimeType` enum
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Self::Ipv4 => write!(f, "IPv4"),
            Self::Ipv6 => write!(f, "IPv6"),
            Self::Dns => write!(f, "DNS"),
        }
    }
}
impl FromStr for NtpType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "IPv4" => Ok(Self::Ipv4),
            "IPv6" => Ok(Self::Ipv6),
            "DNS" => Ok(Self::Dns),
            _ => Err(Error::InvalidArgument),
        }
    }
}

/// ONVIF network host value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkHost {
    /// IPv4 network host.
    Ipv4(Ipv4Addr),
    /// IPv6 network host.
    Ipv6(Ipv6Addr),
    /// DNS network host.
    Dns(String),
}

impl NetworkHost {
    const fn host_type(&self) -> NtpType {
        match self {
            Self::Ipv4(_) => NtpType::Ipv4,
            Self::Ipv6(_) => NtpType::Ipv6,
            Self::Dns(_) => NtpType::Dns,
        }
    }
}

/// Struct representing device ntp settings
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NtpSettings {
    /// should ntp settings come from ntp
    pub from_dhcp: bool,
    /// NTP servers supplied by DHCP.
    pub ntp_from_dhcp: Vec<NetworkHost>,
    /// Manually configured NTP servers.
    pub ntp_manual: Vec<NetworkHost>,
}

impl NtpSettings {
    /// Returns NTP settings using DHCP.
    #[must_use]
    pub const fn from_dhcp() -> Self {
        Self {
            from_dhcp: true,
            ntp_from_dhcp: Vec::new(),
            ntp_manual: Vec::new(),
        }
    }

    /// Returns NTP settings using manual server values.
    #[must_use]
    pub const fn manual(ntp_manual: Vec<NetworkHost>) -> Self {
        Self {
            from_dhcp: false,
            ntp_from_dhcp: Vec::new(),
            ntp_manual,
        }
    }
}

struct GetSystemDateAndTimeRequest;

impl SoapBody for GetSystemDateAndTimeRequest {
    fn write_body(&self, xml: &mut XmlWriter) -> Result<(), Error> {
        xml.empty_attr("GetSystemDateAndTime", &[("xmlns", NS_DEVICE)])?;
        Ok(())
    }
}

struct SetDateAndTimeRequest<'a> {
    datetime: &'a DeviceDateAndTime,
}

impl SoapBody for SetDateAndTimeRequest<'_> {
    fn write_body(&self, xml: &mut XmlWriter) -> Result<(), Error> {
        xml.start_attr("SetSystemDateAndTime", &[("xmlns", NS_DEVICE)])?;
        xml.text_element("DateTimeType", self.datetime.time_type)?;
        xml.text_element("DaylightSavings", self.datetime.daylight_savings)?;
        xml.start("TimeZone")?;
        xml.text_element_attr("TZ", &[("xmlns", NS_SCHEMA)], &self.datetime.timezone)?;
        xml.end("TimeZone")?;

        if let Some(utc) = self.datetime.utc_datetime.as_ref() {
            xml.start("UTCDateTime")?;
            xml.start_attr("Time", &[("xmlns", NS_SCHEMA)])?;
            xml.text_element("Hour", utc.hour())?;
            xml.text_element("Minute", utc.minute())?;
            xml.text_element("Second", utc.second())?;
            xml.end("Time")?;
            xml.start_attr("Date", &[("xmlns", NS_SCHEMA)])?;
            xml.text_element("Year", utc.year())?;
            xml.text_element("Month", utc.month())?;
            xml.text_element("Day", utc.day())?;
            xml.end("Date")?;
            xml.end("UTCDateTime")?;
        }

        xml.end("SetSystemDateAndTime")?;
        Ok(())
    }
}

struct GetNtpRequest;

impl SoapBody for GetNtpRequest {
    fn write_body(&self, xml: &mut XmlWriter) -> Result<(), Error> {
        xml.empty_attr("GetNTP", &[("xmlns", NS_DEVICE)])?;
        Ok(())
    }
}

struct SetNtpRequest<'a> {
    ntp_settings: &'a NtpSettings,
}

impl SoapBody for SetNtpRequest<'_> {
    fn write_body(&self, xml: &mut XmlWriter) -> Result<(), Error> {
        xml.start_attr("SetNTP", &[("xmlns", NS_DEVICE)])?;
        xml.text_element("FromDHCP", self.ntp_settings.from_dhcp)?;
        for host in &self.ntp_settings.ntp_manual {
            write_network_host(xml, "NTPManual", host)?;
        }
        xml.end("SetNTP")?;
        Ok(())
    }
}

fn write_network_host(
    xml: &mut XmlWriter,
    element_name: &str,
    host: &NetworkHost,
) -> Result<(), Error> {
    xml.start(element_name)?;
    xml.text_element_attr("Type", &[("xmlns", NS_SCHEMA)], host.host_type())?;

    match host {
        NetworkHost::Ipv4(address) => {
            xml.text_element_attr("IPv4Address", &[("xmlns", NS_SCHEMA)], address)?;
        }
        NetworkHost::Ipv6(address) => {
            xml.text_element_attr("IPv6Address", &[("xmlns", NS_SCHEMA)], address)?;
        }
        NetworkHost::Dns(name) => {
            xml.text_element_attr("DNSname", &[("xmlns", NS_SCHEMA)], name)?;
        }
    }

    xml.end(element_name)?;
    Ok(())
}

struct RelativeMoveRequest<'a> {
    profile_token: &'a str,
    x: f32,
    y: f32,
    zoom: f32,
}

impl SoapBody for RelativeMoveRequest<'_> {
    fn write_body(&self, xml: &mut XmlWriter) -> Result<(), Error> {
        xml.start_attr("RelativeMove", &[("xmlns", NS_PTZ)])?;
        xml.text_element("ProfileToken", self.profile_token)?;
        xml.start("Translation")?;
        write_pan_tilt_vectors(xml, self.x, self.y, self.zoom)?;
        xml.end("Translation")?;
        xml.end("RelativeMove")?;
        Ok(())
    }
}

struct ContinuousMoveRequest<'a> {
    profile_token: &'a str,
    x: f32,
    y: f32,
    zoom: f32,
    timeout: f32,
}

impl SoapBody for ContinuousMoveRequest<'_> {
    #[allow(clippy::float_arithmetic)]
    fn write_body(&self, xml: &mut XmlWriter) -> Result<(), Error> {
        xml.start_attr("ContinuousMove", &[("xmlns", NS_PTZ)])?;
        xml.text_element("ProfileToken", self.profile_token)?;
        xml.start("Velocity")?;
        write_pan_tilt_vectors(xml, self.x, self.y, self.zoom)?;
        xml.end("Velocity")?;

        if self.timeout != 0.0 {
            let seconds = (self.timeout / 1000.0).to_string();
            let mut timeout = String::with_capacity(seconds.len() + 3);
            timeout.push_str("PT");
            timeout.push_str(&seconds);
            timeout.push('S');
            xml.text_element("Timeout", timeout)?;
        }

        xml.end("ContinuousMove")?;
        Ok(())
    }
}

struct AbsoluteMoveRequest<'a> {
    profile_token: &'a str,
    x: f32,
    y: f32,
    zoom: f32,
}

impl SoapBody for AbsoluteMoveRequest<'_> {
    fn write_body(&self, xml: &mut XmlWriter) -> Result<(), Error> {
        xml.start_attr("AbsoluteMove", &[("xmlns", NS_PTZ)])?;
        xml.text_element("ProfileToken", self.profile_token)?;
        xml.start("Position")?;
        write_pan_tilt_vectors(xml, self.x, self.y, self.zoom)?;
        xml.end("Position")?;
        xml.end("AbsoluteMove")?;
        Ok(())
    }
}

struct StopRequest<'a> {
    profile_token: &'a str,
}

impl SoapBody for StopRequest<'_> {
    fn write_body(&self, xml: &mut XmlWriter) -> Result<(), Error> {
        xml.start_attr("Stop", &[("xmlns", NS_PTZ)])?;
        xml.text_element("ProfileToken", self.profile_token)?;
        xml.end("Stop")?;
        Ok(())
    }
}

fn write_pan_tilt_vectors(xml: &mut XmlWriter, x: f32, y: f32, zoom: f32) -> Result<(), Error> {
    let x = x.to_string();
    let y = y.to_string();
    xml.empty_attr(
        "PanTilt",
        &[("x", x.as_str()), ("y", y.as_str()), ("xmlns", NS_SCHEMA)],
    )?;

    if zoom != 0.0 {
        let zoom = zoom.to_string();
        xml.empty_attr("Zoom", &[("x", zoom.as_str()), ("xmlns", NS_SCHEMA)])?;
    }

    Ok(())
}

fn format_uri_host(host: &str) -> String {
    if host.parse::<Ipv6Addr>().is_ok() {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

fn validate_endpoint(endpoint: &str) -> Result<hyper::Uri, Error> {
    let Ok(uri) = endpoint.parse::<hyper::Uri>() else {
        return Err(Error::InvalidArgument);
    };

    match uri.scheme_str() {
        Some("http" | "https") if uri.host().is_some() => Ok(uri),
        _ => Err(Error::InvalidArgument),
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

fn parse_soap_fault<'d>(doc: &'d Document<'d>) -> Result<Option<Error>, Error> {
    if !has_element(doc, "Fault")? {
        return Ok(None);
    }

    let fault_text = xpath_string(doc, "normalize-space(//*[local-name()='Fault'][1])")?;
    if has_element(doc, "InvalidTimeZone")?
        || has_element(doc, "InvalidDateTime")?
        || has_element(doc, "NtpServerUndefined")?
        || fault_text.contains("InvalidTimeZone")
        || fault_text.contains("InvalidDateTime")
        || fault_text.contains("NtpServerUndefined")
    {
        return Ok(Some(Error::InvalidArgument));
    }

    let reason = xpath_string(
        doc,
        "normalize-space(//*[local-name()='Fault'][1]//*[local-name()='Reason']/*[local-name()='Text'][1])",
    )?;
    let code = xpath_string(
        doc,
        "normalize-space(//*[local-name()='Fault'][1]//*[local-name()='Code']//*[local-name()='Value'][last()])",
    )?;
    let message = [reason, code, fault_text]
        .into_iter()
        .find(|value| !value.is_empty())
        .unwrap_or_else(|| String::from("SOAP Fault"));

    Ok(Some(Error::SoapFault(message)))
}

fn require_response_element<'d>(doc: &'d Document<'d>, local_name: &str) -> Result<(), Error> {
    if has_element(doc, local_name)? {
        return Ok(());
    }

    if let Some(error) = parse_soap_fault(doc)? {
        return Err(error);
    }

    Err(Error::InvalidResponse)
}

fn parse_empty_response(body: Vec<u8>, response_name: &str) -> Result<(), Error> {
    let string_body = String::from_utf8(body)?;
    let doc = parser::parse(&string_body)?;
    let doc = doc.as_document();

    require_response_element(&doc, response_name)
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

fn parse_network_host(node: XPathNode<'_>) -> Result<NetworkHost, Error> {
    let Some(element) = node.element() else {
        return Err(Error::InvalidResponse);
    };

    let host_type = match child_text(element, "Type").as_deref() {
        Some("IPv4") => NtpType::Ipv4,
        Some("IPv6") => NtpType::Ipv6,
        Some("DNS") => NtpType::Dns,
        Some(_) | None => return Err(Error::InvalidResponse),
    };

    match host_type {
        NtpType::Ipv4 => {
            let address = child_text(element, "IPv4Address").ok_or(Error::InvalidResponse)?;
            address
                .parse()
                .map(NetworkHost::Ipv4)
                .map_err(|_err| Error::InvalidResponse)
        }
        NtpType::Ipv6 => {
            let address = child_text(element, "IPv6Address").ok_or(Error::InvalidResponse)?;
            address
                .parse()
                .map(NetworkHost::Ipv6)
                .map_err(|_err| Error::InvalidResponse)
        }
        NtpType::Dns => {
            let name = child_text(element, "DNSname").ok_or(Error::InvalidResponse)?;
            if name.is_empty() {
                Err(Error::InvalidResponse)
            } else {
                Ok(NetworkHost::Dns(name))
            }
        }
    }
}

fn parse_network_hosts<'d>(
    doc: &'d Document<'d>,
    element_name: &str,
) -> Result<Vec<NetworkHost>, Error> {
    match evaluate_xpath(doc, &format!("//*[local-name()='{element_name}']"))? {
        Value::Nodeset(nodes) => nodes
            .document_order()
            .into_iter()
            .map(parse_network_host)
            .collect(),
        Value::Boolean(..) | Value::Number(..) | Value::String(..) => Err(Error::InvalidResponse),
    }
}

impl Camera {
    /// Returns a camera using the default ONVIF device service path.
    pub fn new(
        host: impl AsRef<str>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, Error> {
        let host = format_uri_host(host.as_ref());
        let endpoint = format!("http://{host}:{port}{DEFAULT_CAMERA_PATH}");
        Self::from_xaddr(endpoint, username, password)
    }

    /// Returns a camera using a discovery-provided or custom service endpoint.
    pub fn from_xaddr(
        xaddr: impl AsRef<str>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, Error> {
        let endpoint = validate_endpoint(xaddr.as_ref())?;

        Ok(Self {
            endpoint,
            username: username.into(),
            password: password.into(),
            timeout: DEFAULT_REQUEST_TIMEOUT,
            client: build_soap_client()?,
        })
    }

    /// Sets the request timeout and returns the camera.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Returns the configured request timeout.
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Returns the configured service endpoint.
    #[must_use]
    pub const fn endpoint(&self) -> &hyper::Uri {
        &self.endpoint
    }

    /// Returns the configured service endpoint as a string.
    #[must_use]
    pub fn url(&self) -> String {
        self.endpoint.to_string()
    }

    fn authenticated_envelope<B: SoapBody>(&self, body: &B) -> Result<String, Error> {
        build_envelope(body, Some(Credentials::new(&self.username, &self.password)))
    }

    /// Perform request for date and time information from
    /// camera. Returns xml response body.
    ///
    pub async fn request_get_date_and_time(&self) -> Result<Vec<u8>, Error> {
        let body = build_envelope(&GetSystemDateAndTimeRequest, None)?;
        soap_request(&self.client, self.endpoint(), body, self.timeout).await
    }

    /// Returns parsed date and time body.
    ///
    /// # Arguments
    ///
    /// * `body` - response body from camera
    ///
    pub fn parse_get_date_and_time(body: Vec<u8>) -> Result<DeviceDateAndTime, Error> {
        let string_body = String::from_utf8(body)?;
        let doc = parser::parse(&string_body)?;
        let doc = doc.as_document();

        require_response_element(&doc, "GetSystemDateAndTimeResponse")?;

        let camera_datetime = if has_element(&doc, "UTCDateTime")? {
            let year = xpath_string(
                &doc,
                "normalize-space(//*[local-name()='UTCDateTime']/*[local-name()='Date']/*[local-name()='Year'][1])",
            )?
            .parse::<i32>()?;
            let month = xpath_string(
                &doc,
                "normalize-space(//*[local-name()='UTCDateTime']/*[local-name()='Date']/*[local-name()='Month'][1])",
            )?
            .parse::<u32>()?;
            let day = xpath_string(
                &doc,
                "normalize-space(//*[local-name()='UTCDateTime']/*[local-name()='Date']/*[local-name()='Day'][1])",
            )?
            .parse::<u32>()?;
            let hour = xpath_string(
                &doc,
                "normalize-space(//*[local-name()='UTCDateTime']/*[local-name()='Time']/*[local-name()='Hour'][1])",
            )?
            .parse::<u32>()?;
            let minute = xpath_string(
                &doc,
                "normalize-space(//*[local-name()='UTCDateTime']/*[local-name()='Time']/*[local-name()='Minute'][1])",
            )?
            .parse::<u32>()?;
            let second = xpath_string(
                &doc,
                "normalize-space(//*[local-name()='UTCDateTime']/*[local-name()='Time']/*[local-name()='Second'][1])",
            )?
            .parse::<u32>()?;

            match Utc.with_ymd_and_hms(year, month, day, hour, minute, second) {
                chrono::LocalResult::Single(datetime) => Some(datetime),
                chrono::LocalResult::None | chrono::LocalResult::Ambiguous(_, _) => {
                    return Err(Error::InvalidResponse);
                }
            }
        } else {
            None
        };

        let date_time_type =
            match xpath_string(&doc, "normalize-space(//*[local-name()='DateTimeType'][1])")?
                .as_ref()
            {
                "Manual" => TimeType::Manual,
                "NTP" => TimeType::Ntp,
                _ => return Err(Error::InvalidResponse),
            };

        let daylight_savings = xpath_string(
            &doc,
            "normalize-space(//*[local-name()='DaylightSavings'][1])",
        )?
        .parse::<bool>()?;

        let timezone = xpath_string(
            &doc,
            "normalize-space(//*[local-name()='TimeZone'][1]/*[local-name()='TZ'][1])",
        )?;

        Ok(DeviceDateAndTime {
            time_type: date_time_type,
            daylight_savings,
            timezone,
            utc_datetime: camera_datetime,
        })
    }

    /// Requests date and time settings from camera
    pub async fn get_date_and_time(&self) -> Result<DeviceDateAndTime, Error> {
        let res = self.request_get_date_and_time().await?;
        Self::parse_get_date_and_time(res)
    }

    /// Submits `set_date_and_time` call and returns the raw result as a Future.
    ///
    /// # Arguments
    ///
    /// * `datetime` - date and time configuration to send to camera
    ///
    pub async fn request_set_date_and_time(
        &self,
        datetime: &DeviceDateAndTime,
    ) -> Result<Vec<u8>, Error> {
        let body = self.authenticated_envelope(&SetDateAndTimeRequest { datetime })?;
        soap_request(&self.client, self.endpoint(), body, self.timeout).await
    }

    /// Returns nothing if the parsed body represents a successful
    /// call and an Error otherwise.
    ///
    /// # Arguments
    ///
    /// * `body` - result of `set_date_and_time` request
    ///
    /// # Errors
    ///
    /// If this function encounters a parsing error it will return
    /// `Error::InvalidArgument`.
    ///
    pub fn parse_set_date_and_time(body: Vec<u8>) -> Result<(), Error> {
        parse_empty_response(body, "SetSystemDateAndTimeResponse")
    }

    /// Returns nothing on success.
    ///
    /// # Arguments
    ///
    /// * `datetime` - date and time settings to assign camera
    ///
    pub async fn set_date_and_time(&self, datetime: &DeviceDateAndTime) -> Result<(), Error> {
        let res = self.request_set_date_and_time(datetime).await?;
        Self::parse_set_date_and_time(res)
    }

    /// Performs request to get ntp configuration and returns response
    /// text on success.
    pub async fn request_get_ntp(&self) -> Result<Vec<u8>, Error> {
        let body = self.authenticated_envelope(&GetNtpRequest)?;
        soap_request(&self.client, self.endpoint(), body, self.timeout).await
    }

    /// Returns NTP configuration struct
    ///
    /// # Arguments
    ///
    /// * `body` - utf8 encoded xml response body
    ///
    /// # Errors
    ///
    /// If this function encounters a parsing error it will return an
    /// `Error::InvalidResponse`.
    ///
    pub fn parse_get_ntp(body: Vec<u8>) -> Result<NtpSettings, Error> {
        let string_body = String::from_utf8(body)?;
        let doc = parser::parse(&string_body)?;
        let doc = doc.as_document();

        require_response_element(&doc, "GetNTPResponse")?;

        let from_dhcp = xpath_string(&doc, "normalize-space(//*[local-name()='FromDHCP'][1])")?
            .parse::<bool>()?;
        let ntp_from_dhcp = parse_network_hosts(&doc, "NTPFromDHCP")?;
        let ntp_manual = parse_network_hosts(&doc, "NTPManual")?;

        Ok(NtpSettings {
            from_dhcp,
            ntp_from_dhcp,
            ntp_manual,
        })
    }

    /// Fetch camera's ntp settings
    pub async fn get_ntp(&self) -> Result<NtpSettings, Error> {
        let res = self.request_get_ntp().await?;
        Self::parse_get_ntp(res)
    }

    /// Performs a request to set camera's ntp settings. Returns
    /// response body.
    ///
    /// # Arguments
    ///
    /// * `ntp_settings` - new settings for camera
    ///
    pub async fn request_set_ntp(&self, ntp_settings: &NtpSettings) -> Result<Vec<u8>, Error> {
        let body = self.authenticated_envelope(&SetNtpRequest { ntp_settings })?;
        debug!("SetNTP request to {}", self.endpoint());
        soap_request(&self.client, self.endpoint(), body, self.timeout).await
    }

    /// Parse set ntp response body. Returns () on success.
    ///
    /// # Arguments
    ///
    /// * `body` - utf-8 encoded response body from set ntp call
    ///
    pub fn parse_set_ntp(body: Vec<u8>) -> Result<(), Error> {
        debug!("SetNTP response received");
        parse_empty_response(body, "SetNTPResponse")
    }

    /// Attempts to set camera's ntp settings. Returns () on success.
    ///
    /// # Arguments
    ///
    /// * `ntp_settings` - new ntp settings
    ///
    pub async fn set_ntp(&self, ntp_settings: &NtpSettings) -> Result<(), Error> {
        let res = self.request_set_ntp(ntp_settings).await?;
        Self::parse_set_ntp(res)
    }

    /// performs relative ptz move request
    pub async fn request_relative_move(
        &self,
        profile_token: &str,
        x: f32,
        y: f32,
        zoom: f32,
    ) -> Result<Vec<u8>, Error> {
        let body = self.authenticated_envelope(&RelativeMoveRequest {
            profile_token,
            x,
            y,
            zoom,
        })?;
        debug!("RelativeMove request to {}", self.endpoint());
        soap_request(&self.client, self.endpoint(), body, self.timeout).await
    }

    /// parse result of relative ptz move request
    pub fn parse_relative_move(body: Vec<u8>) -> Result<(), Error> {
        debug!("RelativeMove response received");
        parse_empty_response(body, "RelativeMoveResponse")
    }

    /// Requests a relative ptz move. Returns () on success.
    ///
    /// # Arguments
    ///
    /// * `profile_token` - ptz profile token to use for request
    /// * `x` - amount to move, inclusively between -1.0 and 1.0, in x axis
    /// * `y` - amount to move, inclusively between -1.0 and 1.0, in y axis
    /// * `zoom` - amount to change zoom, inclusively between -1.0 and 1.0
    pub async fn relative_move(
        &self,
        profile_token: &str,
        x: f32,
        y: f32,
        zoom: f32,
    ) -> Result<(), Error> {
        let res = self
            .request_relative_move(profile_token, x, y, zoom)
            .await?;
        Self::parse_relative_move(res)
    }

    /// performs continuous ptz move request
    pub async fn request_continuous_move(
        &self,
        profile_token: &str,
        x: f32,
        y: f32,
        zoom: f32,
        timeout: f32,
    ) -> Result<Vec<u8>, Error> {
        let body = self.authenticated_envelope(&ContinuousMoveRequest {
            profile_token,
            x,
            y,
            zoom,
            timeout,
        })?;
        debug!("ContinuousMove request to {}", self.endpoint());
        soap_request(&self.client, self.endpoint(), body, self.timeout).await
    }

    /// parse result of continuous ptz move request
    ///
    /// # Error
    ///
    /// Returns Err when the body is fails to parse as xml.
    ///
    pub fn parse_continuous_move(body: Vec<u8>) -> Result<(), Error> {
        debug!("ContinuousMove response received");
        parse_empty_response(body, "ContinuousMoveResponse")
    }

    /// Requests a continuous ptz move. Returns () on success.
    ///
    /// # Arguments
    ///
    /// * `profile_token` - ptz profile token to use for request
    /// * `x` - speed to move, inclusively between 0.0 and 1.0, in x axis
    /// * `y` - speed to move, inclusively between 0.0 and 1.0, in y axis
    /// * `zoom` - speed to zoom, inclusively between 0.0 and 1.0
    /// * `timeout` - timeout in milliseconds for move to last, or 0.0 for indefinite
    ///
    /// # Error
    ///
    /// Returns Err when we cannot connect to the camera, the camera
    /// signals an error, or we cannot parse the response.
    ///
    pub async fn continuous_move(
        &self,
        profile_token: &str,
        x: f32,
        y: f32,
        zoom: f32,
        timeout: f32,
    ) -> Result<(), Error> {
        let res = self
            .request_continuous_move(profile_token, x, y, zoom, timeout)
            .await?;
        Self::parse_continuous_move(res)
    }

    /// performs an absolute ptz move request
    pub async fn request_absolute_move(
        &self,
        profile_token: &str,
        x: f32,
        y: f32,
        zoom: f32,
    ) -> Result<Vec<u8>, Error> {
        let body = self.authenticated_envelope(&AbsoluteMoveRequest {
            profile_token,
            x,
            y,
            zoom,
        })?;
        debug!("AbsoluteMove request to {}", self.endpoint());
        soap_request(&self.client, self.endpoint(), body, self.timeout).await
    }

    /// parse result of absolute ptz move request
    pub fn parse_absolute_move(body: Vec<u8>) -> Result<(), Error> {
        debug!("AbsoluteMove response received");
        parse_empty_response(body, "AbsoluteMoveResponse")
    }

    /// Requests an absolute ptz move. Returns () on success.
    ///
    /// # Arguments
    ///
    /// * `profile_token` - ptz profile token to use for request
    /// * `x` - speed to move, inclusively between -1.0 and 1.0, in x axis
    /// * `y` - speed to move, inclusively between -1.0 and 1.0, in y axis
    /// * `zoom` - speed to zoom, inclusively between 0.0 and 1.0
    ///
    pub async fn absolute_move(
        &self,
        profile_token: &str,
        x: f32,
        y: f32,
        zoom: f32,
    ) -> Result<(), Error> {
        let res = self
            .request_absolute_move(profile_token, x, y, zoom)
            .await?;
        Self::parse_absolute_move(res)
    }

    /// performs a stop ptz move request
    pub async fn request_stop(&self, profile_token: &str) -> Result<Vec<u8>, Error> {
        let body = self.authenticated_envelope(&StopRequest { profile_token })?;
        debug!("Stop request to {}", self.endpoint());
        soap_request(&self.client, self.endpoint(), body, self.timeout).await
    }

    /// parse result of stop ptz move request
    pub fn parse_stop(body: Vec<u8>) -> Result<(), Error> {
        debug!("Stop response received");
        parse_empty_response(body, "StopResponse")
    }

    /// Requests a ptz stop. Returns () on success.
    ///
    /// # Arguments
    ///
    /// * `profile_token` - ptz profile token to use for request
    ///
    pub async fn stop(&self, profile_token: &str) -> Result<(), Error> {
        let res = self.request_stop(profile_token).await?;
        Self::parse_stop(res)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};
    use sxd_document::parser;
    use sxd_xpath::evaluate_xpath;

    use super::{
        AbsoluteMoveRequest, Camera, ContinuousMoveRequest, DeviceDateAndTime, GetNtpRequest,
        GetSystemDateAndTimeRequest, NetworkHost, NtpSettings, NtpType, RelativeMoveRequest,
        SetDateAndTimeRequest, SetNtpRequest, StopRequest, TimeType,
    };
    use crate::error::Error;
    use crate::soap::{Credentials, SoapBody, build_envelope};

    fn request_xml(body: &impl SoapBody) -> String {
        build_envelope(body, Some(Credentials::new("user", "pass"))).unwrap()
    }

    fn xpath_string(xml: &str, xpath: &str) -> String {
        let doc = parser::parse(xml).unwrap();
        evaluate_xpath(&doc.as_document(), xpath).unwrap().string()
    }

    fn xpath_bool(xml: &str, xpath: &str) -> bool {
        let doc = parser::parse(xml).unwrap();
        evaluate_xpath(&doc.as_document(), xpath).unwrap().boolean()
    }

    fn assert_xpath(xml: &str, xpath: &str) {
        assert!(xpath_bool(xml, xpath), "{xpath}");
    }

    fn response_xml(response_name: &str) -> Vec<u8> {
        format!("<Envelope><Body><{response_name}/></Body></Envelope>").into_bytes()
    }

    fn soap_fault_xml(detail: &str) -> Vec<u8> {
        format!(
            "<Envelope><Body><Fault><Code><Value>s:Sender</Value></Code><Reason><Text>failed</Text></Reason><Detail>{detail}</Detail></Fault></Body></Envelope>"
        )
        .into_bytes()
    }

    #[test]
    fn camera_endpoint_constructors_validate_and_format_urls() {
        let camera = Camera::new("2001:db8::1", 8899, "user", "pass").unwrap();
        assert_eq!(
            camera.url(),
            "http://[2001:db8::1]:8899/onvif/device_service"
        );

        let camera =
            Camera::from_xaddr("https://camera.local/custom/path", "user", "pass").unwrap();
        assert_eq!(camera.url(), "https://camera.local/custom/path");

        assert!(matches!(
            Camera::from_xaddr("ftp://camera.local/onvif", "user", "pass"),
            Err(Error::InvalidArgument)
        ));
    }

    #[test]
    fn get_system_date_and_time_xml_is_unauthenticated_device_request() {
        let xml = build_envelope(&GetSystemDateAndTimeRequest, None).unwrap();

        assert_xpath(&xml, "boolean(//*[local-name()='GetSystemDateAndTime'])");
        assert!(!xpath_bool(&xml, "boolean(//*[local-name()='Header'])"));
    }

    #[test]
    fn set_system_date_and_time_xml_contains_datetime_fields() {
        let datetime = DeviceDateAndTime {
            time_type: TimeType::Manual,
            daylight_savings: true,
            timezone: String::from("UTC&Zone"),
            utc_datetime: Some(Utc.with_ymd_and_hms(2026, 5, 24, 12, 34, 56).unwrap()),
        };
        let xml = request_xml(&SetDateAndTimeRequest {
            datetime: &datetime,
        });

        assert_xpath(&xml, "boolean(//*[local-name()='SetSystemDateAndTime'])");
        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='DateTimeType'])"),
            "Manual"
        );
        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='DaylightSavings'])"),
            "true"
        );
        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='TZ'])"),
            "UTC&Zone"
        );
        assert_xpath(&xml, "boolean(//*[local-name()='TimeZone'])");
        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='Year'])"),
            "2026"
        );
        assert_eq!(xpath_string(&xml, "string(//*[local-name()='Month'])"), "5");
        assert_eq!(xpath_string(&xml, "string(//*[local-name()='Day'])"), "24");
        assert_eq!(xpath_string(&xml, "string(//*[local-name()='Hour'])"), "12");
        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='Minute'])"),
            "34"
        );
        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='Second'])"),
            "56"
        );
    }

    #[test]
    fn set_system_date_and_time_xml_uses_ntp_schema_value() {
        let datetime = DeviceDateAndTime {
            time_type: TimeType::Ntp,
            daylight_savings: false,
            timezone: String::from("UTC"),
            utc_datetime: None,
        };
        let xml = request_xml(&SetDateAndTimeRequest {
            datetime: &datetime,
        });

        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='DateTimeType'])"),
            "NTP"
        );
    }

    #[test]
    fn ntp_xml_contains_expected_device_requests() {
        let get_xml = request_xml(&GetNtpRequest);
        assert_xpath(&get_xml, "boolean(//*[local-name()='GetNTP'])");

        let ntp_settings = NtpSettings::manual(vec![
            NetworkHost::Dns(String::from("pool.ntp.org")),
            NetworkHost::Ipv4("192.0.2.1".parse().unwrap()),
            NetworkHost::Ipv6("2001:db8::1".parse().unwrap()),
        ]);
        let set_xml = request_xml(&SetNtpRequest {
            ntp_settings: &ntp_settings,
        });

        assert_xpath(&set_xml, "boolean(//*[local-name()='SetNTP'])");
        assert_eq!(
            xpath_string(&set_xml, "string(//*[local-name()='FromDHCP'])"),
            "false"
        );
        assert_eq!(
            xpath_string(&set_xml, "count(//*[local-name()='NTPManual'])"),
            "3"
        );
        assert_eq!(
            xpath_string(
                &set_xml,
                "string((//*[local-name()='NTPManual'])[1]/*[local-name()='Type'])"
            ),
            "DNS"
        );
        assert_eq!(
            xpath_string(
                &set_xml,
                "string((//*[local-name()='NTPManual'])[1]/*[local-name()='DNSname'])"
            ),
            "pool.ntp.org"
        );
        assert_eq!(
            xpath_string(
                &set_xml,
                "string((//*[local-name()='NTPManual'])[2]/*[local-name()='Type'])"
            ),
            "IPv4"
        );
        assert_eq!(
            xpath_string(
                &set_xml,
                "string((//*[local-name()='NTPManual'])[2]/*[local-name()='IPv4Address'])"
            ),
            "192.0.2.1"
        );
        assert_eq!(
            xpath_string(
                &set_xml,
                "string((//*[local-name()='NTPManual'])[3]/*[local-name()='Type'])"
            ),
            "IPv6"
        );
        assert_eq!(
            xpath_string(
                &set_xml,
                "string((//*[local-name()='NTPManual'])[3]/*[local-name()='IPv6Address'])"
            ),
            "2001:db8::1"
        );
    }

    #[test]
    fn ntp_xml_allows_dhcp_without_manual_hosts() {
        let ntp_settings = NtpSettings::from_dhcp();
        let set_xml = request_xml(&SetNtpRequest {
            ntp_settings: &ntp_settings,
        });

        assert_eq!(
            xpath_string(&set_xml, "string(//*[local-name()='FromDHCP'])"),
            "true"
        );
        assert!(!xpath_bool(
            &set_xml,
            "boolean(//*[local-name()='NTPManual'])"
        ));
    }

    #[test]
    fn relative_move_xml_contains_profile_and_translation() {
        let xml = request_xml(&RelativeMoveRequest {
            profile_token: "profile&one",
            x: 0.25,
            y: -0.5,
            zoom: 0.75,
        });

        assert_xpath(&xml, "boolean(//*[local-name()='RelativeMove'])");
        assert_xpath(&xml, "boolean(//*[local-name()='Translation'])");
        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='ProfileToken'])"),
            "profile&one"
        );
        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='PanTilt']/@x)"),
            "0.25"
        );
        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='PanTilt']/@y)"),
            "-0.5"
        );
        assert_eq!(
            xpath_string(&xml, "string(//*[local-name()='Zoom']/@x)"),
            "0.75"
        );
    }

    #[test]
    fn continuous_absolute_and_stop_ptz_xml_shapes_are_generated() {
        let continuous_xml = request_xml(&ContinuousMoveRequest {
            profile_token: "continuous",
            x: 1.0,
            y: 0.0,
            zoom: 0.0,
            timeout: 1500.0,
        });
        assert_xpath(
            &continuous_xml,
            "boolean(//*[local-name()='ContinuousMove'])",
        );
        assert_xpath(&continuous_xml, "boolean(//*[local-name()='Velocity'])");
        assert_eq!(
            xpath_string(&continuous_xml, "string(//*[local-name()='Timeout'])"),
            "PT1.5S"
        );
        assert!(!xpath_bool(
            &continuous_xml,
            "boolean(//*[local-name()='Zoom'])"
        ));

        let absolute_xml = request_xml(&AbsoluteMoveRequest {
            profile_token: "absolute",
            x: -1.0,
            y: 1.0,
            zoom: 0.5,
        });
        assert_xpath(&absolute_xml, "boolean(//*[local-name()='AbsoluteMove'])");
        assert_xpath(&absolute_xml, "boolean(//*[local-name()='Position'])");
        assert_eq!(
            xpath_string(&absolute_xml, "string(//*[local-name()='Zoom']/@x)"),
            "0.5"
        );

        let stop_xml = request_xml(&StopRequest {
            profile_token: "stop",
        });
        assert_xpath(&stop_xml, "boolean(//*[local-name()='Stop'])");
        assert_eq!(
            xpath_string(&stop_xml, "string(//*[local-name()='ProfileToken'])"),
            "stop"
        );
    }

    #[test]
    fn parse_get_date_and_time_reads_text_values() {
        let body = br"
            <Envelope>
                <Body>
                    <GetSystemDateAndTimeResponse>
                        <SystemDateAndTime>
                            <DateTimeType>NTP</DateTimeType>
                            <DaylightSavings>false</DaylightSavings>
                            <TimeZone><TZ>UTC0</TZ></TimeZone>
                        </SystemDateAndTime>
                    </GetSystemDateAndTimeResponse>
                </Body>
            </Envelope>
        "
        .to_vec();

        let parsed = Camera::parse_get_date_and_time(body).unwrap();

        assert_eq!(parsed.time_type, TimeType::Ntp);
        assert!(!parsed.daylight_savings);
        assert_eq!(parsed.timezone, "UTC0");
        assert!(parsed.utc_datetime.is_none());
    }

    #[test]
    fn parse_get_ntp_reads_onvif_network_hosts() {
        let body = br"
            <Envelope>
                <Body>
                    <GetNTPResponse>
                        <NTPInformation>
                            <FromDHCP>true</FromDHCP>
                            <NTPFromDHCP>
                                <Type>IPv4</Type>
                                <IPv4Address>192.0.2.10</IPv4Address>
                            </NTPFromDHCP>
                            <NTPManual>
                                <Type>DNS</Type>
                                <DNSname>time.example.test</DNSname>
                            </NTPManual>
                            <NTPManual>
                                <Type>IPv6</Type>
                                <IPv6Address>2001:db8::123</IPv6Address>
                            </NTPManual>
                        </NTPInformation>
                    </GetNTPResponse>
                </Body>
            </Envelope>
        "
        .to_vec();

        let parsed = Camera::parse_get_ntp(body).unwrap();

        assert!(parsed.from_dhcp);
        assert_eq!(
            parsed.ntp_from_dhcp,
            vec![NetworkHost::Ipv4("192.0.2.10".parse().unwrap())]
        );
        assert_eq!(
            parsed.ntp_manual,
            vec![
                NetworkHost::Dns(String::from("time.example.test")),
                NetworkHost::Ipv6("2001:db8::123".parse().unwrap()),
            ]
        );
    }

    #[test]
    fn parse_get_ntp_rejects_mismatched_network_host() {
        let body = br"
            <Envelope>
                <Body>
                    <GetNTPResponse>
                        <NTPInformation>
                            <FromDHCP>false</FromDHCP>
                            <NTPManual>
                                <Type>IPv4</Type>
                                <DNSname>time.example.test</DNSname>
                            </NTPManual>
                        </NTPInformation>
                    </GetNTPResponse>
                </Body>
            </Envelope>
        "
        .to_vec();

        let err = Camera::parse_get_ntp(body).unwrap_err();

        assert!(matches!(err, Error::InvalidResponse));
    }

    #[test]
    fn ntp_type_parses_schema_casing() {
        assert!(matches!("IPv4".parse::<NtpType>(), Ok(NtpType::Ipv4)));
        assert!(matches!("IPv6".parse::<NtpType>(), Ok(NtpType::Ipv6)));
        assert!(matches!("DNS".parse::<NtpType>(), Ok(NtpType::Dns)));
        assert!(matches!(
            "IPV4".parse::<NtpType>(),
            Err(Error::InvalidArgument)
        ));
    }

    #[test]
    fn parsers_require_success_response_elements() {
        assert!(
            Camera::parse_set_date_and_time(response_xml("SetSystemDateAndTimeResponse")).is_ok()
        );
        assert!(Camera::parse_set_ntp(response_xml("SetNTPResponse")).is_ok());
        assert!(Camera::parse_relative_move(response_xml("RelativeMoveResponse")).is_ok());
        assert!(Camera::parse_continuous_move(response_xml("ContinuousMoveResponse")).is_ok());
        assert!(Camera::parse_absolute_move(response_xml("AbsoluteMoveResponse")).is_ok());
        assert!(Camera::parse_stop(response_xml("StopResponse")).is_ok());

        assert!(matches!(
            Camera::parse_set_date_and_time(response_xml("SetSystemDateAndTime")),
            Err(Error::InvalidResponse)
        ));
        assert!(matches!(
            Camera::parse_set_ntp(response_xml("GetNTPResponse")),
            Err(Error::InvalidResponse)
        ));
        assert!(matches!(
            Camera::parse_relative_move(response_xml("ContinuousMoveResponse")),
            Err(Error::InvalidResponse)
        ));
        assert!(matches!(
            Camera::parse_absolute_move(response_xml("ContinuousMoveResponse")),
            Err(Error::InvalidResponse)
        ));
        assert!(matches!(
            Camera::parse_stop(response_xml("ContinuousMoveResponse")),
            Err(Error::InvalidResponse)
        ));
    }

    #[test]
    fn parsers_map_soap_faults() {
        assert!(matches!(
            Camera::parse_set_date_and_time(soap_fault_xml("<InvalidTimeZone/>")),
            Err(Error::InvalidArgument)
        ));
        assert!(matches!(
            Camera::parse_set_ntp(soap_fault_xml("<OtherFailure/>")),
            Err(Error::SoapFault(message)) if message == "failed"
        ));
    }
}
