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

use diesel::{QueryDsl, RunQueryDsl};

use crate::{
    api::{
        ResourceMetadata,
        cameras::{CameraSpec, CameraStatus},
    },
    schema::cameras,
};

#[derive(Identifiable, PartialEq, Eq, Debug, Queryable, Insertable, Clone)]
#[diesel(primary_key(name))]
#[diesel(table_name = cameras)]
pub struct Camera {
    pub name: String,
    pub display_name: String,
    pub storage_group_name: String,
    pub ip: String,
    pub onvif_port: i32,
    pub mac: String,
    pub username: String,
    pub password: String,
    pub rtsp_url: String,
    pub ptz_type: String,
    pub ptz_profile_token: String,
    pub enabled: bool,
    pub ptz_x_step_size: i16,
    pub ptz_y_step_size: i16,
}

impl From<Camera> for crate::api::cameras::Camera {
    fn from(c: Camera) -> Self {
        let status = if c.enabled {
            CameraStatus {
                phase: "stopped".to_string(),
                active: false,
                last_started_at: None,
                last_packet_at: None,
                video_codec: None,
                audio_codec: None,
                average_bitrate: None,
                error_message: None,
            }
        } else {
            CameraStatus::disabled()
        };

        Self {
            metadata: ResourceMetadata {
                name: c.name,
                display_name: c.display_name,
            },
            spec: CameraSpec {
                storage_group_name: c.storage_group_name,
                ip: c.ip,
                onvif_port: c.onvif_port,
                mac: c.mac,
                username: c.username,
                rtsp_url: c.rtsp_url,
                ptz_type: c.ptz_type,
                ptz_profile_token: c.ptz_profile_token,
                enabled: c.enabled,
                ptz_x_step_size: c.ptz_x_step_size,
                ptz_y_step_size: c.ptz_y_step_size,
            },
            status,
        }
    }
}

impl super::Service {
    pub fn fetch_camera_row(&self, camera_name: &str) -> Result<Camera, super::Error> {
        let mut conn = self.pool.get()?;

        let c = crate::schema::cameras::dsl::cameras
            .find(camera_name)
            .get_result::<Camera>(&mut conn)?;

        Ok(c)
    }

    pub fn fetch_camera(
        &self,
        camera_name: &str,
    ) -> Result<crate::api::cameras::Camera, super::Error> {
        Ok(self.fetch_camera_row(camera_name)?.into())
    }

    pub fn fetch_all_camera_rows(&self) -> Result<Vec<Camera>, super::Error> {
        let mut conn = self.pool.get()?;

        let cameras: Vec<Camera> = crate::schema::cameras::dsl::cameras.load(&mut conn)?;
        Ok(cameras)
    }

    pub fn fetch_all_cameras(&self) -> Result<Vec<crate::api::cameras::Camera>, super::Error> {
        Ok(self
            .fetch_all_camera_rows()?
            .into_iter()
            .map(std::convert::Into::into)
            .collect())
    }
}
