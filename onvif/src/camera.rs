//! Onvif camera api client

use chrono::offset::TimeZone;
use chrono::{DateTime, Datelike, Timelike, Utc};
use futures::future::Either;
use futures::Future;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sxd_document::parser;
use sxd_xpath::evaluate_xpath;

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
    /// Implementing display format for the TimeType enum
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TimeType::Manual => write!(f, "Manual"),
            TimeType::Ntp => write!(f, "Ntp"),
        }
    }
}

impl Serialize for TimeType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TimeType::Manual => serializer.serialize_str("Manual"),
            TimeType::Ntp => serializer.serialize_str("NTP"),
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
            "Manual" => Ok(TimeType::Manual),
            "NTP" => Ok(TimeType::Ntp),
            _ => Err(serde::de::Error::custom("invalid TimeType specified")),
        }
    }
}

/// Struct representing device date and time settings
#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceDateAndTime {
    /// specifies whether ntp is enabled for device
    pub time_type: TimeType,

    /// specified whether daylight saving time is enabled for device
    pub daylight_savings: bool,

    /// time for device in POSIX format
    pub timezone: String,

    /// utc time for device
    pub utc_datetime: DateTime<Utc>,
}

impl DeviceDateAndTime {
    /// Returns new DeviceDateAndTime struct
    pub fn new(time: DateTime<Utc>) -> Self {
        Self {
            time_type: TimeType::Manual,
            daylight_savings: false,
            timezone: String::from("UTC"),
            utc_datetime: time,
        }
    }
}

impl Camera {
    /// Returns url for camera
    pub fn url(&self) -> String {
        format!(
            "http://{}:{}{}",
            self.host,
            self.port.to_string(),
            "/onvif/device_service",
        )
    }

    /// Returns unparsed body of get date and time request
    pub fn request_get_date_and_time(&self) -> impl Future<Item = Vec<u8>, Error = Error> {
        let request_body = r#"
  <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
            xmlns:xsd="http://www.w3.org/2001/XMLSchema">
      <GetSystemDateAndTime xmlns="http://www.onvif.org/ver10/device/wsdl"/>
    </s:Body>
  </s:Envelope>
  "#;
        soap_request(&self.url(), request_body.to_string())
    }

    /// Returns parsed date and time body
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

        let camera_datetime = Utc.ymd(year, month, day).and_hms(hour, minute, second);

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
            utc_datetime: camera_datetime,
        })
    }

    /// Requests date and time settings from camera
    pub fn get_date_and_time(&self) -> Box<Future<Item = DeviceDateAndTime, Error = Error>> {
        Box::new(
            self.request_get_date_and_time()
                .and_then(Self::parse_get_date_and_time)
                .map_err(|_err| Error::ConnectionFailed),
        )
    }

    /// Submits set_date_and_time call and returns the raw result as a Future.
    ///
    /// # Arguments
    ///
    /// * `datetime` - date and time configuration to send to camera
    ///
    pub fn request_set_date_and_time(
        &self,
        datetime: &DeviceDateAndTime,
    ) -> impl Future<Item = Vec<u8>, Error = Error> {
        let body = format!(
            r#"
          <SetSystemDateAndTime
             xmlns="http://www.onvif.org/ver10/device/wsdl">
            <DateTimeType>{}</DateTimeType>
            <DaylightSavings>{}</DaylightSavings>
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
          </SetSystemDateAndTime>
           "#,
            datetime.time_type,
            datetime.daylight_savings,
            datetime.utc_datetime.hour(),
            datetime.utc_datetime.minute(),
            datetime.utc_datetime.second(),
            datetime.utc_datetime.year(),
            datetime.utc_datetime.month(),
            datetime.utc_datetime.day(),
        );
        let header = match envelope_header(&self.username, &self.password) {
            Ok(h) => h,
            Err(err) => return Either::A(futures::future::err(err)),
        };
        let body = format!("{}{}{}", header, body, envelope_footer());

        Either::B(soap_request(&self.url(), body))
    }

    /// Returns nothing if the parsed body represents a successful
    /// call and an Error otherwise.
    ///
    /// # Arguments
    ///
    /// * `body` - result of set_date_and_time request
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
    pub fn set_date_and_time(
        &self,
        datetime: &DeviceDateAndTime,
    ) -> Box<Future<Item = (), Error = Error>> {
        Box::new(
            self.request_set_date_and_time(datetime)
                .and_then(Self::parse_set_date_and_time)
                .map_err(|_err| Error::ConnectionFailed),
        )
    }
}
