use std::{
    collections::HashSet,
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use axum::{Json, http::header, response::IntoResponse};
use futures_util::{StreamExt, future::join_all, stream};
use oxvif::{
    OnvifError, OnvifSession,
    soap::SoapError,
    transport::TransportError,
    types::{DeviceInfo, MediaProfile, MediaProfile2, Resolution},
};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use tracing::instrument::WithSubscriber;
use url::{Host, Url};

use crate::{
    api::UserError,
    onvif_client::{Target, device_url},
};

const DISCOVERY_TIMEOUT: Duration = Duration::from_millis(900);
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProbeRequest {
    host: String,
    port: Option<u16>,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProbeResult {
    host: String,
    port: u16,
    device_service_url: String,
    device: Option<DeviceDetails>,
    profiles: Vec<ProfileResult>,
    profile_error: Option<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeviceDetails {
    manufacturer: String,
    model: String,
    firmware_version: String,
    serial_number: String,
    hardware_id: String,
}

impl From<DeviceInfo> for DeviceDetails {
    fn from(info: DeviceInfo) -> Self {
        Self {
            manufacturer: info.manufacturer,
            model: info.model,
            firmware_version: info.firmware_version,
            serial_number: info.serial_number,
            hardware_id: info.hardware_id,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileResult {
    name: String,
    token: String,
    ptz_supported: bool,
    video: Option<VideoDetails>,
    rtsp_url: Option<String>,
    reuse_credentials: bool,
    snapshot_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoDetails {
    codec: String,
    width: u32,
    height: u32,
    frame_rate: Option<f32>,
}

impl VideoDetails {
    const fn new(codec: String, resolution: Resolution, frame_rate: Option<f32>) -> Self {
        Self {
            codec,
            width: resolution.width,
            height: resolution.height,
            frame_rate,
        }
    }
}

async fn resolve_target(input: &str) -> Result<Target, UserError> {
    let host = Host::parse(input.trim())
        .map_err(|_| invalid_host())?
        .to_string();
    let addresses = tokio::net::lookup_host((host.as_str(), 0))
        .await
        .map_err(|_| invalid_host())?
        .filter(|addr| match addr.ip() {
            IpAddr::V4(ip) => !ip.is_multicast() && !ip.is_unspecified() && !ip.is_broadcast(),
            IpAddr::V6(_) => false,
        })
        .collect::<Vec<_>>();
    if addresses.is_empty() {
        return Err(invalid_host());
    }
    Ok(Target { host, addresses })
}

const fn invalid_host() -> UserError {
    UserError::Field {
        field: "host",
        message: "Enter an IPv4 address or a hostname that resolves to one.",
    }
}

pub(super) async fn probe(
    Json(request): Json<ProbeRequest>,
) -> Result<impl IntoResponse, UserError> {
    let port = request.port.unwrap_or(80);
    if port == 0 {
        return Err(UserError::Field {
            field: "port",
            message: "Enter a port from 1 to 65535.",
        });
    }

    // oxvif traces whole SOAP messages, including WS-Security headers and
    // camera-returned URLs. Keep this request's camera traffic out of logs.
    let result = timeout(
        PROBE_TIMEOUT,
        probe_camera(request, port).with_subscriber(tracing::subscriber::NoSubscriber::default()),
    )
    .await
    .map_err(|_| UserError::Validation("The ONVIF probe timed out.".to_string()))??;
    Ok(([(header::CACHE_CONTROL, "no-store")], Json(result)))
}

async fn probe_camera(request: ProbeRequest, port: u16) -> Result<ProbeResult, UserError> {
    let target = resolve_target(&request.host).await?;
    let mut seen = HashSet::new();
    let addresses = target
        .addresses
        .iter()
        .map(SocketAddr::ip)
        .filter(|ip| seen.insert(*ip));
    let devices = join_all(addresses.map(|ip| oxvif::probe_unicast(ip, DISCOVERY_TIMEOUT))).await;
    let advertised = devices
        .into_iter()
        .flatten()
        .flat_map(|device| device.xaddrs);
    probe_endpoints(&request, &target, port, advertised).await
}

async fn probe_endpoints(
    request: &ProbeRequest,
    target: &Target,
    port: u16,
    advertised: impl Iterator<Item = String>,
) -> Result<ProbeResult, UserError> {
    let fallback = Url::parse(&device_url(&target.host, port)).map_err(|_| invalid_host())?;
    let mut seen = HashSet::new();
    let endpoints = advertised
        .filter_map(|url| Url::parse(&url).ok())
        .filter(|url| target.allows_soap(url))
        .chain(std::iter::once(fallback))
        .filter(|url| seen.insert(url.clone()));

    let mut partial_result = None;
    for endpoint in endpoints {
        let builder = target
            .session(&endpoint, &request.username, &request.password)
            .map_err(|_| UserError::InternalError("camera probe transport setup failed".into()))?;
        let session = match builder.build().await {
            Ok(session) => session,
            Err(error) if is_auth_error(&error) => return Err(authentication_error()),
            Err(_) => continue,
        };

        let (device, profiles) = tokio::join!(session.get_device_info(), media_profiles(&session));
        let mut result = ProbeResult {
            host: target.host.clone(),
            port: endpoint.port_or_known_default().unwrap_or(port),
            device_service_url: endpoint.into(),
            device: device.ok().map(DeviceDetails::from),
            profiles: Vec::new(),
            profile_error: None,
        };
        match profiles {
            Ok(profiles) => {
                result.profiles = stream::iter(profiles)
                    .map(|profile| probe_profile(&session, target, profile))
                    .buffered(8)
                    .collect()
                    .await;
                return Ok(result);
            }
            Err(error) if is_auth_error(&error) => return Err(authentication_error()),
            Err(_) => {
                result.profile_error =
                    Some("The camera responded, but its ONVIF media profiles could not be read.");
                partial_result.get_or_insert(result);
            }
        }
    }

    partial_result.ok_or_else(|| UserError::Validation(
        "Could not reach the ONVIF Device Service. Check the host, port, and camera ONVIF settings.".to_string(),
    ))
}

fn is_auth_error(error: &OnvifError) -> bool {
    match error {
        OnvifError::Transport(TransportError::HttpStatus {
            status: 401 | 403, ..
        }) => true,
        OnvifError::Soap(SoapError::Fault {
            subcode, reason, ..
        }) => {
            subcode.as_deref().is_some_and(|code| {
                matches!(
                    code.rsplit(':').next(),
                    Some("NotAuthorized" | "FailedAuthentication")
                )
            }) || reason.eq_ignore_ascii_case("Not Authorized")
        }
        _ => false,
    }
}

fn authentication_error() -> UserError {
    UserError::Validation("The camera rejected the ONVIF credentials.".to_string())
}

enum MediaProfileVersion {
    Media1(MediaProfile),
    Media2(MediaProfile2),
}

async fn media_profiles(session: &OnvifSession) -> Result<Vec<MediaProfileVersion>, OnvifError> {
    let media1_fallback = match session.get_profiles().await {
        Ok(profiles) if !profiles.is_empty() => {
            return Ok(profiles
                .into_iter()
                .map(MediaProfileVersion::Media1)
                .collect());
        }
        Err(error) if is_auth_error(&error) => return Err(error),
        Ok(_) => Ok(Vec::new()),
        Err(error) => Err(error),
    };

    match session.get_profiles_media2().await {
        Ok(profiles) => Ok(profiles
            .into_iter()
            .map(MediaProfileVersion::Media2)
            .collect()),
        Err(error) if is_auth_error(&error) => Err(error),
        Err(_) => media1_fallback,
    }
}

async fn probe_profile(
    session: &OnvifSession,
    target: &Target,
    profile: MediaProfileVersion,
) -> ProfileResult {
    let (token, name, ptz_supported, stream, snapshot, video) = match profile {
        MediaProfileVersion::Media1(profile) => {
            let (stream, snapshot, video) = tokio::join!(
                session.get_stream_uri(&profile.token),
                session.get_snapshot_uri(&profile.token),
                async {
                    let encoder = profile.video_encoder_token.as_deref()?;
                    let config = session
                        .get_video_encoder_configuration(encoder)
                        .await
                        .ok()?;
                    #[expect(
                        clippy::cast_precision_loss,
                        reason = "Preserve the probe response's f32 frame rate across Media1 and Media2"
                    )]
                    let frame_rate = config.rate_control.map(|rate| rate.frame_rate_limit as f32);
                    Some(VideoDetails::new(
                        config.encoding.to_string(),
                        config.resolution,
                        frame_rate,
                    ))
                },
            );
            (
                profile.token,
                profile.name,
                profile.ptz_config_token.is_some(),
                stream.ok().map(|uri| uri.uri),
                snapshot.ok().map(|uri| uri.uri),
                video,
            )
        }
        MediaProfileVersion::Media2(profile) => {
            let (stream, snapshot, video) = tokio::join!(
                session.get_stream_uri_media2(&profile.token),
                session.get_snapshot_uri_media2(&profile.token),
                async {
                    let encoder = profile.video_encoder_token.as_deref()?;
                    let config = session
                        .get_video_encoder_configuration_media2(encoder)
                        .await
                        .ok()?;
                    Some(VideoDetails::new(
                        config.encoding.to_string(),
                        config.resolution,
                        config.rate_control.map(|rate| rate.frame_rate_limit),
                    ))
                },
            );
            (
                profile.token,
                profile.name,
                profile.ptz_config_token.is_some(),
                stream.ok(),
                snapshot.ok(),
                video,
            )
        }
    };
    let rtsp_url = stream
        .and_then(|uri| Url::parse(&uri).ok())
        .filter(|url| matches!(url.scheme(), "rtsp" | "rtsps") && url.has_host());
    let reuse_credentials = rtsp_url.as_ref().is_some_and(|url| target.matches(url));
    let snapshot_url = snapshot
        .and_then(|uri| Url::parse(&uri).ok())
        .filter(|url| matches!(url.scheme(), "http" | "https") && url.has_host())
        .map(|mut url| {
            let _ = url.set_username("");
            let _ = url.set_password(None);
            url.into()
        });

    ProfileResult {
        token,
        name,
        ptz_supported,
        video,
        rtsp_url: rtsp_url.map(Into::into),
        reuse_credentials,
        snapshot_url,
    }
}

#[cfg(test)]
mod tests;
