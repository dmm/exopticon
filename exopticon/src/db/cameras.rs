/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2023 David Matthew Mattli <dmm@mattli.us>
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

use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use uuid::Uuid;

use crate::db::storage_groups::StorageGroup;
use crate::schema::cameras;

use super::Service;

/// Full camera model, represents database row
#[derive(Identifiable, PartialEq, Eq, Associations, Debug, Queryable, Insertable)]
#[diesel(belongs_to(StorageGroup))]
#[diesel(table_name = cameras)]
pub struct Camera {
    /// id of camera
    pub id: Uuid,
    /// id of associated storage group
    pub storage_group_id: Uuid,
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
    /// ptz type, either `onvif` or `onvif_continuous`
    pub ptz_type: String,
    /// ONVIF profile token for ptz
    pub ptz_profile_token: String,
    /// whether camera capture is enabled.
    pub enabled: bool,
    /// ptz x step size, in hundredths
    pub ptz_x_step_size: i16,
    /// ptz y step size, in hundredths
    pub ptz_y_step_size: i16,
}

#[derive(PartialEq, Eq, Associations, Debug, Queryable, Insertable)]
#[diesel(belongs_to(StorageGroup))]
#[diesel(table_name = cameras)]
pub struct CreateCamera {
    /// id of associated storage group
    pub storage_group_id: Uuid,
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
    /// ptz type, either `onvif` or `onvif_continuous`
    pub ptz_type: String,
    /// ONVIF profile token for ptz
    pub ptz_profile_token: String,
    /// whether camera capture is enabled.
    pub enabled: bool,
    /// ptz x step size, in hundredths
    pub ptz_x_step_size: i16,
    /// ptz y step size, in hundredths
    pub ptz_y_step_size: i16,
}

impl From<crate::api::cameras::CreateCamera> for CreateCamera {
    fn from(c: crate::api::cameras::CreateCamera) -> Self {
        Self {
            storage_group_id: c.storage_group_id,
            name: c.name,
            ip: c.ip,
            onvif_port: c.onvif_port,
            mac: c.mac,
            username: c.username,
            password: c.password,
            rtsp_url: c.rtsp_url,
            ptz_type: c.ptz_type,
            ptz_profile_token: c.ptz_profile_token,
            enabled: c.enabled,
            ptz_x_step_size: c.ptz_x_step_size,
            ptz_y_step_size: c.ptz_y_step_size,
        }
    }
}

#[derive(AsChangeset, Debug)]
#[diesel(table_name = cameras)]
pub struct UpdateCamera {
    /// if present, new storage group id
    pub storage_group_id: Option<Uuid>,
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
    /// if present, new `rtsp_url`
    pub rtsp_url: Option<String>,
    /// if present, new ptz type
    pub ptz_type: Option<String>,
    /// if present, new ONVIF ptz profile token
    pub ptz_profile_token: Option<String>,
    /// if present, updates enabled status
    pub enabled: Option<bool>,
    /// ptz x step size, in hundredths
    pub ptz_x_step_size: Option<i16>,
    /// ptz y step size, in hundredths
    pub ptz_y_step_size: Option<i16>,
}

impl From<crate::api::cameras::UpdateCamera> for UpdateCamera {
    fn from(u: crate::api::cameras::UpdateCamera) -> Self {
        Self {
            storage_group_id: u.storage_group_id,
            name: u.name,
            ip: u.ip,
            onvif_port: u.onvif_port,
            mac: u.mac,
            username: u.username,
            password: u.password,
            rtsp_url: u.rtsp_url,
            ptz_type: u.ptz_type,
            ptz_profile_token: u.ptz_profile_token,
            enabled: u.enabled,
            ptz_x_step_size: u.ptz_x_step_size,
            ptz_y_step_size: u.ptz_y_step_size,
        }
    }
}

impl Service {
    pub fn create_camera(
        &self,
        create_camera: crate::api::cameras::CreateCamera,
    ) -> Result<crate::api::cameras::Camera, super::Error> {
        let mut conn = self.pool.get()?;

        let c: Camera = diesel::insert_into(crate::schema::cameras::dsl::cameras)
            .values(&Into::<CreateCamera>::into(create_camera))
            .get_result(&mut conn)?;

        Ok(c.into())
    }

    pub fn update_camera(
        &self,
        camera_id: Uuid,
        camera: crate::api::cameras::UpdateCamera,
    ) -> Result<crate::api::cameras::Camera, super::Error> {
        use crate::schema::cameras::dsl::*;
        let mut conn = self.pool.get()?;

        let c: Camera = diesel::update(cameras.filter(id.eq(camera_id)))
            .set(&Into::<UpdateCamera>::into(camera))
            .get_result(&mut conn)?;

        Ok(c.into())
    }

    pub fn delete_camera(&self, cid: Uuid) -> Result<(), super::Error> {
        use crate::schema::cameras::dsl::*;
        let mut conn = self.pool.get()?;

        diesel::delete(cameras.filter(id.eq(cid))).execute(&mut conn)?;
        Ok(())
    }

    pub fn fetch_camera(&self, id: Uuid) -> Result<crate::api::cameras::Camera, super::Error> {
        let mut conn = self.pool.get()?;

        let c = crate::schema::cameras::dsl::cameras
            .find(id)
            .get_result::<Camera>(&mut conn)?;

        Ok(c.into())
    }

    pub fn fetch_all_cameras(&self) -> Result<Vec<crate::api::cameras::Camera>, super::Error> {
        let mut conn = self.pool.get()?;

        let cameras: Vec<crate::db::cameras::Camera> =
            crate::schema::cameras::dsl::cameras.load(&mut conn)?;
        Ok(cameras.into_iter().map(std::convert::Into::into).collect())
    }
}
