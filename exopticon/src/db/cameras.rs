/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2022 David Matthew Mattli <dmm@mattli.us>
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

use chrono::NaiveDateTime;

use crate::models::StorageGroup;
use crate::schema::cameras;

/// Full camera model, represents database row
#[derive(
    Identifiable, PartialEq, Associations, Debug, Serialize, Deserialize, Queryable, Insertable,
)]
#[belongs_to(StorageGroup)]
#[serde(rename_all = "camelCase")]
#[table_name = "cameras"]
pub struct Camera {
    /// id of camera
    pub id: i32,
    /// id of associated storage group
    pub storage_group_id: i32,
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
