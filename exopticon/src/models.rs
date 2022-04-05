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

// models.rs
use actix::{Actor, SyncContext};
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use uuid::Uuid;

use crate::ws_camera_server::SubscriptionSubject;

/// This is db executor actor. can be run in parallel
pub struct DbExecutor(pub Pool<ConnectionManager<PgConnection>>);

// Actors communicate exclusively by exchanging messages.
// The sending actor can optionally wait for the response.
// Actors are not referenced directly, but by means of addresses.
// Any rust type can be an actor, it only needs to implement the Actor trait.
impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

/// Sync actor to allow file io without blocking
pub struct FileExecutor {}

impl Actor for FileExecutor {
    type Context = SyncContext<Self>;
}

/// Message requesting a file removal
pub struct RemoveFile {
    /// path to file to remove
    pub path: String,
}

use crate::schema::{
    alert_rule_cameras, alert_rules, analysis_engines, analysis_instances, camera_groups, cameras,
    event_observations, events, notification_contacts, notifiers, observation_snapshots,
    observations, user_sessions, users, video_files, video_units,
};

/// Full camera group model. Represents a full row returned from the
/// database.
#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[table_name = "camera_groups"]
#[serde(rename_all = "camelCase")]
pub struct CameraGroup {
    /// camera group id
    pub id: i32,
    /// camera group name
    pub name: String,
    /// full path to video storage path, e.g. /mnt/video/8/
    pub storage_path: String,
    /// maximum allowed storage size in bytes
    pub max_storage_size: i64,
    /// insertion time
    pub inserted_at: NaiveDateTime,
    /// update time
    pub updated_at: NaiveDateTime,
}

impl CameraGroup {
    pub fn get_snapshot_path(&self, camera_id: i32, observation_id: i64) -> String {
        format!(
            "{}/{}/observations/{}.jpg",
            self.id, camera_id, observation_id
        )
    }
}

/// Represents a camera group creation request
#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "camera_groups"]
pub struct CreateCameraGroup {
    /// camera group name
    pub name: String,
    /// full path to camera group storage, e.g. /mnt/video/8
    pub storage_path: String,
    /// maximum allowed storage size in bytes
    pub max_storage_size: i64,
}

/// Represents a camera group update request
#[derive(AsChangeset, Debug, Deserialize, Identifiable, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "camera_groups"]
pub struct UpdateCameraGroup {
    /// id of camera group to update
    pub id: i32,
    /// if provided, updated name for camera group
    pub name: Option<String>,
    /// if provided, updated storage path for camera group
    pub storage_path: Option<String>,
    /// if provided, updated storage size for camera group
    pub max_storage_size: Option<i64>,
}

/// Represents a request to fetch a camera group
pub struct FetchCameraGroup {
    /// id of camera group to fetch
    pub id: i32,
}

/// Represents a request to fetch all camera groups
pub struct FetchAllCameraGroup {}

/// Represents a request to fetch all cameras groups and associated
/// cameras
pub struct FetchAllCameraGroupAndCameras {}

/// Represents a camera group and its associated cameras
#[derive(Serialize)]
pub struct CameraGroupAndCameras(pub CameraGroup, pub Vec<Camera>);

/// Represents a request to fetch up to `count` files from the
/// specified camera group
pub struct FetchCameraGroupFiles {
    /// id of camera group to fetch associated files from
    pub camera_group_id: i32,
    /// maximum number of files to return
    pub count: i64,
}

