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
use std::str::FromStr;
use sxd_document::parser;
use sxd_xpath::{evaluate_xpath, Value};

use crate::error::Error;
use crate::util::{envelope_footer, envelope_header, soap_request};

/// An Onvif Camera is represented here
pub struct Camera {
    /// camera hostname or ip
    pub host: String,

    /// camera onvif port
    pub port: i32,

    /// camera username
    pub username: String,

    /// camera password
    pub password: String,
}

/// Ntp setting for device
#[derive(Debug)]
pub enum TimeType {
    /// indicates device should use manual configuration for clock
    Manual,

    /// indicates device should use ntp server to configure clock
    Ntp,
}

impl std::fmt::Display for TimeType {
    /// Implementing display format for the `TimeType` enum
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Self::Manual => write!(f, "Manual"),
            Self::Ntp => write!(f, "Ntp"),
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
    #[must_use]
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
#[derive(Debug, Serialize, Deserialize)]
pub enum NtpType {
    /// an ipv4 address
    Ipv4,
    /// an ipv6 address
    Ipv6,
    /// a dns hostname
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
            "IPV4" => Ok(Self::Ipv4),
            "IPV6" => Ok(Self::Ipv6),
            "DNS" => Ok(Self::Dns),
            _ => Err(Error::InvalidArgument),
        }
    }
}

/// Struct representing device ntp settings
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NtpSettings {
    /// should ntp settings come from ntp
    from_dhcp: bool,
    /// vec of ntp server names
    ntp_server_hostnames: Option<Vec<String>>,
}

