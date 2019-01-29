// models.rs
use actix::{Actor, SyncContext};
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};

/// This is db executor actor. can be run in parallel
pub struct DbExecutor(pub Pool<ConnectionManager<PgConnection>>);

// Actors communicate exclusively by exchanging messages.
// The sending actor can optionally wait for the response.
// Actors are not referenced directly, but by means of addresses.
// Any rust type can be an actor, it only needs to implement the Actor trait.
impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

use crate::schema::{camera_groups, cameras, users, video_files, video_units};
use chrono::NaiveDateTime;

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[table_name = "camera_groups"]
#[serde(rename_all = "camelCase")]
pub struct CameraGroup {
    pub id: i32,
    pub name: String,
    pub storage_path: String,
    pub max_storage_size: i64,
    pub inserted_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "camera_groups"]
pub struct CreateCameraGroup {
    pub name: String,
    pub storage_path: String,
    pub max_storage_size: i64,
}

#[derive(AsChangeset, Debug, Deserialize, Identifiable, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "camera_groups"]
pub struct UpdateCameraGroup {
    pub id: i32,
    pub name: Option<String>,
    pub storage_path: Option<String>,
    pub max_storage_size: Option<i64>,
}

pub struct FetchCameraGroup {
    pub id: i32,
}

pub struct FetchAllCameraGroup {}

pub struct FetchAllCameraGroupAndCameras {}

pub struct FetchCameraGroupFiles {
    pub camera_group_id: i32,
    pub count: i64,
}

#[derive(
    Identifiable, PartialEq, Associations, Debug, Serialize, Deserialize, Queryable, Insertable,
)]
#[belongs_to(CameraGroup)]
#[serde(rename_all = "camelCase")]
#[table_name = "cameras"]
pub struct Camera {
    pub id: i32,
    pub camera_group_id: i32,
    pub name: String,
    pub ip: String,
    pub onvif_port: i32,
    pub mac: String,
    pub username: String,
    pub password: String,
    pub rtsp_url: String,
    pub ptz_type: String,
    pub ptz_profile_token: String,
    pub enabled: bool,
    pub inserted_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize)]
pub struct CameraGroupAndCameras(pub CameraGroup, pub Vec<Camera>);

#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "cameras"]
pub struct CreateCamera {
    pub camera_group_id: i32,
    pub name: String,
    pub ip: String,
    pub onvif_port: i32,
    pub mac: String,
    pub username: String,
    pub password: String,
    pub rtsp_url: String,
    pub ptz_type: String,
    pub ptz_profile_token: String,
    pub enabled: bool,
}

#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "cameras"]
pub struct UpdateCamera {
    pub id: i32,
    pub camera_group_id: Option<i32>,
    pub name: Option<String>,
    pub ip: Option<String>,
    pub onvif_port: Option<i32>,
    pub mac: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub rtsp_url: Option<String>,
    pub ptz_type: Option<String>,
    pub ptz_profile_token: Option<String>,
    pub enabled: Option<bool>,
}

pub struct FetchCamera {
    pub id: i32,
}

pub struct FetchAllCamera {}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputVideoUnit {
    pub id: i32,
    pub camera_id: i32,
    pub monotonic_index: i32,
    pub begin_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub files: Vec<VideoFile>,
    pub inserted_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Identifiable, Associations, Serialize, Queryable)]
#[serde(rename_all = "camelCase")]
#[belongs_to(Camera)]
#[table_name = "video_units"]
pub struct VideoUnit {
    pub id: i32,
    pub camera_id: i32,
    pub monotonic_index: i32,
    pub begin_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
    pub inserted_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "video_units"]
pub struct CreateVideoUnit {
    pub camera_id: i32,
    pub monotonic_index: i32,
    pub begin_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
}

#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "video_units"]
pub struct UpdateVideoUnit {
    pub id: i32,
    pub camera_id: Option<i32>,
    pub monotonic_index: Option<i32>,
    pub begin_time: Option<NaiveDateTime>,
    pub end_time: Option<NaiveDateTime>,
}

pub struct FetchVideoUnit {
    pub id: i32,
}

pub struct FetchBetweenVideoUnit {
    pub begin_time: NaiveDateTime,
    pub end_time: NaiveDateTime,
}

#[derive(Queryable, Associations, Identifiable, Serialize)]
#[serde(rename_all = "camelCase")]
#[table_name = "video_files"]
#[belongs_to(VideoUnit)]
pub struct VideoFile {
    pub id: i32,
    pub video_unit_id: i32,
    pub filename: String,
    pub size: i32,
    pub inserted_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "video_files"]
pub struct CreateVideoFile {
    pub video_unit_id: i32,
    pub filename: String,
    pub size: i32,
}

#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "video_files"]
pub struct UpdateVideoFile {
    pub id: i32,
    pub video_unit_id: Option<i32>,
    pub filename: Option<String>,
    pub size: Option<i32>,
}

pub struct CreateVideoUnitFile {
    pub camera_id: i32,
    pub monotonic_index: i32,
    pub begin_time: NaiveDateTime,
    pub filename: String,
}

pub struct UpdateVideoUnitFile {
    pub video_unit_id: i32,
    pub end_time: NaiveDateTime,
    pub video_file_id: i32,
    pub size: i32,
}

pub struct FetchOldVideoUnitFile {
    pub camera_group_id: i32,
    pub count: i64,
}

pub struct DeleteVideoUnitFiles {
    pub video_unit_ids: Vec<i32>,
    pub video_file_ids: Vec<i32>,
}

pub struct FetchEmptyVideoFile;

#[derive(Queryable, Associations, Identifiable, Serialize)]
#[serde(rename_all = "camelCase")]
#[table_name = "users"]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password: String,
    pub timezone: String,
    pub inserted_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Serialize)]
pub struct SlimUser {
    pub id: i32,
    pub username: String,
    pub timezone: String,
}

impl From<User> for SlimUser {
    fn from(user: User) -> Self {
        SlimUser {
            id: user.id,
            username: user.username,
            timezone: user.timezone,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "users"]
pub struct CreateUser {
    pub username: String,
    pub password: String,
    pub timezone: String,
}