/// Full camera model, represents database row
#[derive(
    Identifiable, PartialEq, Associations, Debug, Serialize, Deserialize, Queryable, Insertable,
)]
#[belongs_to(CameraGroup)]
#[serde(rename_all = "camelCase")]
#[table_name = "cameras"]
pub struct Camera {
    /// id of camera
    pub id: i32,
    /// id of associated camera group
    pub camera_group_id: i32,
    /// name of camera
    pub name: String,
    /// ip address associated with camera, e.g. 192.168.0.53
    pub ip: String,
    /// port used for ONVIF protocol
    pub onvif_port: i32,
    /// MAC address of camera, e.g. 9C-84-AE-0E-33-5A
    pub mac: String,
    /// username for ONVIF and RTSP authentication
    pub username: String,
    /// plaintext password for ONVIF and RTSP authentication
    pub password: String,
    /// url for rtsp stream
    pub rtsp_url: String,
    /// ptz type, either onvif or onvif_continuous
    pub ptz_type: String,
    /// ONVIF profile token for ptz
    pub ptz_profile_token: String,
    /// whether camera capture is enabled.
    pub enabled: bool,
    /// insertion time
    pub inserted_at: NaiveDateTime,
    /// update time
    pub updated_at: NaiveDateTime,
}

/// Represents a request to create a camera
#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "cameras"]
pub struct CreateCamera {
    /// id of camera group to associate with new camera
    pub camera_group_id: i32,
    /// name of camera
    pub name: String,
    /// ip address associated with camera, e.g. 192.168.0.53
    pub ip: String,
    /// port used for ONVIF protocol
    pub onvif_port: i32,
    /// MAC address of camera, e.g. 9C-84-AE-0E-33-5A
    pub mac: String,
    /// username for ONVIF and RTSP authentication
    pub username: String,
    /// plaintext password for ONVIF and RTSP authentication
    pub password: String,
    /// url for rtsp stream
    pub rtsp_url: String,
    /// ptz type, either onvif or onvif_continuous
    pub ptz_type: String,
    /// ONVIF profile token for ptz
    pub ptz_profile_token: String,
    /// whether camera capture is enabled.
    pub enabled: bool,
}

/// Represents a request to update existing camera
#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "cameras"]
pub struct UpdateCamera {
    /// id of camera to update
    pub id: i32,
    /// if present, new camera group id
    pub camera_group_id: Option<i32>,
    /// if present, new camera name
    pub name: Option<String>,
    /// if present, new ip address
    pub ip: Option<String>,
    /// if present, new onvif port
    pub onvif_port: Option<i32>,
    /// if present, new MAC address
    pub mac: Option<String>,
    /// if present, new username for ONVIF and RTSP streaming
    pub username: Option<String>,
    /// if present, new plaintext password of ONVIF and RTSP streaming
    pub password: Option<String>,
    /// if present, new rtsp_url
    pub rtsp_url: Option<String>,
    /// if present, new ptz type
    pub ptz_type: Option<String>,
    /// if present, new ONVIF ptz profile token
    pub ptz_profile_token: Option<String>,
    /// if present, updates enabled status
    pub enabled: Option<bool>,
}

/// Represents a request to fetch a camera from the database
pub struct FetchCamera {
    /// id of desired camera record
    pub id: i32,
}

/// Represents a request to fetch all camera records from database
pub struct FetchAllCamera {}

/// Represents the results of a video unit api fetch.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputVideoUnit {
    /// id of video unit
    pub id: Uuid,
    /// id of associated camera
    pub camera_id: i32,
    /// monotonic index of video unit
    pub monotonic_index: i32,
    /// begin time in UTC
    pub begin_time: NaiveDateTime,
    /// end time in UTC
    pub end_time: NaiveDateTime,
    /// insertion time
    pub inserted_at: NaiveDateTime,
    /// update time
    pub updated_at: NaiveDateTime,
}

/// Full video unit model, represents entire database row
#[derive(Identifiable, Associations, Serialize, Queryable, Clone)]
#[serde(rename_all = "camelCase")]
#[belongs_to(Camera)]
#[table_name = "video_units"]
pub struct VideoUnit {
    /// id of associated camera
    pub camera_id: i32,
    /// monotonic index
    pub monotonic_index: i32,
    /// begin time in UTC
    pub begin_time: NaiveDateTime,
    /// end time in UTC
    pub end_time: NaiveDateTime,
    /// insertion time
    pub inserted_at: NaiveDateTime,
    /// update time
    pub updated_at: NaiveDateTime,
    /// id of video unit
    pub id: Uuid,
}