/// Helper that returns Pan-Tilt and Zoom sections for ptz requests
#[must_use]
fn generate_pan_tilt_vectors(x: f32, y: f32, zoom: f32) -> String {
    let xy_element =
        format!(r#"<PanTilt x="{x}" y="{y}" xmlns="http://www.onvif.org/ver10/schema" />"#,);
    let zoom_element = if zoom == 0.0 {
        String::new()
    } else {
        format!(r#"<Zoom x="{zoom}" xmlns="http://www.onvif.org/ver10/schema" />"#,)
    };

    format!("{xy_element} {zoom_element}")
}

impl Camera {
    /// Returns url for camera
    #[must_use]
    pub fn url(&self) -> String {
        format!(
            "http://{}:{}{}",
            self.host, self.port, "/onvif/device_service",
        )
    }

    /// Perform request for date and time information from
    /// camera. Returns xml response body.
    ///
    pub async fn request_get_date_and_time(&self) -> Result<Vec<u8>, Error> {
        let request_body = r#"
  <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
            xmlns:xsd="http://www.w3.org/2001/XMLSchema">
      <GetSystemDateAndTime xmlns="http://www.onvif.org/ver10/device/wsdl"/>
    </s:Body>
  </s:Envelope>
  "#;
        soap_request(&self.url(), request_body.to_string()).await
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

        let year = evaluate_xpath(
            &doc,
            "//*[local-name()='UTCDateTime']/*[local-name()='Date']/*[local-name()='Year'][1]",
        )?
        .string()
        .parse::<i32>()?;
        let month = evaluate_xpath(
            &doc,
            "//*[local-name()='UTCDateTime']/*[local-name()='Date']/*[local-name()='Month'][1]",
        )?
        .string()
        .parse::<u32>()?;
        let day = evaluate_xpath(
            &doc,
            "//*[local-name()='UTCDateTime']/*[local-name()='Date']/*[local-name()='Day'][1]",
        )?
        .string()
        .parse::<u32>()?;

        let hour = evaluate_xpath(
            &doc,
            "//*[local-name()='UTCDateTime']/*[local-name()='Time']/*[local-name()='Hour'][1]",
        )?
        .string()
        .parse::<u32>()?;

        let minute = evaluate_xpath(
            &doc,
            "//*[local-name()='UTCDateTime']/*[local-name()='Time']/*[local-name()='Minute'][1]",
        )?
        .string()
        .parse::<u32>()?;

        let second = evaluate_xpath(
            &doc,
            "//*[local-name()='UTCDateTime']/*[local-name()='Time']/*[local-name()='Second'][1]",
        )?
        .string()
        .parse::<u32>()?;

        let camera_datetime = match Utc.with_ymd_and_hms(year, month, day, hour, minute, second) {
            chrono::LocalResult::Single(datetime) => datetime,
            chrono::LocalResult::None | chrono::LocalResult::Ambiguous(_, _) => {
                return Err(Error::InvalidArgument)
            }
        };

        let date_time_type = match evaluate_xpath(&doc, "//*[local-name()='DateTimeType'][1]")?
            .string()
            .as_ref()
        {
            "Manual" => TimeType::Manual,
            "NTP" => TimeType::Ntp,
            _ => return Err(Error::InvalidResponse),
        };

        let daylight_savings =
            evaluate_xpath(&doc, "//*[local-name()='DaylightSavings'][1]")?.boolean();

        let timezone = evaluate_xpath(&doc, "//*[local-name()='Timezone'][1]")?.string();

        Ok(DeviceDateAndTime {
            time_type: date_time_type,
            daylight_savings,
            timezone,
            utc_datetime: Some(camera_datetime),
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
        let utc_body = datetime.utc_datetime.map_or_else(String::new, |utc| {
            format!(
                r#"
            <UTCDateTime>
              <Time xmlns="http://www.onvif.org/ver10/schema">
                <Hour>{}</Hour>
                <Minute>{}</Minute>
                <Second>{}</Second>
              </Time>
              <Date xmlns="http://www.onvif.org/ver10/schema">
                <Year>{}</Year>
                <Month>{}</Month>
                <Day>{}</Day>
              </Date>
            </UTCDateTime>
            "#,
                utc.hour(),
                utc.minute(),
                utc.second(),
                utc.year(),
                utc.month(),
                utc.day()
            )
        });
        let body = format!(
            r#"
          <SetSystemDateAndTime
             xmlns="http://www.onvif.org/ver10/device/wsdl">
            <DateTimeType>{}</DateTimeType>
            <DaylightSavings>{}</DaylightSavings>
            <Timezone>
              <TZ xmlns="http://www.onvif.org/ver10/schema">{}</TZ>
            </Timezone>
            {utc_body}
          </SetSystemDateAndTime>
           "#,
            datetime.time_type, datetime.daylight_savings, datetime.timezone,
        );
        let header = match envelope_header(&self.username, &self.password) {
            Ok(h) => h,
            Err(err) => return Err(err),
        };
        let body = format!("{header}{body}{}", envelope_footer());

        soap_request(&self.url(), body).await
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
        let string_body = String::from_utf8(body)?;
        let doc = parser::parse(&string_body)?;
        let doc = doc.as_document();

        if evaluate_xpath(&doc, "//*[local-name()='SetSystemDateAndTime'][1]").is_ok() {
            return Ok(());
        }

        // The happy path failed, so let's look for errors.

        // check for ter:InvalidTimeZone
        if evaluate_xpath(&doc, "//*[local-name()='InvalidTimeZone'][1]").is_ok() {
            return Err(Error::InvalidArgument);
        }

        // check for ter:InvalidDatetime
        if evaluate_xpath(&doc, "//*[local-name()='InvalidDateTime'][1]").is_ok() {
            return Err(Error::InvalidArgument);
        }

        // check for NtpServerUndefined
        if evaluate_xpath(&doc, "//*[local-name()='NtpServerUndefined'][1]").is_ok() {
            return Err(Error::InvalidArgument);
        }

        // Otherwise unknown error
        Err(Error::InvalidResponse)
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
        {
            let body = r#"
          <GetNTP
             xmlns="http://www.onvif.org/ver10/device/wsdl">
          </GetNTP>
           "#;

            let header = match envelope_header(&self.username, &self.password) {
                Ok(h) => h,
                Err(err) => return Err(err),
            };
            let body = format!("{header}{body}{}", envelope_footer());

            soap_request(&self.url(), body).await
        }
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
        info!("{string_body}");
        let doc = parser::parse(&string_body)?;
        let doc = doc.as_document();

        let from_dhcp = evaluate_xpath(&doc, "//*[local-name()='FromDHCP'][1]")?
            .string()
            .parse::<bool>()?;

        if from_dhcp {
            match evaluate_xpath(&doc, "//*[local-name()='NTPFromDHCP'][1]")? {
                Value::Nodeset(nodes) => Ok(NtpSettings {
                    from_dhcp,
                    ntp_server_hostnames: Some(
                        nodes
                            .iter()
                            .map(|node| node.string_value().trim().to_string())
                            .collect(),
                    ),
                }),
                sxd_xpath::Value::Boolean(..)
                | sxd_xpath::Value::Number(..)
                | sxd_xpath::Value::String(..) => Err(Error::InvalidResponse),
            }
        } else {
            match evaluate_xpath(&doc, "//*[local-name()='NTPManual'][1]")? {
                Value::Nodeset(nodes) => Ok(NtpSettings {
                    from_dhcp,
                    ntp_server_hostnames: Some(
                        nodes
                            .iter()
                            .map(|node| node.string_value().trim().to_string())
                            .collect(),
                    ),
                }),
                sxd_xpath::Value::Boolean(..)
                | sxd_xpath::Value::Number(..)
                | sxd_xpath::Value::String(..) => Err(Error::InvalidResponse),
            }
        }
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
        let manual_body = format!(
            r#"
                <NTPManual>
                  <Type xmlns="http://www.onvif.org/ver10/schema">{}</Type>
                  <IPv4Address xmlns="http://www.onvif.org/ver10/schema">{}</IPv4Address>
                </NTPManual>
                "#,
            "type", "hostname"
        );
        let body = format!(
            r#"
          <SetNTP xmlns="http://www.onvif.org/ver10/device/wsdl">
            <FromDHCP>{}</FromDHCP>
            {manual_body}
          </SetNTP>
           "#,
            ntp_settings.from_dhcp,
        );

        let header = match envelope_header(&self.username, &self.password) {
            Ok(h) => h,
            Err(err) => return Err(err),
        };
        let body = format!("{header}{body}{}", envelope_footer());
        debug!("SetNTP: {body}");
        soap_request(&self.url(), body).await
    }

    /// Parse set ntp response body. Returns () on success.
    ///
    /// # Arguments
    ///
    /// * `body` - utf-8 encoded response body from set ntp call
    ///
    pub fn parse_set_ntp(body: Vec<u8>) -> Result<(), Error> {
        let string_body = String::from_utf8(body)?;
        let doc = parser::parse(&string_body)?;
        let doc = doc.as_document();
        debug!("SetNtp Response: {string_body}");
        evaluate_xpath(&doc, "//*[local-name()='SetNTPResponse'][1]")?.string();
        // If the SetNTPResponse node is present the command was a success
        Ok(())
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
        let ptz_vectors = generate_pan_tilt_vectors(x, y, zoom);
        let body = format!(
            r#"
          <RelativeMove xmlns="http://www.onvif.org/ver20/ptz/wsdl">
            <ProfileToken>{profile_token}</ProfileToken>
            <Translation>{ptz_vectors}</Translation>
          </RelativeMove>
           "#,
        );

        let header = match envelope_header(&self.username, &self.password) {
            Ok(h) => h,
            Err(err) => return Err(err),
        };
        let body = format!("{header}{body}{}", envelope_footer());
        debug!("Relative Move: {} {}", &self.url(), body);
        soap_request(&self.url(), body).await
    }

    /// parse result of relative ptz move request
    pub fn parse_relative_move(body: Vec<u8>) -> Result<(), Error> {
        let string_body = String::from_utf8(body)?;
        debug!("RelativeMove Response: {string_body}");
        let _doc = parser::parse(&string_body)?.as_document();

        Ok(())
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
    #[allow(clippy::float_arithmetic)]
    pub async fn request_continuous_move(
        &self,
        profile_token: &str,
        x: f32,
        y: f32,
        zoom: f32,
        timeout: f32,
    ) -> Result<Vec<u8>, Error> {
        let ptz_vectors = generate_pan_tilt_vectors(x, y, zoom);
        let timeout_body = if timeout == 0.0 {
            String::new()
        } else {
            format!(r"<Timeout>PT{}S</Timeout>", timeout / 1000.0)
        };
        let body = format!(
            r#"
          <ContinuousMove xmlns="http://www.onvif.org/ver20/ptz/wsdl">
            <ProfileToken>{profile_token}</ProfileToken>
            <Velocity>{ptz_vectors}</Velocity>
            {timeout_body}
          </ContinuousMove>
           "#
        );

        let header = match envelope_header(&self.username, &self.password) {
            Ok(h) => h,
            Err(err) => return Err(err),
        };
        let body = format!("{header}{body}{}", envelope_footer());
        debug!("Relative Move: {} {body}", &self.url());
        soap_request(&self.url(), body).await
    }

    /// parse result of continuous ptz move request
    ///
    /// # Error
    ///
    /// Returns Err when the body is fails to parse as xml.
    ///
    pub fn parse_continuous_move(body: Vec<u8>) -> Result<(), Error> {
        let string_body = String::from_utf8(body)?;
        debug!("ContinuousMove Response: {string_body}");
        let _doc = parser::parse(&string_body)?.as_document();

        Ok(())
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
        let ptz_vectors = generate_pan_tilt_vectors(x, y, zoom);

        let body = format!(
            r#"
          <AbsoluteMove xmlns="http://www.onvif.org/ver20/ptz/wsdl">
            <ProfileToken>{profile_token}</ProfileToken>
            <Position>{ptz_vectors}</Position>
          </AbsoluteMove>
           "#,
        );

        let header = match envelope_header(&self.username, &self.password) {
            Ok(h) => h,
            Err(err) => return Err(err),
        };
        let body = format!("{header}{body}{}", envelope_footer());
        debug!("Absolute Move: {} {body}", &self.url());
        soap_request(&self.url(), body).await
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
        Self::parse_continuous_move(res)
    }

    /// performs a stop ptz move request
    pub async fn request_stop(&self, profile_token: &str) -> Result<Vec<u8>, Error> {
        let body = format!(
            r#"
          <Stop xmlns="http://www.onvif.org/ver20/ptz/wsdl">
            <ProfileToken>{profile_token}</ProfileToken>
          </Stop>
           "#,
        );

        let header = match envelope_header(&self.username, &self.password) {
            Ok(h) => h,
            Err(err) => return Err(err),
        };
        let body = format!("{header}{body}{}", envelope_footer());
        debug!("Stop: {} {}", &self.url(), body);
        soap_request(&self.url(), body).await
    }

    /// Requests a ptz stop. Returns () on success.
    ///
    /// # Arguments
    ///
    /// * `profile_token` - ptz profile token to use for request
    ///
    pub async fn stop(&self, profile_token: &str) -> Result<(), Error> {
        let res = self.request_stop(profile_token).await?;
        Self::parse_continuous_move(res)
    }
}
