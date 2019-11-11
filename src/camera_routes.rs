use actix_web::{AsyncResponder, FutureResponse, HttpResponse, Json, Path, ResponseError, State};
use futures::future;
use futures::future::Either;
use futures::future::Future;
use std::time::{Duration, Instant};
use tokio::timer::Delay;

use onvif;
use onvif::camera::{DeviceDateAndTime, NtpSettings};

use crate::app::RouteState;
use crate::models::{CreateCamera, FetchAllCamera, FetchCamera, UpdateCamera};

/// Route to create new camera
pub fn create_camera(
    (camera_request, state): (Json<CreateCamera>, State<RouteState>),
) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(camera_request.into_inner())
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera) => Ok(HttpResponse::Ok().json(camera)),
            Err(err) => Ok(err.error_response()),
        })
        .responder()
}

/// Route to update existing camera
pub fn update_camera(
    (path, camera_request, state): (Path<i32>, Json<UpdateCamera>, State<RouteState>),
) -> FutureResponse<HttpResponse> {
    let camera_update = UpdateCamera {
        id: path.into_inner(),
        ..camera_request.into_inner()
    };
    state
        .db
        .send(camera_update)
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera) => Ok(HttpResponse::Ok().json(camera)),
            Err(err) => Ok(err.error_response()),
        })
        .responder()
}

/// Route to fetch specific camera by camera id
pub fn fetch_camera((path, state): (Path<i32>, State<RouteState>)) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(FetchCamera {
            id: path.into_inner(),
        })
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(camera) => Ok(HttpResponse::Ok().json(camera)),
            Err(err) => Ok(err.error_response()),
        })
        .responder()
}

/// Route to fetch all cameras
pub fn fetch_all_cameras((state,): (State<RouteState>,)) -> FutureResponse<HttpResponse> {
    state
        .db
        .send(FetchAllCamera {})
        .from_err()
        .and_then(|db_response| match db_response {
            Ok(cameras) => Ok(HttpResponse::Ok().json(cameras)),
            Err(err) => Ok(err.error_response()),
        })
        .responder()
}

/// Discovery cameras using ONVIF discovery
pub fn discover(
    (_state,): (State<RouteState>,),
) -> Box<dyn Future<Item = HttpResponse, Error = actix_web::error::Error>> {
    onvif::discovery::probe(Duration::new(5, 0))
        .map_err(actix_web::error::ErrorBadRequest)
        .and_then(|_| Ok(HttpResponse::Ok().finish()))
        .responder()
}

/// Returns current time of specified camera
pub fn fetch_time(
    (path, state): (Path<i32>, State<RouteState>),
) -> Box<dyn Future<Item = HttpResponse, Error = actix_web::error::Error>> {
    state
        .db
        .send(FetchCamera {
            id: path.into_inner(),
        })
        .map_err(actix_web::error::ErrorBadRequest)
        .and_then(|db_response| match db_response {
            Ok(camera) => {
                let onvif_cam = onvif::camera::Camera {
                    host: camera.ip,
                    port: camera.onvif_port,
                    username: camera.username,
                    password: camera.password,
                };
                Either::A(
                    onvif_cam
                        .get_date_and_time()
                        .map_err(actix_web::error::ErrorBadRequest)
                        .and_then(|datetime| Ok(HttpResponse::Ok().json(datetime))),
                )
            }
            Err(_err) => Either::B(future::done(Ok(HttpResponse::NotFound().finish()))),
        })
        .responder()
}

/// Api route to set a cameras time
///
/// # Arguments
///
/// * `path` - path containing camera id to set time for
/// * `datetime` - new time for camera
/// * `state` - route state struct
///
pub fn set_time(
    (path, datetime, state): (Path<i32>, Json<DeviceDateAndTime>, State<RouteState>),
) -> Box<dyn Future<Item = HttpResponse, Error = actix_web::error::Error>> {
    state
        .db
        .send(FetchCamera {
            id: path.into_inner(),
        })
        .map_err(actix_web::error::ErrorBadRequest)
        .and_then(|db_response| match db_response {
            Ok(camera) => {
                let onvif_cam = onvif::camera::Camera {
                    host: camera.ip,
                    port: camera.onvif_port,
                    username: camera.username,
                    password: camera.password,
                };
                let dt = datetime.into_inner();

                Either::A(
                    onvif_cam
                        .set_date_and_time(&dt)
                        .map_err(actix_web::error::ErrorBadRequest)
                        .and_then(|_| Ok(HttpResponse::Ok().finish())),
                )
            }
            Err(_err) => Either::B(future::done(Ok(HttpResponse::NotFound().finish()))),
        })
        .responder()
}

/// Returns current ntp settings of camera
pub fn fetch_ntp(
    (path, state): (Path<i32>, State<RouteState>),
) -> Box<dyn Future<Item = HttpResponse, Error = actix_web::error::Error>> {
    state
        .db
        .send(FetchCamera {
            id: path.into_inner(),
        })
        .map_err(actix_web::error::ErrorBadRequest)
        .and_then(|db_response| match db_response {
            Ok(camera) => {
                let onvif_cam = onvif::camera::Camera {
                    host: camera.ip,
                    port: camera.onvif_port,
                    username: camera.username,
                    password: camera.password,
                };
                Either::A(
                    onvif_cam
                        .get_ntp()
                        .map_err(actix_web::error::ErrorBadRequest)
                        .and_then(|datetime| Ok(HttpResponse::Ok().json(datetime))),
                )
            }
            Err(_err) => Either::B(future::done(Ok(HttpResponse::NotFound().finish()))),
        })
        .responder()
}

