use std::path::PathBuf;

use actix_web::dev::FromParam;
use actix_web::{Body, HttpRequest, HttpResponse, Responder};
use askama::Template;
use mime_guess::guess_mime_type;

use crate::app::AppState;

#[derive(Template)]
#[template(path = "index.html")]
struct Index;

pub fn index(_req: HttpRequest<AppState>) -> HttpResponse {
    let s = Index.render().unwrap();

    HttpResponse::Ok().content_type("text/html").body(s)
}

#[derive(RustEmbed)]
#[folder = "web/static"]
struct Asset;

pub fn fetch_static_file(req: &HttpRequest<AppState>) -> impl Responder {
    let tail: String = match req.match_info().query("tail") {
        Ok(t) => t,
        Err(_e) => return HttpResponse::NotFound().body("404 Not Found"),
    };

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
