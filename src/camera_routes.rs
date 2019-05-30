use actix_web::{AsyncResponder, FutureResponse, HttpResponse, Json, Path, ResponseError, State};
use futures::future;
use futures::future::Either;
use futures::future::Future;
use std::time::Duration;

use onvif;
use onvif::camera::{DeviceDateAndTime, DeviceNtpSettings};

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
) -> Box<Future<Item = HttpResponse, Error = actix_web::error::Error>> {
    onvif::discovery::probe(Duration::new(5, 0))
        .map_err(actix_web::error::ErrorBadRequest)
        .and_then(|_| Ok(HttpResponse::Ok().finish()))
        .responder()
}

/// Returns current time of specified camera
pub fn fetch_time(
    (path, state): (Path<i32>, State<RouteState>),
) -> Box<Future<Item = HttpResponse, Error = actix_web::error::Error>> {
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
) -> Box<Future<Item = HttpResponse, Error = actix_web::error::Error>> {
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
) -> Box<Future<Item = HttpResponse, Error = actix_web::error::Error>> {
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
    (path, ntp_settings, state): (Path<i32>, Json<DeviceNtpSettings>, State<RouteState>),
) -> Box<Future<Item = HttpResponse, Error = actix_web::error::Error>> {
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
                        .set_ntp(ntp_settings.into_inner())
                        .map_err(actix_web::error::ErrorBadRequest)
                        .and_then(|datetime| Ok(HttpResponse::Ok().json(datetime))),
                )
            }
            Err(_err) => Either::B(future::done(Ok(HttpResponse::NotFound().finish()))),
        })
        .responder()
}
