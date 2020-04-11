// We have to pass by value to satisfy the actix route interface.
#![allow(clippy::needless_pass_by_value)]

use actix_http::ResponseBuilder;
use actix_web::{http::StatusCode, web::Data, web::Json, web::Path, Error, HttpResponse};
use std::time::Duration;
use tokio::time::delay_for;

use onvif;
use onvif::camera::{DeviceDateAndTime, NtpSettings};

use crate::app::RouteState;
use crate::models::{CreateCamera, FetchAllCamera, FetchCamera, UpdateCamera};

#[derive(Debug)]
struct CameraError {
    msg: String,
}
impl CameraError {
    fn new(msg: &str) -> CameraError {
        CameraError {
            msg: msg.to_string(),
        }
    }
}

impl std::fmt::Display for CameraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Camera Error {}", self.msg)
    }
}

impl actix_web::error::ResponseError for CameraError {
    fn error_response(&self) -> HttpResponse {
        ResponseBuilder::new(self.status_code()).body(self.msg.clone())
    }

    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

impl From<onvif::error::Error> for CameraError {
    fn from(err: onvif::error::Error) -> CameraError {
        match err {
            onvif::error::Error::ConnectionFailed => CameraError::new("Failed to connect to camera. It's down or its host/port is misconfigured."),
            onvif::error::Error::Unauthorized => CameraError::new("Camera access unauthorized. Check camera username/password."),
            onvif::error::Error::InvalidResponse => CameraError::new("Camera returned an invalid response. This is a bug in the onvif library or the camera."),
            onvif::error::Error::InvalidArgument => CameraError::new("An invalid argument was provided. This is an exopticon bug.."),
        }
    }
}

/// Route to create new camera
pub async fn create_camera(
    camera_request: Json<CreateCamera>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let camera = state.db.send(camera_request.into_inner()).await??;

    Ok(HttpResponse::Ok().json(camera))
}

/// Route to update existing camera
pub async fn update_camera(
    path: Path<i32>,
    camera_request: Json<UpdateCamera>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let camera_update = UpdateCamera {
        id: path.into_inner(),
        ..camera_request.into_inner()
    };

    let new_camera = state.db.send(camera_update).await??;

    Ok(HttpResponse::Ok().json(new_camera))
}

/// Route to fetch specific camera by camera id
pub async fn fetch_camera(path: Path<i32>, state: Data<RouteState>) -> Result<HttpResponse, Error> {
    let camera = state
        .db
        .send(FetchCamera {
            id: path.into_inner(),
        })
        .await??;
    Ok(HttpResponse::Ok().json(camera))
}

/// Route to fetch all cameras
pub async fn fetch_all_cameras(state: Data<RouteState>) -> Result<HttpResponse, Error> {
    let cameras = state.db.send(FetchAllCamera {}).await??;

    Ok(HttpResponse::Ok().json(cameras))
}

/// Discovery cameras using ONVIF discovery
pub async fn discover() -> Result<HttpResponse, Error> {
    onvif::discovery::probe(Duration::new(5, 0)).await?;
    Ok(HttpResponse::Ok().finish())
}

/// Returns current time of specified camera
pub async fn fetch_time(path: Path<i32>, state: Data<RouteState>) -> Result<HttpResponse, Error> {
    let camera = state
        .db
        .send(FetchCamera {
            id: path.into_inner(),
        })
        .await??;

    let onvif_cam = onvif::camera::Camera {
        host: camera.ip,
        port: camera.onvif_port,
        username: camera.username,
        password: camera.password,
    };
    match onvif_cam.get_date_and_time().await {
        Ok(datetime) => Ok(HttpResponse::Ok().json(datetime)),
        Err(_err) => Ok(HttpResponse::InternalServerError().finish()),
    }
}

/// Api route to set a cameras time
///
/// # Arguments
///
/// * `path` - path containing camera id to set time for
/// * `datetime` - new time for camera
/// * `state` - route state struct
///
pub async fn set_time(
    path: Path<i32>,
    datetime: Json<DeviceDateAndTime>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let db_response = state
        .db
        .send(FetchCamera {
            id: path.into_inner(),
        })
        .await?;

    match db_response {
        Ok(camera) => {
            let onvif_cam = onvif::camera::Camera {
                host: camera.ip,
                port: camera.onvif_port,
                username: camera.username,
                password: camera.password,
            };
            let dt = datetime.into_inner();
            match onvif_cam.set_date_and_time(&dt).await {
                Ok(_) => Ok(HttpResponse::Ok().finish()),
                Err(_err) => Ok(HttpResponse::NotFound().finish()),
            }
        }
        Err(_err) => Ok(HttpResponse::NotFound().finish()),
    }
}

/// Returns current ntp settings of camera
pub async fn fetch_ntp(path: Path<i32>, state: Data<RouteState>) -> Result<HttpResponse, Error> {
    let db_response = state
        .db
        .send(FetchCamera {
            id: path.into_inner(),
        })
        .await?;
    let camera = match db_response {
        Ok(camera) => onvif::camera::Camera {
            host: camera.ip,
            port: camera.onvif_port,
            username: camera.username,
            password: camera.password,
        },
        Err(_err) => return Ok(HttpResponse::NotFound().finish()),
    };
    match camera.get_ntp().await {
        Ok(datetime) => Ok(HttpResponse::Ok().json(datetime)),
        Err(_) => Ok(HttpResponse::BadRequest().finish()),
    }
}

/// Returns current ntp settings of camera
pub async fn set_ntp(
    path: Path<i32>,
    ntp_settings: Json<NtpSettings>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let db_response = state
        .db
        .send(FetchCamera {
            id: path.into_inner(),
        })
        .await?;
    let camera = match db_response {
        Ok(camera) => onvif::camera::Camera {
            host: camera.ip,
            port: camera.onvif_port,
            username: camera.username,
            password: camera.password,
        },
        Err(_err) => return Ok(HttpResponse::NotFound().finish()),
    };
    match camera.set_ntp(&ntp_settings.into_inner()).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(_) => Ok(HttpResponse::InternalServerError().finish()),
    }
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
/// * `state` - `RouteState`
/// * `camera_id` - id of camera to move
/// * `x` - relative x move amount
/// * `y` - relative y move amount
/// * `zoom` - relative zoom amount
///
pub async fn ptz_relative_move(
    state: Data<RouteState>,
    camera_id: i32,
    x: f32,
    y: f32,
    zoom: f32,
) -> Result<HttpResponse, Error> {
    let db_response = state.db.send(FetchCamera { id: camera_id }).await?;

    let camera = match db_response {
        Ok(camera) => camera,
        Err(_err) => return Ok(HttpResponse::NotFound().finish()),
    };
    let onvif_cam = onvif::camera::Camera {
        host: camera.ip,
        port: camera.onvif_port,
        username: camera.username,
        password: camera.password,
    };
    if camera.ptz_type == "onvif_continuous" {
        //camera.ptz_type == "onvif_continuous"
        // other cases??
        // start continuous move
        let con = onvif_cam
            .continuous_move(&camera.ptz_profile_token, x, y, zoom, 500.0)
            .await;

        if let Err(_err) = con {
            return Ok(HttpResponse::InternalServerError().finish());
        }

        // wait
        delay_for(Duration::from_millis(500)).await;

        // stop continuous move
        let con = onvif_cam.stop(&camera.ptz_profile_token).await;

        if let Err(_err) = con {
            return Ok(HttpResponse::InternalServerError().finish());
        }
    } else {
        // default to using a relative move
        if let Err(_err) = onvif_cam
            .relative_move(&camera.ptz_profile_token, x, y, zoom)
            .await
        {
            return Ok(HttpResponse::InternalServerError().finish());
        }
    }
    Ok(HttpResponse::Ok().finish())
}

/// Api route to request relative ptz move
////// # Arguments
///
/// * `path` - id of camera to move
/// * `movement` - requested movement
/// * `state` - route state argument
///
pub async fn ptz_relative(
    path: Path<i32>,
    movement: Json<PtzMovement>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let ntp = movement.into_inner();
    ptz_relative_move(state, path.into_inner(), ntp.x, ntp.y, ntp.zoom).await
}

/// Api route to request standard ptz move in specified direction
///
/// # Arguments
///
/// * `path` - id of camera to move
/// * `state` - one of 'left', 'right', 'up', 'down' indicaing which direction to move.
///
#[allow(clippy::float_arithmetic)]
pub async fn ptz_direction(
    path: Path<(i32, String)>,
    state: Data<RouteState>,
) -> Result<HttpResponse, Error> {
    let (x, y) = match path.1.as_ref() {
        "left" => (-0.1, 0.0),
        "right" => (0.1, 0.0),
        "up" => (0.0, 0.1),
        "down" => (0.0, -0.1),
        _ => return Ok(HttpResponse::BadRequest().finish()),
    };

    ptz_relative_move(state, path.0, x, y, 0.0).await
}