/// Represents request to create new video unit record
#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "video_units"]
pub struct CreateVideoUnit {
    /// id of associated camera
    pub camera_id: i32,
    /// monotonic index
    pub monotonic_index: i32,
    /// begin time in UTC
    pub begin_time: NaiveDateTime,
    /// end time in UTC
    pub end_time: NaiveDateTime,
    /// id of video unit
    pub id: Uuid,
}

/// Represents request to update video unit record
#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "video_units"]
pub struct UpdateVideoUnit {
    /// if present, new associated camera id
    pub camera_id: Option<i32>,
    /// if present, new monotonic index
    pub monotonic_index: Option<i32>,
    /// if present, new begin time, in UTC
    pub begin_time: Option<NaiveDateTime>,
    /// if present, new end time, in UTC
    pub end_time: Option<NaiveDateTime>,
    /// id of video unit to update
    pub id: Uuid,
}

/// Represents a request to fetch a specified video unit
pub struct FetchVideoUnit {
    /// id of video unit to fetch
    pub id: Uuid,
}

/// Represents a request to fetch video units between specified times
pub struct FetchBetweenVideoUnit {
    /// id of camera to fetch video for
    pub camera_id: i32,
    /// in UTC
    pub begin_time: DateTime<Utc>,
    /// in UTC
    pub end_time: DateTime<Utc>,
}

/// Full video file model, represents full database row
#[derive(Queryable, Associations, Identifiable, Serialize)]
#[serde(rename_all = "camelCase")]
#[table_name = "video_files"]
#[belongs_to(VideoUnit)]
pub struct VideoFile {
    /// id of video file
    pub id: i32,
    /// filename of video file
    pub filename: String,
    /// size in bytes of video file
    pub size: i32,
    /// insertion time
    pub inserted_at: NaiveDateTime,
    /// update time
    pub updated_at: NaiveDateTime,
    /// id of associated video unit
    pub video_unit_id: Uuid,
}

/// Represents request to create new video file
#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "video_files"]
pub struct CreateVideoFile {
    /// filename for new video file
    pub filename: String,
    /// size in bytes of new video file
    pub size: i32,
    /// id of video unit to own this video file
    pub video_unit_id: Uuid,
}

/// Represents request to update video file
#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "video_files"]
pub struct UpdateVideoFile {
    /// id of video file to update
    pub id: i32,
    /// if present, new id of associated video unit
    pub video_unit_id: Option<Uuid>,
    /// if present, new filename
    pub filename: Option<String>,
    /// if present, new file size
    pub size: Option<i32>,
}

/// Represents a request to create a new video unit and file pair
pub struct CreateVideoUnitFile {
    /// id of video unit
    pub video_unit_id: Uuid,
    /// id of camera associated with new video unit and file
    pub camera_id: i32,
    /// monotonic index
    pub monotonic_index: i32,
    /// begin time, in UTC
    pub begin_time: NaiveDateTime,
    /// video file filename
    pub filename: String,
}

/// Represents request to update a video unit and video file pair
pub struct UpdateVideoUnitFile {
    /// id of video unit
    pub video_unit_id: Uuid,
    /// end time, in UTC
    pub end_time: NaiveDateTime,
    /// id of video file
    pub video_file_id: i32,
    /// video file size in bytes
    pub size: i32,
}

/// Represents request to fetch oldest video unit/video file pairs
pub struct FetchOldVideoUnitFile {
    /// id of camera unit to fetch from
    pub camera_group_id: i32,
    /// number of video unit/video file pairs to fetch
    pub count: i64,
}

