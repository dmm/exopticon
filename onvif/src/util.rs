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

//! Onvif api utilities
use std::time::Duration;

use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Request, StatusCode, Uri};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use tokio::time::timeout;

use crate::error::Error;

type SoapRequestBody = Full<Bytes>;

/// HTTP client used for SOAP requests.
pub type SoapClient = Client<HttpsConnector<HttpConnector>, SoapRequestBody>;

/// Builds an HTTP/HTTPS client for SOAP requests.
pub fn build_soap_client() -> Result<SoapClient, Error> {
    let connector = HttpsConnectorBuilder::new()
        .with_native_roots()?
        .https_or_http()
        .enable_http1()
        .build();

    Ok(Client::builder(TokioExecutor::new()).build(connector))
}

/// Returns Future resolving to a response Result
///
/// # Arguments
///
/// * `url` - a url to submit to request to
/// * `body` - request body
///
pub async fn soap_request(
    client: &SoapClient,
    url: &Uri,
    body: String,
    request_timeout: Duration,
) -> Result<Vec<u8>, Error> {
    let Ok(req) = Request::builder()
        .method("POST")
        .uri(url.clone())
        .header("Content-Type", "application/soap+xml")
        .body(Full::from(body))
    else {
        return Err(Error::InvalidArgument);
    };

    timeout(request_timeout, async {
        let response = client.request(req).await?;
        let status = response.status();

        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(Error::Unauthorized);
        }

        if !status.is_success() {
            return Err(Error::HttpStatus(status.as_u16()));
        }

        let body = response.into_body().collect().await?;
        Ok(body.to_bytes().to_vec())
    })
    .await
    .map_err(|_err| Error::Timeout)?
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::{build_soap_client, soap_request};
    use crate::error::Error;

    async fn serve_once(response: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0; 1024];
            let _bytes = stream.read(&mut buf).await.unwrap();
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        format!("http://{addr}/onvif/device_service")
    }

    async fn serve_timeout() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_millis(200)).await;
        });

        format!("http://{addr}/onvif/device_service")
    }

    #[tokio::test]
    async fn soap_request_returns_success_body() {
        let url = serve_once("HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\npong")
            .await
            .parse()
            .unwrap();
        let client = build_soap_client().unwrap();

        let body = soap_request(&client, &url, String::from("<s/>"), Duration::from_secs(1))
            .await
            .unwrap();

        assert_eq!(body, b"pong");
    }

    #[tokio::test]
    async fn soap_request_maps_unauthorized_status() {
        let url = serve_once("HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n")
            .await
            .parse()
            .unwrap();
        let client = build_soap_client().unwrap();

        let err = soap_request(&client, &url, String::from("<s/>"), Duration::from_secs(1))
            .await
            .unwrap_err();

        assert!(matches!(err, Error::Unauthorized));
    }

    #[tokio::test]
    async fn soap_request_maps_server_error_status() {
        let url = serve_once("HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n")
            .await
            .parse()
            .unwrap();
        let client = build_soap_client().unwrap();

        let err = soap_request(&client, &url, String::from("<s/>"), Duration::from_secs(1))
            .await
            .unwrap_err();

        assert!(matches!(err, Error::HttpStatus(500)));
    }

    #[tokio::test]
    async fn soap_request_times_out() {
        let url = serve_timeout().await.parse().unwrap();
        let client = build_soap_client().unwrap();

        let err = soap_request(
            &client,
            &url,
            String::from("<s/>"),
            Duration::from_millis(20),
        )
        .await
        .unwrap_err();

        assert!(matches!(err, Error::Timeout));
    }
}
