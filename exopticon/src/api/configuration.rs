use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use tokio::task::spawn_blocking;

use crate::{AppState, capture_supervisor::Command, db::Service};

use super::{UserError, auth::User};

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetadataInput {
    pub name: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceInput<S> {
    #[serde(default)]
    pub metadata: MetadataInput,
    #[serde(default)]
    pub spec: S,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraInput {
    pub storage_group_name: Option<String>,
    pub ip: Option<String>,
    pub onvif_port: Option<u16>,
    pub mac: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub rtsp_url: Option<String>,
    pub ptz_type: Option<String>,
    pub onvif_profile_token: Option<String>,
    pub enabled: Option<bool>,
    pub ptz_x_step_size: Option<i16>,
    pub ptz_y_step_size: Option<i16>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraGroupInput {
    pub members: Option<Vec<String>>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageGroupInput {
    pub storage_path: Option<String>,
    pub max_storage_size: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserInput {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub password: Option<String>,
}

async fn run_camera_change(
    state: AppState,
    change: impl FnOnce(Service) -> Result<(), crate::db::Error> + Send + 'static,
) -> Result<(), UserError> {
    spawn_blocking(move || {
        change(state.db_service)?;
        // A full queue means the supervisor has already queued wakeup.
        let _ = state.capture_channel.try_send(Command::RestartAll);
        Ok::<_, crate::db::Error>(())
    })
    .await??;
    Ok(())
}

pub async fn create_camera(
    State(state): State<AppState>,
    Json(input): Json<ResourceInput<CameraInput>>,
) -> Result<StatusCode, UserError> {
    run_camera_change(state, move |db| db.save_camera(None, input)).await?;
    Ok(StatusCode::CREATED)
}

pub async fn edit_camera(
    Path(name): Path<String>,
    State(state): State<AppState>,
    Json(input): Json<ResourceInput<CameraInput>>,
) -> Result<StatusCode, UserError> {
    run_camera_change(state, move |db| db.save_camera(Some(name), input)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_camera(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, UserError> {
    run_camera_change(state, move |db| db.request_camera_deletion(&name)).await?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn create_camera_group(
    State(state): State<AppState>,
    Json(input): Json<ResourceInput<CameraGroupInput>>,
) -> Result<StatusCode, UserError> {
    spawn_blocking(move || state.db_service.save_camera_group(None, input)).await??;
    Ok(StatusCode::CREATED)
}

pub async fn edit_camera_group(
    Path(name): Path<String>,
    State(state): State<AppState>,
    Json(input): Json<ResourceInput<CameraGroupInput>>,
) -> Result<StatusCode, UserError> {
    spawn_blocking(move || state.db_service.save_camera_group(Some(name), input)).await??;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_camera_group(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, UserError> {
    spawn_blocking(move || state.db_service.delete_camera_group(&name)).await??;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_storage_group(
    State(state): State<AppState>,
    Json(input): Json<ResourceInput<StorageGroupInput>>,
) -> Result<StatusCode, UserError> {
    spawn_blocking(move || state.db_service.save_storage_group(None, input)).await??;
    Ok(StatusCode::CREATED)
}

pub async fn edit_storage_group(
    Path(name): Path<String>,
    State(state): State<AppState>,
    Json(input): Json<ResourceInput<StorageGroupInput>>,
) -> Result<StatusCode, UserError> {
    spawn_blocking(move || state.db_service.save_storage_group(Some(name), input)).await??;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_storage_group(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, UserError> {
    spawn_blocking(move || state.db_service.delete_storage_group(&name)).await??;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_users(State(state): State<AppState>) -> Result<Json<Vec<User>>, UserError> {
    Ok(Json(
        spawn_blocking(move || state.db_service.list_users()).await??,
    ))
}

async fn fetch_user(
    Path(username): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<User>, UserError> {
    Ok(Json(
        spawn_blocking(move || state.db_service.fetch_user(&username)).await??,
    ))
}

async fn create_user(
    State(state): State<AppState>,
    Json(input): Json<UserInput>,
) -> Result<StatusCode, UserError> {
    spawn_blocking(move || state.db_service.save_user(None, input)).await??;
    Ok(StatusCode::CREATED)
}

async fn edit_user(
    Path(username): Path<String>,
    State(state): State<AppState>,
    Json(input): Json<UserInput>,
) -> Result<StatusCode, UserError> {
    spawn_blocking(move || state.db_service.save_user(Some(username), input)).await??;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_user(
    Path(username): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, UserError> {
    spawn_blocking(move || state.db_service.delete_user(&username)).await??;
    Ok(StatusCode::NO_CONTENT)
}

pub fn users_router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users).post(create_user))
        .route(
            "/{username}",
            get(fetch_user).patch(edit_user).delete(delete_user),
        )
}
