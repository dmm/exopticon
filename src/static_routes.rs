// Allow undocumented structs in this module
#![allow(clippy::missing_docs_in_private_items)]
// These functions pass-by-value because that's the interface
// implemented by actix-web.
#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;

use actix_web::dev::FromParam;
use actix_web::{Body, HttpRequest, HttpResponse};
use askama::Template;
use mime_guess::guess_mime_type;

use crate::app::RouteState;

/// Fetches static index file, returns `HttpResponse`
pub fn index(_req: HttpRequest<RouteState>) -> HttpResponse {
    match Asset::get("index.html") {
        Some(content) => HttpResponse::Ok()
            .content_type("text/html")
            .body(Body::from_slice(content.as_ref())),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct Login;

/// Fetches login page and returns `HttpResponse`
pub fn login(_req: HttpRequest<RouteState>) -> HttpResponse {
    let s = Login
        .render()
        .expect("unabled to file login page, build is broken?");

    HttpResponse::Ok().content_type("text/html").body(s)
}

/// Fetches specified static js file and resturns `HttpResponse`
pub fn get_js_file(req: HttpRequest<RouteState>) -> HttpResponse {
    let filename: String = match req.match_info().query("script") {
        Ok(t) => t,
        Err(_e) => return HttpResponse::NotFound().body("js file not found"),
    };

    info!("javascript filename: {}", filename);
    let path = format!("{}.js", filename);
    info!("javascript path: {}", path);
    match Asset::get(&path) {
        Some(content) => HttpResponse::Ok()
            .content_type("application/javascript")
            .body(Body::from_slice(content.as_ref())),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

/// Fetches js map file and returns `HttpResponse`
pub fn get_js_map_file(req: HttpRequest<RouteState>) -> HttpResponse {
    let filename: String = match req.match_info().query("scriptmap") {
        Ok(t) => t,
        Err(_e) => return HttpResponse::NotFound().body("js map file not found"),
    };

    let path = format!("{}.js.map", filename);
    match Asset::get(&path) {
        Some(content) => HttpResponse::Ok()
            .content_type("application/octet-stream")
            .body(Body::from_slice(content.as_ref())),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

/// Returns static css file and returns `HttpResponse`
pub fn get_css_file(req: HttpRequest<RouteState>) -> HttpResponse {
    let filename: String = match req.match_info().query("stylesheet") {
        Ok(t) => t,
        Err(_e) => return HttpResponse::NotFound().body("css file not found"),
    };

    let path = format!("{}.css", filename);
    match Asset::get(&path) {
        Some(content) => HttpResponse::Ok()
            .content_type("text/css")
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
pub fn fetch_static_file(req: &HttpRequest<RouteState>) -> HttpResponse {
    let tail: String = match req.match_info().query("tail") {
        Ok(t) => t,
        Err(_e) => return HttpResponse::NotFound().body("404 Not Found"),
    };
    info!("Static path: {}", tail);
    let relpath = match PathBuf::from_param(tail.trim_left_matches('/')) {
        Ok(r) => r,
        Err(_e) => return HttpResponse::NotFound().body("404 Not Found"),
    };

    let path = match relpath.to_str() {
        Some(p) => p,
        None => return HttpResponse::NotFound().body("404 Not Found"),
    };

    match Asset::get(path) {
        Some(content) => HttpResponse::Ok()
            .content_type(guess_mime_type(path).as_ref())
            .body(Body::from_slice(content.as_ref())),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}
