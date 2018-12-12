// app.rs
use crate::actix::prelude::*;
use crate::actix_web::middleware::identity::{CookieIdentityPolicy, IdentityService};
use crate::actix_web::{
    http::Method, middleware::Logger, ws, App, Error, HttpRequest, HttpResponse,
};
use crate::auth_routes::{login, logout};
use crate::camera_group_routes::{
    create_camera_group, fetch_all_camera_groups, fetch_camera_group, update_camera_group,
};
use crate::camera_routes::{create_camera, fetch_all_cameras, fetch_camera, update_camera};
use crate::chrono::Duration;
use crate::models::DbExecutor;
use crate::static_routes::{fetch_static_file, index};
use crate::user_routes::create_user;
use crate::video_unit_routes::{fetch_video_unit, fetch_video_units_between};
use crate::ws_session::WsSession;

pub struct AppState {
    pub db: Addr<DbExecutor>,
}

pub fn ws_route(req: &HttpRequest<AppState>) -> Result<HttpResponse, Error> {
    debug!("Starting websocket session...");
    ws::start(req, WsSession::default())
}

// helper function to create and returns the app after mounting all routes/resources
pub fn create_app(db: Addr<DbExecutor>) -> App<AppState> {
    // secret is a random 32 character long base 64 string
    let secret: String = std::env::var("SECRET_KEY").unwrap_or_else(|_| "0".repeat(32));
    let domain: String = std::env::var("DOMAIN").unwrap_or_else(|_| "localhost".to_string());

    App::with_state(AppState { db })
        // setup builtin logger to get nice logging for each request
        .middleware(Logger::default())
        .middleware(IdentityService::new(
            CookieIdentityPolicy::new(secret.as_bytes())
                .name("id")
                .path("/")
                .domain(domain.as_str())
                .max_age(Duration::days(1)) // just for testing
                .secure(false),
        ))
        // routes for static files
        .route("/", Method::GET, index)
        .handler("/static/", fetch_static_file)
        .resource("/ws", |r| r.route().f(ws_route))
        // routes for authentication
        .resource("/auth", |r| {
            r.method(Method::POST).with(login);
            r.method(Method::DELETE).with(logout);
        })
        // routes to camera_group
        .resource("/v1/camera_groups", |r| {
            r.method(Method::POST).with(create_camera_group);
            r.method(Method::GET).with(fetch_all_camera_groups);
        })
        .resource("/v1/camera_groups/{id}", |r| {
            r.method(Method::POST).with(update_camera_group);
            r.method(Method::GET).with(fetch_camera_group);
        })
        // routes to camera
        .resource("/v1/cameras", |r| {
            r.method(Method::POST).with(create_camera);
            r.method(Method::GET).with(fetch_all_cameras);
        })
        .resource("/v1/cameras/{id}", |r| {
            r.method(Method::POST).with(update_camera);
            r.method(Method::GET).with(fetch_camera);
        })
        // routes to video_unit
        .resource("/v1/video_units/{id}", |r| {
            r.method(Method::GET).with(fetch_video_unit);
        })
        .resource("/v1/video_units/between/{begin}/{end}", |r| {
            r.method(Method::GET).with(fetch_video_units_between);
        })
        // routes to user
        .resource("/v1/users", |r| {
            r.method(Method::POST).with(create_user);
            //            r.method(Method::GET).with(fetch_all_users);
        })
}
