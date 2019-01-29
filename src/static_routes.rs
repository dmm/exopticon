use std::path::PathBuf;

use actix_web::dev::FromParam;
use actix_web::{Body, HttpRequest, HttpResponse};
use askama::Template;
use mime_guess::guess_mime_type;

use crate::app::AppState;

pub fn index(_req: HttpRequest<AppState>) -> HttpResponse {
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

pub fn login(_req: HttpRequest<AppState>) -> HttpResponse {
    let s = Login.render().unwrap();

    HttpResponse::Ok().content_type("text/html").body(s)
}

pub fn get_js_file(req: HttpRequest<AppState>) -> HttpResponse {
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

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Asset;

pub fn fetch_static_file(req: &HttpRequest<AppState>) -> HttpResponse {
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