/// Represents request to delete video units
pub struct DeleteVideoUnits {
    /// vec of ids of video units to delete
    pub video_unit_ids: Vec<Uuid>,
}

/// Represents a request to fetch empty video files, video files
/// without a size specified.
pub struct FetchEmptyVideoFile;

/// Represents an observation derived from a frame of video
#[derive(Clone, Queryable, Associations, Identifiable, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[table_name = "observations"]
#[belongs_to(VideoUnit)]
pub struct Observation {
    /// id of Observation
    pub id: i64,
    /// offset from beginning of video unit, starts at 0
    pub frame_offset: i64,
    /// Identifies the type of observation, eg Person, Motion, Deer
    pub tag: String,
    /// Details associated with observation, eg John, Male, whatever
    pub details: String,
    /// A value between 0-100 representing the percentage certainty of
    /// the observation.
    pub score: i16,
    /// upper-left x coordinate
    pub ul_x: i16,
    /// upper-left y coordinate
    pub ul_y: i16,
    /// lower-right x coordinate
    pub lr_x: i16,
    /// lower-right y coordinate
    pub lr_y: i16,
    /// Time that observation record was inserted
    pub inserted_at: DateTime<Utc>,
    /// id of owning video unit
    pub video_unit_id: Uuid,
}

/// Represents a request to create a single observation.
#[derive(AsChangeset, Debug, Serialize, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "observations"]
pub struct CreateObservation {
    /// offset from beginning of video unit, starts at 0
    pub frame_offset: i64,
    /// Identifies the type of observation, eg Person, Motion, Deer
    pub tag: String,
    /// Details associated with observation, eg John, Male, whatever
    pub details: String,
    /// A value between 0-100 representing the percentage certainty of
    /// the observation.
    pub score: i16,
    /// upper-left x coordinate
    pub ul_x: i16,
    /// upper-left y coordinate
    pub ul_y: i16,
    /// lower-right x coordinate
    pub lr_x: i16,
    /// lower-right y coordinate
    pub lr_y: i16,
    /// id of owning video unit
    pub video_unit_id: Uuid,
}

/// Represents a request to create one or more observation records.
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateObservations {
    /// Vec of observations to create
    pub observations: Vec<CreateObservation>,
}

/// Represents a request to query `Observations`
#[derive(Debug, Serialize, Deserialize)]
pub struct FetchObservations {
    /// camera id to fetch observations for
    pub camera_id: i32,
    /// beginning time - inclusive
    pub begin_time: DateTime<Utc>,
    /// end time - exclusive
    pub end_time: DateTime<Utc>,
}

/// Represents a request to query `Observations` by video unit id
#[derive(Debug, Serialize, Deserialize)]
pub struct FetchObservationsByVideoUnit {
    /// video unit id to fetch observations for
    pub video_unit_id: Uuid,
}

/// Represents a request fetch an `Observation` by observation id
#[derive(Debug, Serialize, Deserialize)]
pub struct FetchObservation {
    /// observation id to fetch
    pub id: i64,
}

/// Full user model struct, represents full value from database.
#[derive(Queryable, Associations, Identifiable, Serialize)]
#[serde(rename_all = "camelCase")]
#[table_name = "users"]
pub struct User {
    /// user id
    pub id: i32,
    ///  username
    pub username: String,
    /// hashed password
    pub password: String,
    /// Olson timezone, e.g. America/Chicago
    pub timezone: String,
    /// insertion date time
    pub inserted_at: NaiveDateTime,
    /// modified date time
    pub updated_at: NaiveDateTime,
}

/// User model without password. This is used as a return value for
/// user operations.
#[derive(Serialize)]
pub struct SlimUser {
    /// User id
    pub id: i32,
    /// username
    pub username: String,
    /// Olson database timezone, e.g. America/Chicago
    pub timezone: String,
}

impl From<User> for SlimUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            timezone: user.timezone,
        }
    }
}

