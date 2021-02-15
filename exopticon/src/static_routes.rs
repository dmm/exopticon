/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

// Allow undocumented structs in this module
#![allow(clippy::missing_docs_in_private_items)]
// These functions pass-by-value because that's the interface
// implemented by actix-web.
#![allow(clippy::needless_pass_by_value)]

use std::collections::BTreeMap;
use std::env;
use std::str::from_utf8;

use actix_web::{body::Body, web::Path, HttpRequest, HttpResponse};
use handlebars::Handlebars;

/// Fetches static index file, returns `HttpResponse`
pub fn index(_req: HttpRequest) -> HttpResponse {
    match Asset::get("index.html") {
        Some(content) => HttpResponse::Ok()
            .content_type("text/html")
            .body(Body::from_slice(content.as_ref())),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Asset;

/// Returns `HttpResponse` with specified static file or error.
///
/// # Arguments
/// `req` - file request
///
pub fn fetch_static_file(tail: Path<String>) -> HttpResponse {
    debug!("Fetching static file: {}", tail);

    let path = tail.into_inner();

    match Asset::get(&path) {
        Some(content) => HttpResponse::Ok()
            .content_type(mime_guess::from_path(path).first_or_octet_stream().as_ref())
            .body(Body::from_slice(content.as_ref())),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

/// Returns `HttpResponse` with templated webmanifest
///
/// # Arguments
/// `req` - file request
///
pub fn fetch_webmanifest(_req: HttpRequest) -> HttpResponse {
    error!("Calling fetch_webmanifest!");
    let handlebars = Handlebars::new();
    let mut data = BTreeMap::new();

    data.insert(
        "name",
        env::var("EXOPTICON_NAME").unwrap_or_else(|_| "Exopticon".to_string()),
    );
    data.insert(
        "short_name",
        env::var("EXOPTICON_SHORT_NAME").unwrap_or_else(|_| "Exopticon".to_string()),
    );

    match Asset::get("manifest.webmanifest") {
        Some(content) => {
            let template = if let Ok(templ) = from_utf8(&content) {
                templ
            } else {
                error!("Failed to parse webmanifest template. Invalid utf8!");
                return HttpResponse::InternalServerError().finish();
            };
            let manifest = if let Ok(manifest) = handlebars.render_template(template, &data) {
                manifest
            } else {
                error!("Failed to render webmanifest template!");
                return HttpResponse::InternalServerError().finish();
            };

            HttpResponse::Ok()
                .content_type("application/json")
                .body(Body::from_slice(manifest.as_ref()))
        }

        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}
