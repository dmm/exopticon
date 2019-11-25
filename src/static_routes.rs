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

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Asset;

/// Returns `HttpResponse` with specified static file or error.
///
/// # Arguments
/// `req` - file request
///
pub fn fetch_static_file(tail: Path<String>) -> HttpResponse {
    info!("Static path: {}", tail);

    let path = tail.into_inner();

    match Asset::get(&path) {
        Some(content) => HttpResponse::Ok()
            .content_type(mime_guess::from_path(path).first_or_octet_stream().as_ref())
            .body(Body::from_slice(content.as_ref())),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}
