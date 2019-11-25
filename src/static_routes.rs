// Allow undocumented structs in this module
#![allow(clippy::missing_docs_in_private_items)]
// These functions pass-by-value because that's the interface
// implemented by actix-web.
#![allow(clippy::needless_pass_by_value)]

use actix_web::{body::Body, web::Path, HttpRequest, HttpResponse};
use askama::Template;

/// Fetches static index file, returns `HttpResponse`
pub fn index(_req: HttpRequest) -> HttpResponse {
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
pub fn login(_req: HttpRequest) -> HttpResponse {
    let s = Login
        .render()
        .expect("unabled to file login page, build is broken?");

    HttpResponse::Ok().content_type("text/html").body(s)
}

/// Fetches specified static js file and resturns `HttpResponse`
pub fn get_js_file(script: Path<String>) -> HttpResponse {
    let path = format!("{}.js", script);

    info!("javascript path: {}", path);
    match Asset::get(&path) {
        Some(content) => HttpResponse::Ok()
            .content_type("application/javascript")
            .body(Body::from_slice(content.as_ref())),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

/// Fetches js map file and returns `HttpResponse`
pub fn get_js_map_file(map: Path<String>) -> HttpResponse {
    let path = format!("{}.js.map", map);
    match Asset::get(&path) {
        Some(content) => HttpResponse::Ok()
            .content_type("application/json")
            .body(Body::from_slice(content.as_ref())),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

/// Returns static css file and returns `HttpResponse`
pub fn get_css_file(css: Path<String>) -> HttpResponse {
    let path = format!("{}.css", css);
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
/*
/// Returns `HttpResponse` with specified static file or error.
///
/// # Arguments
/// `req` - file request
///
pub fn fetch_static_file(req: HttpRequest) -> HttpResponse {
    let tail: String = match req.match_info().query("tail") {
        Ok(t) => t,
        Err(_e) => return HttpResponse::NotFound().body("404 Not Found"),
    };
    info!("Static path: {}", tail);
    let relpath = match PathBuf::from_param(tail.trim_start_matches('/')) {
        Ok(r) => r,
        Err(_e) => return HttpResponse::NotFound().body("404 Not Found"),
    };

    let path = match relpath.to_str() {
        Some(p) => p,
        None => return HttpResponse::NotFound().body("404 Not Found"),
    };

    match Asset::get(path) {
        Some(content) => HttpResponse::Ok()
            .content_type(mime_guess::from_path(path).first_or_octet_stream().as_ref())
            .body(Body::from_slice(content.as_ref())),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}
*/
