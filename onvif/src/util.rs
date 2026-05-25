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
use hyper::{Body, Client, Request};
use tokio_stream::StreamExt;

use crate::error::Error;

/// Returns Future resolving to a response Result
///
/// # Arguments
///
/// * `url` - a url to submit to request to
/// * `body` - request body
///
pub async fn soap_request(url: &str, body: String) -> Result<Vec<u8>, Error> {
    let client = Client::new();

    let url: hyper::Uri = match url.parse() {
        Ok(u) => u,
        Err(_) => return Err(Error::InvalidArgument),
    };

    let Ok(req) = Request::builder()
        .method("POST")
        .uri(url)
        .header("Content-Type", "application/soap+xml")
        .body(Body::from(body))
    else {
        return Err(Error::InvalidArgument);
    };

    let mut response = client.request(req).await?;
    let body = response.body_mut();
    let mut output = Vec::new();

    while let Some(chunk) = body.next().await {
        let bytes = chunk?;
        output.extend(&bytes[..]);
    }

    Ok(output)
}
