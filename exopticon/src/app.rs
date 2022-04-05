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

// app.rs

use actix::Addr;
//use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;

use crate::alert_rule_routes::{create_alert_rule, fetch_all_alert_rules};
use crate::analysis_routes::{
    create_analysis_engine, create_analysis_instance, delete_analysis_engine,
    delete_analysis_instance, fetch_analysis_configuration, fetch_analysis_engine,
    fetch_analysis_instance, update_analysis_configuration, update_analysis_engine,
    update_analysis_instance,
};
use crate::auth_routes::{
    check_login, create_personal_access_token, delete_personal_access_token,
    fetch_personal_access_tokens, login, logout, Auth, WebAuth,
};
use crate::camera_group_routes::{
    create_camera_group, fetch_all_camera_groups, fetch_camera_group, update_camera_group,
};
use crate::camera_routes::{
    create_camera, discover, fetch_all_cameras, fetch_camera, fetch_ntp, fetch_time, ptz_direction,
    ptz_relative, set_ntp, set_time, update_camera,
};
use crate::models::DbExecutor;
use crate::observation_routes::{
    fetch_event_clip, fetch_event_snapshot, fetch_events, fetch_observation,
    fetch_observation_clip, fetch_observation_snapshot, fetch_observations_between,
};
use crate::static_routes;
use crate::static_routes::index;
use crate::user_routes::{create_user, fetch_current_user};
use crate::video_unit_routes::{fetch_video_unit, fetch_video_units_between};
use crate::ws_session::{WsSerialization, WsSession};

/// Struct representing main application state
pub struct RouteState {
    /// address of database actor
    pub db: Addr<DbExecutor>,
}

// /// We have to pass by value to satisfy the actix route interface.
#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::unused_async)]
/// Route to return a websocket session using messagepack serialization
pub async fn ws_route(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    debug!("Starting websocket session...");
    ws::start(WsSession::new(WsSerialization::MsgPack), &req, stream)
}

/// We have to pass by value to satisfy the actix route interface.
#[allow(clippy::needless_pass_by_value)]
#[allow(clippy::unused_async)]
/// Route to return a websocket session using json serialization
pub async fn ws_json_route(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    debug!("Starting json websocket session...");
    ws::start(WsSession::new(WsSerialization::Json), &req, stream)
}

/// helper function to create and returns the app after mounting all routes/resources
#[allow(clippy::too_many_lines)]
pub fn generate_config(cfg: &mut web::ServiceConfig) {
    cfg
        // routes for authentication
        .service(
            web::resource("/auth")
                .route(web::post().to(login))
                .route(web::get().to(check_login))
                .route(web::delete().to(logout)),
        )
        .service(web::resource("/index.html").route(web::get().to(index)))
        .service(
            web::resource("/{file:[^/]+(.js|.js.map|.css)}")
                .route(web::get().to(static_routes::fetch_static_file)),
        )
        .service(
            web::resource("/manifest.webmanifest")
                .route(web::get().to(static_routes::fetch_webmanifest)),
        )
        .service(
            web::resource("/static/{tail:.*}")
                .route(web::get().to(static_routes::fetch_static_file)),
        )
        // v1 api scope
        .service(
            web::scope("/v1")
                .wrap(Auth)
                .service(web::resource("/ws").route(web::get()).to(ws_route))
                .service(web::resource("/ws_json").route(web::get().to(ws_json_route)))
                // personal access token routes
                .service(
                    web::resource("/personal_access_tokens")
                        .route(web::get().to(fetch_personal_access_tokens))
                        .route(web::post().to(create_personal_access_token)),
                )
                .service(
                    web::resource("/personal_access_tokens/{id}")
                        .route(web::delete().to(delete_personal_access_token)),
                )
                // routes to camera_group
                .service(
                    web::resource("/camera_groups")
                        .route(web::post().to(create_camera_group))
                        .route(web::get().to(fetch_all_camera_groups)),
                )
                .service(
                    web::resource("/camera_groups/{id}")
                        .route(web::post().to(update_camera_group))
                        .route(web::get().to(fetch_camera_group)),
                )
                // routes to camera
                .service(
                    web::resource("/cameras")
                        .route(web::post().to(create_camera))
                        .route(web::get().to(fetch_all_cameras)),
                )
                .service(web::resource("/cameras/discover").route(web::get().to(discover)))
                .service(
                    web::resource("/cameras/{id}")
                        .route(web::post().to(update_camera))
                        .route(web::get().to(fetch_camera)),
                )
                .service(
                    web::resource("/cameras/{id}/time")
                        .route(web::post().to(fetch_time))
                        .route(web::get().to(set_time)),
                )
                .service(
                    web::resource("/cameras/{id}/ntp")
                        .route(web::get())
                        .to(fetch_ntp)
                        .route(web::post())
                        .to(set_ntp),
                )
                .service(
                    web::resource("/cameras/{id}/ptz/relative").route(web::post().to(ptz_relative)),
                )
                .service(
                    web::resource("/cameras/{id}/ptz/{direction}")
                        .route(web::post().to(ptz_direction)),
                )
                .service(
                    web::resource("/cameras/{camera_id}/video")
                        .route(web::get().to(fetch_video_units_between)),
                )
                .service(
                    web::resource("/cameras/{camera_id}/observations")
                        .route(web::get().to(fetch_observations_between)),
                )
                .service(
                    web::resource("/cameras/{camera_id}/analysis_configuration")
                        .route(web::get().to(fetch_analysis_configuration))
                        .route(web::post().to(update_analysis_configuration)),
                )
                // routes to video_unit
                .service(web::resource("/video_units/{id}").route(web::get().to(fetch_video_unit)))
                // routes to user
                .service(web::resource("/users").route(web::post().to(create_user)))
                .service(web::resource("/users/me").route(web::get().to(fetch_current_user)))
                // routes to analysis_engine
                .service(
                    web::resource("/analysis_engines")
                        .route(web::post().to(create_analysis_engine)),
                )
                .service(
                    web::resource("/analysis_engines/{id}")
                        .route(web::get().to(fetch_analysis_engine))
                        .route(web::post().to(update_analysis_engine))
                        .route(web::delete().to(delete_analysis_engine)),
                )
                .service(
                    web::resource("/analysis_instances")
                        .route(web::post().to(create_analysis_instance)),
                )
                .service(
                    web::resource("/analysis_instances/{id}")
                        .route(web::get().to(fetch_analysis_instance))
                        .route(web::post().to(update_analysis_instance))
                        .route(web::delete().to(delete_analysis_instance)),
                )
                // Observation routes
                .service(
                    web::resource("/observations/{id}").route(web::get().to(fetch_observation)),
                )
                .service(
                    web::resource("/observations/{id}/snapshot")
                        .route(web::get().to(fetch_observation_snapshot)),
                )
                .service(
                    web::resource("/observations/{id}/clip")
                        .route(web::get().to(fetch_observation_clip)),
                )
                // Event routes
                .service(web::resource("/events").route(web::get().to(fetch_events)))
                .service(
                    web::resource("/events/{event_id}/snapshot")
                        .route(web::get().to(fetch_event_snapshot)),
                )
                .service(
                    web::resource("/events/{event_id}/clip").route(web::get().to(fetch_event_clip)),
                )
                // routes to alert rules
                .service(
                    web::resource("/alert_rule")
                        .route(web::get().to(fetch_all_alert_rules))
                        .route(web::post().to(create_alert_rule)),
                ),
        )
        // Create default route
        .service(
            web::resource("/{tail:.*}")
                .wrap(WebAuth)
                .route(web::get().to(index)),
        );
}
