use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use diqwest::{DigestAuthSession, WithDigestAuth};
use oxvif::{
    OnvifSession, OnvifSessionBuilder,
    transport::{Transport, TransportError},
};
use url::{Host, Url};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(4);
const MAX_SOAP_BYTES: usize = 2 * 1024 * 1024;

pub fn device_url(host: &str, port: u16) -> String {
    let host = if host.parse::<Ipv6Addr>().is_ok() {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    format!("http://{host}:{port}/onvif/device_service")
}

pub fn is_device_url(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
        && url.has_host()
        && url.username().is_empty()
        && url.password().is_none()
}

#[derive(Clone)]
pub struct Target {
    pub host: String,
    pub addresses: Vec<SocketAddr>,
}

impl Target {
    pub fn matches(&self, url: &Url) -> bool {
        let ip = match url.host() {
            Some(Host::Domain(host)) if host.eq_ignore_ascii_case(&self.host) => return true,
            Some(Host::Domain(host)) => host.parse().ok(),
            Some(Host::Ipv4(ip)) => Some(IpAddr::V4(ip)),
            Some(Host::Ipv6(ip)) => Some(IpAddr::V6(ip)),
            None => None,
        };
        ip.is_some_and(|ip| self.addresses.iter().any(|addr| addr.ip() == ip))
    }

    pub fn allows_soap(&self, url: &Url) -> bool {
        is_device_url(url) && self.matches(url)
    }

    pub fn session(
        &self,
        endpoint: &Url,
        username: &str,
        password: &str,
    ) -> Result<OnvifSessionBuilder, reqwest::Error> {
        let credentials =
            (!username.is_empty() || !password.is_empty()).then_some((username, password));
        let transport = CameraTransport::new(self.clone(), credentials)?;
        let builder = OnvifSession::builder(endpoint.as_str()).with_transport(Arc::new(transport));
        Ok(if let Some((username, password)) = credentials {
            builder.with_credentials(username, password)
        } else {
            builder
        })
    }
}

struct CameraTransport {
    target: Target,
    client: reqwest::Client,
    digest: Option<DigestAuthSession>,
}

impl CameraTransport {
    fn new(target: Target, credentials: Option<(&str, &str)>) -> Result<Self, reqwest::Error> {
        // Pin the requested hostname to the same addresses used for discovery.
        // Device-provided service URLs and redirects can otherwise forward the
        // SOAP credentials to a different machine.
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .resolve_to_addrs(&target.host, &target.addresses)
            .build()?;
        let digest = credentials.map(|(user, pass)| DigestAuthSession::new(user, pass));
        Ok(Self {
            target,
            client,
            digest,
        })
    }
}

#[async_trait]
impl Transport for CameraTransport {
    async fn soap_post(
        &self,
        url: &str,
        action: &str,
        body: String,
    ) -> Result<String, TransportError> {
        let Ok(url) = Url::parse(url) else {
            return Err(rejected_endpoint());
        };
        if !self.target.allows_soap(&url) {
            return Err(rejected_endpoint());
        }

        let request = self
            .client
            .post(url)
            .header(
                "Content-Type",
                format!("application/soap+xml; charset=utf-8; action=\"{action}\""),
            )
            .body(body);
        let mut response = if let Some(digest) = &self.digest {
            request
                .send_digest_auth(digest)
                .await
                .map_err(|error| match error {
                    diqwest::error::Error::Reqwest(error) => TransportError::Http(error),
                    _ => TransportError::HttpStatus {
                        status: 401,
                        body: String::new(),
                    },
                })?
        } else {
            request.send().await?
        };

        let status = response.status().as_u16();
        // ONVIF devices may return SOAP Faults with HTTP 400 or 500.
        if !matches!(status, 200 | 400 | 500) {
            return Err(TransportError::HttpStatus {
                status,
                body: String::new(),
            });
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if body.len() + chunk.len() > MAX_SOAP_BYTES {
                return Err(TransportError::HttpStatus {
                    status: 413,
                    body: "Camera SOAP response too large.".into(),
                });
            }
            body.extend_from_slice(&chunk);
        }
        Ok(String::from_utf8_lossy(&body).into_owned())
    }
}

fn rejected_endpoint() -> TransportError {
    TransportError::HttpStatus {
        status: 400,
        body: "Camera service URL is outside the requested host.".into(),
    }
}