/// Create new user message
#[derive(Debug, Serialize, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "users"]
pub struct CreateUser {
    /// username
    pub username: String,
    /// plaintext password
    pub password: String,
    /// Olson database timezone, e.g. America/Chicago
    pub timezone: String,
}

/// Analysis Engine database value
#[derive(Queryable, Associations, Clone, Identifiable, Serialize)]
#[serde(rename_all = "camelCase")]
#[table_name = "analysis_engines"]
pub struct AnalysisEngine {
    /// analysis engine id
    pub id: i32,
    /// Name of analysis engine
    pub name: String,
    /// Version of Analysis engine
    pub version: String,
    /// Entry point, or executable name of engine
    pub entry_point: String,
    /// Inserted date time
    pub inserted_at: NaiveDateTime,
    /// modified date time
    pub updated_at: NaiveDateTime,
}

/// Represents a request to create an `AnalysisEngine`
#[derive(AsChangeset, Debug, Serialize, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "analysis_engines"]
pub struct CreateAnalysisEngine {
    /// Name of analysis engine
    pub name: String,
    /// Version of Analysis engine
    pub version: String,
    /// Entry point, or executable name of engine
    pub entry_point: String,
}

/// Represents request to fetch an `AnalysisEngine`
pub struct FetchAnalysisEngine {
    /// id of `AnalysisEngine` to delete
    pub id: i32,
}

/// Represents request to update an `AnalysisEngine`
#[derive(AsChangeset, Debug, Deserialize, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "analysis_engines"]
pub struct UpdateAnalysisEngine {
    /// analysis engine id
    pub id: i32,
    /// Name of analysis engine
    pub name: Option<String>,
    /// Version of Analysis engine
    pub version: Option<String>,
    /// Entry point, or executable name of engine
    pub entry_point: Option<String>,
}

/// Represents a request to delete an `AnalysisEngine`
pub struct DeleteAnalysisEngine {
    /// id of `AnalysisEngine` to delete
    pub id: i32,
}

/// Request to create `AnalysisInstanceModel`
#[derive(Clone, Deserialize, Serialize)]
pub struct CreateAnalysisInstanceModel {
    /// id of owner, an analysis engine
    pub analysis_engine_id: i32,
    /// name of analysis instance
    pub name: String,
    /// max frames-per-second
    pub max_fps: i32,
    /// whether instance is enabled
    pub enabled: bool,
    /// Frame sources this instance subscribes to
    pub subscriptions: Vec<AnalysisSubscriptionModel>,
}

/// Request to fetch `AnalysisInstanceModel`
#[derive(Deserialize, Serialize)]
pub struct FetchAnalysisInstanceModel {
    /// id of `AnalysisInstanceModel` to fetch
    pub id: i32,
}

/// Request to fetch all `AnalysisEngine` and `AnalysisInstanceModel`
#[derive(Deserialize, Serialize)]
pub struct FetchAllAnalysisModel {}

/// Request to update `AnalysisInstanceModel`
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAnalysisInstanceModel {
    /// analysis instance id
    pub id: i32,
    /// id of owner, an analysis engine
    pub analysis_engine_id: Option<i32>,
    /// name of analysis instance
    pub name: Option<String>,
    /// max frames-per-second
    pub max_fps: Option<i32>,
    /// whether instance is enabled
    pub enabled: Option<bool>,
    /// Frame sources this instance subscribes to
    pub subscriptions: Option<Vec<AnalysisSubscriptionModel>>,
}

/// Represents a diesel changeset to update `AnalysisInstanceModel`
#[derive(AsChangeset, Debug, Insertable)]
#[table_name = "analysis_instances"]
pub struct AnalysisInstanceChangeset {
    /// analysis instance id
    pub id: i32,
    /// id of owner, an analysis engine
    pub analysis_engine_id: Option<i32>,
    /// name of analysis instance
    pub name: Option<String>,
    /// max frames-per-second
    pub max_fps: Option<i32>,
    /// whether instance is enabled
    pub enabled: Option<bool>,
}

