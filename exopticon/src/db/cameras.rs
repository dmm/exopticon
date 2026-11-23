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

use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl};

use crate::{
    api::{
        ResourceMetadata,
        cameras::{CameraSpec, CameraStatus, PtzSettings, StreamSource},
    },
    schema::cameras,
};

#[derive(Identifiable, PartialEq, Eq, Debug, Queryable, Insertable, AsChangeset, Clone)]
#[diesel(primary_key(name))]
#[diesel(table_name = cameras, treat_none_as_null = true)]
pub struct Camera {
    pub name: String,
    pub display_name: String,
    #[diesel(skip_update)]
    pub storage_group_name: String,
    pub username: String,
    pub password: String,
    pub rtsp_url: String,
    pub onvif_profile_token: Option<String>,
    pub enabled: bool,
    pub ptz_x_step_size: i16,
    pub ptz_y_step_size: i16,
    pub deletion_state: i32,
    pub onvif_device_url: Option<String>,
    pub ptz_enabled: bool,
    pub ptz_device_url: Option<String>,
    pub ptz_profile_token: Option<String>,
}

impl Camera {
    pub fn stream_source(&self) -> StreamSource {
        match &self.onvif_device_url {
            Some(device_url) => StreamSource::Onvif {
                device_url: device_url.clone(),
                profile_token: self
                    .onvif_profile_token
                    .clone()
                    .expect("ONVIF stream has a profile token"),
            },
            None => StreamSource::StaticRtsp {
                url: self.rtsp_url.clone(),
            },
        }
    }

    pub fn ptz_target(&self) -> Option<(&str, &str)> {
        let device_url = self
            .ptz_device_url
            .as_deref()
            .or(self.onvif_device_url.as_deref())?;
        let profile_token = self
            .ptz_profile_token
            .as_deref()
            .or(self.onvif_profile_token.as_deref())?;
        Some((device_url, profile_token))
    }
}

impl From<Camera> for crate::api::cameras::Camera {
    fn from(c: Camera) -> Self {
        let status = if c.enabled {
            CameraStatus {
                phase: "stopped".to_string(),
                active: false,
                last_started_at: None,
                video_codec: None,
                audio_codec: None,
                average_bitrate: None,
                error_message: None,
            }
        } else {
            CameraStatus::disabled()
        };

        let stream_source = c.stream_source();
        Self {
            metadata: ResourceMetadata {
                name: c.name,
                display_name: c.display_name,
            },
            spec: CameraSpec {
                storage_group_name: c.storage_group_name,
                stream_source,
                ptz: c.ptz_enabled.then_some(PtzSettings {
                    device_url: c.ptz_device_url,
                    profile_token: c.ptz_profile_token,
                    x_step_size: c.ptz_x_step_size,
                    y_step_size: c.ptz_y_step_size,
                }),
                enabled: c.enabled,
            },
            status,
        }
    }
}

impl super::Service {
    pub fn fetch_camera_row(&self, camera_name: &str) -> Result<Camera, super::Error> {
        db_read!(self, "fetch_camera_row", |conn| {
            let c = crate::schema::cameras::dsl::cameras
                .find(camera_name)
                .filter(cameras::deletion_state.eq(super::configuration::ACTIVE))
                .get_result::<Camera>(conn)?;

            Ok(c)
        })
    }

    pub fn fetch_camera(
        &self,
        camera_name: &str,
    ) -> Result<crate::api::cameras::Camera, super::Error> {
        Ok(self.fetch_camera_row(camera_name)?.into())
    }

    pub fn fetch_all_camera_rows(&self) -> Result<Vec<Camera>, super::Error> {
        db_read!(self, "fetch_all_camera_rows", |conn| {
            let cameras: Vec<Camera> = cameras::table
                .filter(cameras::deletion_state.eq(super::configuration::ACTIVE))
                .load(conn)?;
            Ok(cameras)
        })
    }

    pub fn fetch_all_cameras(&self) -> Result<Vec<crate::api::cameras::Camera>, super::Error> {
        Ok(self
            .fetch_all_camera_rows()?
            .into_iter()
            .map(std::convert::Into::into)
            .collect())
    }
}
