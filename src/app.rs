// app.rs

use actix::Addr;
//use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;

use crate::analysis_routes::create_analysis_engine;
use crate::auth_routes::{login, logout, Auth, WebAuth};
use crate::camera_group_routes::{
    create_camera_group, fetch_all_camera_groups, fetch_camera_group, update_camera_group,
};
use crate::camera_routes::{
    create_camera, discover, fetch_all_cameras, fetch_camera, fetch_ntp, fetch_time, ptz_direction,
    ptz_relative, set_ntp, set_time, update_camera,
};
use crate::models::DbExecutor;
use crate::observation_routes::fetch_observations_between;
use crate::static_routes;
use crate::static_routes::index;
use crate::user_routes::create_user;
use crate::video_unit_routes::{fetch_video_unit, fetch_video_units_between};
use crate::ws_session::{WsSerialization, WsSession};

/// Struct representing main application state
pub struct RouteState {
    /// address of database actor
    pub db: Addr<DbExecutor>,
}

/// Route to return a websocket session using messagepack serialization
pub fn ws_route(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    debug!("Starting websocket session...");
    ws::start(WsSession::new(WsSerialization::MsgPack), &req, stream)
}

/// Route to return a websocket session using json serialization
pub fn ws_json_route(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    debug!("Starting json websocket session...");
    ws::start(WsSession::new(WsSerialization::Json), &req, stream)
}

/// helper function to create and returns the app after mounting all routes/resources
pub fn generate_config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/login").route(web::get().to_async(static_routes::login)))
        // routes for authentication
        .service(
            web::resource("/auth")
                .route(web::post().to_async(login))
                .route(web::delete().to_async(logout)),
        )
        // routes for static files
        //        .default_service(web::route(web::get().to_async(index)).wrap(WebAuth))
        .service(
            web::resource("/index.html")
                .wrap(WebAuth)
                .route(web::get().to(index)),
        )
        .service(
            web::resource("/{script}.js")
                .wrap(WebAuth)
                .route(web::get().to_async(static_routes::get_js_file)),
        )
        .service(
            web::resource("/{scriptmap}.js.map")
                .wrap(WebAuth)
                .route(web::get().to_async(static_routes::get_js_map_file)),
        )
        .service(
            web::resource("/{stylesheet}.css")
                .wrap(WebAuth)
                .route(web::get().to_async(static_routes::get_css_file)),
        )
        // v1 api scope
        .service(
            web::scope("/v1")
                .wrap(Auth)
                .service(web::resource("/ws").to(ws_route))
                .service(web::resource("/ws_json").to(ws_json_route))
                // routes to camera_group
                .service(
                    web::resource("/camera_groups")
                        .route(web::post().to_async(create_camera_group))
                        .route(web::get().to_async(fetch_all_camera_groups)),
                )
                .service(
                    web::resource("/camera_groups/{id}")
                        .route(web::post().to_async(update_camera_group))
                        .route(web::get().to_async(fetch_camera_group)),
                )
                // routes to camera
                .service(
                    web::resource("/cameras")
                        .route(web::post().to_async(create_camera))
                        .route(web::get().to_async(fetch_all_cameras)),
                )
                .service(web::resource("/cameras/discover").route(web::get().to_async(discover)))
                .service(
                    web::resource("/cameras{id}")
                        .route(web::post().to_async(update_camera))
                        .route(web::get().to_async(fetch_camera)),
                )
                .service(
                    web::resource("/cameras/{id}/time")
                        .route(web::post().to_async(fetch_time))
                        .route(web::get().to_async(set_time)),
                )
                .service(
                    web::resource("/cameras/{id}/ntp")
                        .route(web::get())
                        .to_async(fetch_ntp)
                        .route(web::post())
                        .to_async(set_ntp),
                )
                .service(
                    web::resource("/cameras/{id}/ptz/relative")
                        .route(web::post().to_async(ptz_relative)),
                )
                .service(
                    web::resource("/cameras/{id}/ptz/{direction}")
                        .route(web::post().to_async(ptz_direction)),
                )
                .service(
                    web::resource("/cameras/{camera_id}/video")
                        .route(web::get().to_async(fetch_video_units_between)),
                )
                .service(
                    web::resource("/cameras/{camera_id}/observations")
                        .route(web::get().to_async(fetch_observations_between)),
                )
                // routes to video_unit
                .service(
                    web::resource("/video_units/{id}").route(web::get().to_async(fetch_video_unit)),
                )
                // routes to user
                .service(web::resource("/users").route(web::post().to_async(create_user)))
                // routes to analysis_engine
                .service(
                    web::resource("/analysis_engines")
                        .route(web::post().to_async(create_analysis_engine)),
                ),
        )
        // Create default route
        .service(
            web::resource("/{tail:.*}")
                .wrap(WebAuth)
                .route(web::get().to(index)),
        );
}