/// Request to delete `AnalysisInstanceModel`
#[derive(Deserialize, Serialize)]
pub struct DeleteAnalysisInstanceModel {
    /// id of `AnalysisInstanceModel` to delete
    pub id: i32,
}

/// Represents the analysis instance domain model
#[derive(Clone, Deserialize, Serialize)]
pub struct AnalysisInstanceModel {
    /// analysis instance id
    pub id: i32,
    /// id of owner, an analysis engine
    pub analysis_engine_id: i32,
    /// name of analysis instance
    pub name: String,
    /// max frames-per-second
    pub max_fps: i32,
    /// whether instance is enabled
    pub enabled: bool,
    /// Frame sources this instance subscribes to
    pub subscriptions: Vec<AnalysisSubscriptionModel>,
}

/// Domain model for analysis subscriptions
#[derive(Clone, Deserialize, Serialize)]
pub struct AnalysisSubscriptionModel {
    /// source of frames
    pub source: SubscriptionSubject,
    /// masks that apply to this subscription
    pub masks: Vec<SubscriptionMask>,
}

/// Represents an instance of an analysis subscription mask
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionMask {
    /// upper-left x
    pub ul_x: i16,
    /// upper-left y
    pub ul_y: i16,
    /// lower-right x
    pub lr_x: i16,
    /// lower-right y
    pub lr_y: i16,
}

/// Represents Alert Rule database entry
#[derive(
    Clone, Deserialize, Serialize, Debug, Queryable, Insertable, Identifiable, PartialEq, Eq, Hash,
)]
#[serde(rename_all = "camelCase")]
#[table_name = "alert_rules"]
pub struct AlertRule {
    /// alert id
    pub id: i32,
    /// name of rule
    pub name: String,
    /// analysis instance to listen to observations from
    pub analysis_instance_id: i32,
    /// tag to alert on
    pub tag: String,
    /// detail to alert on
    pub details: String,
    /// minimum triggering score, 0-100
    pub min_score: i16,
    /// Minimum number of events necessary to create alert
    pub min_cluster_size: i16,
    /// Minimum time in between alerts, in microseconds
    pub cool_down_time: i64,
    /// id of notifier to use
    pub notifier_id: i32,
    /// inserted date time
    pub inserted_at: DateTime<Utc>,
    /// modified date time
    pub updated_at: DateTime<Utc>,
    /// contact group to send alert
    pub contact_group: String,
}

#[derive(Clone, Identifiable, Associations, Debug, Queryable, PartialEq, Eq, Hash)]
#[belongs_to(AlertRule)]
#[table_name = "alert_rule_cameras"]
pub struct AlertRuleCamera {
    /// Alert rule camera id
    pub id: i32,
    /// alert rule id
    pub alert_rule_id: i32,
    /// camera id
    pub camera_id: i32,
}

/// Represents Alert Rule domain model
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct AlertRuleModel(
    /// alert rule parent
    pub AlertRule,
    /// child camera ids
    pub Vec<i32>,
);

impl AlertRuleModel {
    /// Returns rule portion of `AlertRuleModel`
    pub const fn rule(&self) -> &AlertRule {
        &self.0
    }

    /// Returns whether rules matches given camera id
    pub fn matches_camera_id(&self, camera_id: i32) -> bool {
        self.1.is_empty() || self.1.iter().any(|&x| x == camera_id)
    }
    //    pub fn matching_camera_ids(&self) -> &[i32] {
    //        &self.1
    //    }
}

/// Represents a request to create an Alert Rule
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlertRule {
    /// rule name
    pub name: String,
    /// analysis instance to listen to observations from
    pub analysis_instance_id: i32,
    /// cameras ids to match
    pub camera_ids: Vec<i32>,
    /// tag to alert on
    pub tag: String,
    /// detail to alert on
    pub details: String,
    /// Minimum number of events necessary to create alert
    pub min_cluster_size: i16,
    /// Minimum time in between alerts, in microseconds
    pub cool_down_time: i64,
    /// id of notifier to use
    pub notifier_id: i32,
    /// contact group to send alert
    pub contact_group: String,
}