/// Returns current ntp settings of camera
pub fn set_ntp(
    (path, ntp_settings, state): (Path<i32>, Json<NtpSettings>, State<RouteState>),
) -> Box<dyn Future<Item = HttpResponse, Error = actix_web::error::Error>> {
    state
        .db
        .send(FetchCamera {
            id: path.into_inner(),
        })
        .map_err(actix_web::error::ErrorBadRequest)
        .and_then(|db_response| match db_response {
            Ok(camera) => {
                let onvif_cam = onvif::camera::Camera {
                    host: camera.ip,
                    port: camera.onvif_port,
                    username: camera.username,
                    password: camera.password,
                };
                Either::A(
                    onvif_cam
                        .set_ntp(&ntp_settings.into_inner())
                        .map_err(actix_web::error::ErrorBadRequest)
                        .and_then(|_| Ok(HttpResponse::Ok().finish())),
                )
            }
            Err(_err) => Either::B(future::done(Ok(HttpResponse::NotFound().finish()))),
        })
        .responder()
}

/// Struct representing ptz movement parameters
#[derive(Deserialize)]
pub struct PtzMovement {
    /// A value between -1.0 and 1.0 representing movement in the x
    /// plane.
    #[serde(default)]
    pub x: f32,
    /// A value between -1.0 and 1.0 representing movement in the y plane.
    #[serde(default)]
    pub y: f32,
    /// A value betwwen -1.0 and 1.0 representing zoom.
    #[serde(default)]
    pub zoom: f32,
}

/// Api route helper to request a relative ptz move from camera
///
/// # Arguments
///
/// * `state` - route state struct
/// * `camera_id` - id of camera to move
/// * `x` - relative x move amount
/// * `y` - relative y move amount
/// * `zoom` - relative zoom amount
///
pub fn ptz_relative_move(
    state: &State<RouteState>,
    camera_id: i32,
    x: f32,
    y: f32,
    zoom: f32,
) -> Box<dyn Future<Item = HttpResponse, Error = actix_web::error::Error>> {
    state
        .db
        .send(FetchCamera { id: camera_id })
        .map_err(actix_web::error::ErrorBadRequest)
        .and_then(move |db_response| match db_response {
            Ok(camera) => {
                let onvif_cam = onvif::camera::Camera {
                    host: camera.ip,
                    port: camera.onvif_port,
                    username: camera.username,
                    password: camera.password,
                };
                Either::A(if camera.ptz_type == "onvif_continuous" {
                    //camera.ptz_type == "onvif_continuous"
                    // other cases??
                    let stop_fut = onvif_cam
                        .stop(&camera.ptz_profile_token)
                        .map_err(actix_web::error::ErrorBadRequest);
                    let delay_fut = Delay::new(Instant::now() + Duration::from_millis(500))
                        .map_err(actix_web::error::ErrorBadRequest)
                        .and_then(|_| stop_fut);

                    Either::A(
                        onvif_cam
                            .continuous_move(&camera.ptz_profile_token, x, y, zoom, 500.0)
                            .map_err(actix_web::error::ErrorBadRequest)
                            .and_then(|_| delay_fut)
                            .and_then(|_| Ok(HttpResponse::Ok().finish())),
                    )
                } else {
                    // default to using a relative move
                    Either::B(
                        onvif_cam
                            .relative_move(&camera.ptz_profile_token, x, y, zoom)
                            .map_err(actix_web::error::ErrorBadRequest)
                            .and_then(|_| Ok(HttpResponse::Ok().finish())),
                    )
                })
            }
            Err(_err) => Either::B(future::done(Ok(HttpResponse::NotFound().finish()))),
        })
        .responder()
}

/// Api route to request relative ptz move
///
/// # Arguments
///
/// * `path` - id of camera to move
/// * `movement` - requested movement
/// * `state` - route state argument
///
pub fn ptz_relative(
    (path, movement, state): (Path<i32>, Json<PtzMovement>, State<RouteState>),
) -> Box<dyn Future<Item = HttpResponse, Error = actix_web::error::Error>> {
    let ntp = movement.into_inner();
    ptz_relative_move(&state, path.into_inner(), ntp.x, ntp.y, ntp.zoom)
}

/// Api route to request standard ptz move in specified direction
///
/// # Arguments
///
/// * `path` - id of camera to move
/// * `state` - one of 'left', 'right', 'up', 'down' indicaing which direction to move.
///
#[allow(clippy::float_arithmetic)]
pub fn ptz_direction(
    (path, state): (Path<(i32, String)>, State<RouteState>),
) -> Box<dyn Future<Item = HttpResponse, Error = actix_web::error::Error>> {
    let (x, y) = match path.1.as_str() {
        "left" => (-0.1, 0.0),
        "right" => (0.1, 0.0),
        "up" => (0.0, 0.1),
        "down" => (0.0, -0.1),
        _ => return Box::new(future::done(Ok(HttpResponse::BadRequest().finish()))),
    };

    ptz_relative_move(&state, path.0, x, y, 0.0)
}
