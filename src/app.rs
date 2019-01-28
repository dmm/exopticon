// app.rs
use std::env;

use crate::actix::prelude::*;
use crate::actix_web::middleware::identity::{CookieIdentityPolicy, IdentityService};
use crate::actix_web::{
    http::Method, middleware::Logger, ws, App, Error, HttpRequest, HttpResponse,
};
use crate::auth_routes::{login, logout, AuthMiddleware, WebAuthMiddleware};
use crate::camera_group_routes::{
    create_camera_group, fetch_all_camera_groups, fetch_camera_group, update_camera_group,
};
use crate::camera_routes::{
    create_camera, discover, fetch_all_cameras, fetch_camera, fetch_time, set_time, update_camera,
};
use crate::chrono::Duration;

use crate::models::DbExecutor;
use crate::static_routes;
use crate::static_routes::{fetch_static_file, index};
use crate::user_routes::create_user;
use crate::video_unit_routes::{fetch_video_unit, fetch_video_units_between};
use crate::ws_session::{WsSerialization, WsSession};

pub struct AppState {
    pub db: Addr<DbExecutor>,
}

pub fn ws_route(req: &HttpRequest<AppState>) -> Result<HttpResponse, Error> {
    debug!("Starting websocket session...");
    ws::start(req, WsSession::new(WsSerialization::MsgPack))
}

pub fn ws_json_route(req: &HttpRequest<AppState>) -> Result<HttpResponse, Error> {
    debug!("Starting json websocket session...");
    ws::start(req, WsSession::new(WsSerialization::Json))
}

// helper function to create and returns the app after mounting all routes/resources
pub fn create_app(db: Addr<DbExecutor>) -> App<AppState> {
    // secret is a random 32 character long base 64 string
    let secret: String = env::var("SECRET_KEY").unwrap_or_else(|_| "0".repeat(32));

    App::with_state(AppState { db })
        // setup builtin logger to get nice logging for each request
        .middleware(Logger::default())
        .middleware(IdentityService::new(
            CookieIdentityPolicy::new(secret.as_bytes())
                .name("id")
                .path("/")
                //                .domain(domain.as_str())
                .max_age(Duration::days(1)) // just for testing
                .secure(false),
        ))
        .resource("/login", |r| {
            r.method(Method::GET).with(static_routes::login);
        })
        // routes for authentication
        .resource("/auth", |r| {
            r.method(Method::POST).with(login);
            r.method(Method::DELETE).with(logout);
        })
        // routes for static files
        .resource("/", |r| {
            r.middleware(WebAuthMiddleware);
            r.method(Method::GET).with(index);
        })
        .scope("/static/", |s| {
            s.middleware(WebAuthMiddleware)
                .handler("/", fetch_static_file)
        })
        // v1 api scope
        .scope("/v1", |s| {
            s.middleware(AuthMiddleware)
                .resource("/ws", |r| r.route().f(ws_route))
                .resource("/ws_json", |r| r.route().f(ws_json_route))
                // routes to camera_group
                .resource("/camera_groups", |r| {
                    r.method(Method::POST).with(create_camera_group);
                    r.method(Method::GET).with(fetch_all_camera_groups);
                })
                .resource("/camera_groups/{id}", |r| {
                    r.method(Method::POST).with(update_camera_group);
                    r.method(Method::GET).with(fetch_camera_group);
                })
                // routes to camera
                .resource("/cameras", |r| {
                    r.method(Method::POST).with(create_camera);
                    r.method(Method::GET).with(fetch_all_cameras);
                })
                .resource("/cameras/discover", |r| {
                    r.method(Method::GET).with(discover)
                })
                .resource("/cameras/{id}", |r| {
                    r.method(Method::POST).with(update_camera);
                    r.method(Method::GET).with(fetch_camera);
                })
                .resource("/cameras/{id}/time", |r| {
                    r.method(Method::GET).with(fetch_time);
                    r.method(Method::POST).with(set_time);
                })
                // routes to video_unit
                .resource("/video_units/{id}", |r| {
                    r.method(Method::GET).with(fetch_video_unit);
                })
                .resource("/video_units/between/{begin}/{end}", |r| {
                    r.method(Method::GET).with(fetch_video_units_between);
                })
                // routes to user
                .resource("/users", |r| {
                    r.method(Method::POST).with(create_user);
                    //            r.method(Method::GET).with(fetch_all_users);
                })
        })
}