/// Represents a request to delete an Alert Rule
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAlertRule {
    /// id of alert rule to delete
    pub id: i32,
}

/// Represents a request to fetch all `AlertRules`
pub struct FetchAllAlertRule {}

/// Represents a notifier instance (mqtt for now)
#[derive(Clone, Debug, Deserialize, Serialize, Queryable, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "notifiers"]
pub struct Notifier {
    /// notifier id
    pub id: i32,
    /// name of notifier
    pub name: String,
    /// hostname of notifier
    pub hostname: String,
    /// port of notifier
    pub port: i32,
    /// service account username
    pub username: Option<String>,
    /// service account password
    pub password: Option<String>,
    /// inserted date time
    pub inserted_at: DateTime<Utc>,
    /// modified date time
    pub updated_at: DateTime<Utc>,
}

/// Represents a request to create a notifier instance
#[derive(Insertable, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[table_name = "notifiers"]
pub struct CreateNotifier {
    /// name of notifier
    pub name: String,
    /// hostname of notifier
    pub hostname: String,
    /// port of notifier
    pub port: i32,
    /// service account username
    pub username: Option<String>,
    /// service account password
    pub password: Option<String>,
}

/// Represents a request to delete a notifier
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteNotifier {
    /// id of notifier to delete
    pub id: i32,
}

/// Represents a request to fetch all Notifiers
pub struct FetchAllNotifier {}

/// Represents a notification contact
#[derive(Clone, Debug, Deserialize, Serialize, Queryable, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "notification_contacts"]
pub struct NotificationContact {
    /// id of notification contact
    pub id: i32,
    /// All contacts with same `group_name` will be notified together
    pub group_name: String,
    /// username of the contact, currently Telegram id
    pub username: String,
}

/// Request for notification contacts in specified group
pub struct FetchNotificationContactsByGroup {
    /// name of group to fetch contacts for
    pub group_name: String,
}

/// Represents Event record
#[derive(AsChangeset, Clone, Debug, Deserialize, Serialize, Queryable, Insertable)]
#[table_name = "events"]
pub struct Event {
    /// Event id
    pub id: Uuid,
    /// Event tag
    pub tag: String,
    /// Camera id
    pub camera_id: i32,
    /// Event start time
    pub begin_time: DateTime<Utc>,
    /// Event end time
    pub end_time: DateTime<Utc>,
    /// observation for display
    pub display_observation_id: i64,
}

/// Represents event observation record
#[derive(AsChangeset, Clone, Debug, Deserialize, Serialize, Queryable, Insertable)]
#[table_name = "event_observations"]
pub struct EventObservation {
    /// Event id
    pub event_id: Uuid,
    /// Observation id
    pub observation_id: i64,
}

/// Create model for Events
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct CreateEvent {
    /// id of new event
    pub id: Uuid,
    /// Event tag
    pub tag: String,
    // Camera id
    pub camera_id: i32,
    /// Observations
    pub observations: Vec<i64>,
    /// best observation for display
    pub display_observation_id: i64,
}

/// Fetch single Event
pub struct FetchEvent {
    /// event id
    pub event_id: Uuid,
}

/// Domain model for Events
#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct EventModel {
    ///event id
    pub id: Uuid,
    /// Event tag
    pub tag: String,
    /// Camera id
    pub camera_id: i32,
    /// Event start time
    pub begin_time: DateTime<Utc>,
    /// Event end time
    pub end_time: DateTime<Utc>,
    /// Observations
    pub observations: Vec<i64>,
    /// best observation for display
    pub display_observation_id: i64,
}

/// Query Events
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct QueryEvents {
    /// timestamp to begin search
    pub begin_time: Option<DateTime<Utc>>,
    /// timestamp to end search
    pub end_time: Option<DateTime<Utc>>,
    /// page
    pub page: Option<i64>,
}

/// Fetch video files associated with Event
#[derive(Clone, Debug)]
pub struct GetEventFile {
    /// event id
    pub event_id: Uuid,
}

/// File used by Event and bounds within file
#[derive(Clone, Debug)]
pub struct EventFile {
    /// tuple representing video filename, start offset, and end
    /// offset in file. The start offset is only meaningful in the
    /// first file. and the end offset is only meaningful in the last
    /// file.
    pub files: Vec<(String, i64, i64)>,
}

/// Represents image snapshot of observation
#[derive(
    Associations, AsChangeset, Clone, Debug, Deserialize, Serialize, Queryable, Insertable,
)]
#[table_name = "observation_snapshots"]
#[belongs_to(Observation)]
pub struct ObservationSnapshot {
    /// id of observation represented
    pub observation_id: i64,
    /// filename of jpg
    pub snapshot_path: String,
    /// size in bytes of snapshot file
    pub snapshot_size: i32,
}

/// Request to create observation snapshot
pub struct CreateObservationSnapshot {
    /// observation id
    pub observation_id: i64,
}

/// Request to fetch observation snapshot details
pub struct FetchObservationSnapshot {
    /// observation id
    pub observation_id: i64,
}

/// Request to fetch user details
pub struct FetchUser {
    /// id of user to fetch
    pub user_id: i32,
}

/// User login session or token
#[derive(Associations, Insertable, Serialize, Queryable, Clone)]
#[belongs_to(User)]
pub struct UserSession {
    /// user session id
    pub id: i32,
    /// user session name
    pub name: String,
    /// id of user associated with session
    pub user_id: i32,
    /// session key value
    pub session_key: String,
    /// flag indicating where it is an api token or user session
    pub is_token: bool,
    /// Expiration timestamp
    pub expiration: DateTime<Utc>,
    /// session create timestamp
    pub inserted_at: DateTime<Utc>,
}

/// Request to create new user session
#[derive(Debug, Serialize, Deserialize, Queryable, Insertable)]
#[serde(rename_all = "camelCase")]
#[table_name = "user_sessions"]
pub struct CreateUserSession {
    /// user session name
    pub name: String,
    /// id of user associated with session
    pub user_id: i32,
    /// session key value
    pub session_key: String,
    /// flag indicating where it is an api token or user session
    pub is_token: bool,
    /// Expiration timestamp
    pub expiration: DateTime<Utc>,
}

/// Request to fetch user session
pub struct FetchUserSession {
    /// session key of session to fetch
    pub session_key: String,
}

/// Delete user session
pub struct DeleteUserSession {
    /// token value to delete
    pub session_key: String,
}

/// Access Token model to return to user
#[derive(Debug, Serialize, Deserialize)]
pub struct SlimAccessToken {
    /// user session id
    pub id: i32,
    /// user session name
    pub name: String,
    /// id of user associated with session
    pub user_id: i32,
    /// Expiration timestamp
    pub expiration: DateTime<Utc>,
}

impl From<&UserSession> for SlimAccessToken {
    fn from(session: &UserSession) -> Self {
        Self {
            id: session.id,
            name: session.name.clone(),
            user_id: session.user_id,
            expiration: session.expiration,
        }
    }
}

/// Request to fetch user's tokens
pub struct FetchUserTokens {
    pub user_id: i32,
}

/// Request to create personal access token
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserToken {
    /// user id of token owner
    pub user_id: i32,
    /// name of new token
    pub name: String,
    /// expiration timestamp of new token
    pub expiration: DateTime<Utc>,
}

/// Request to delete personal access token
#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteUserToken {
    /// token id
    pub token_id: i32,
}
