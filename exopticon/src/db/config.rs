/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2026 David Matthew Mattli <dmm@mattli.us>
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

use diesel::upsert::excluded;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SqliteConnection};

use crate::{
    config::{Camera, CameraGroup, StorageGroup, User, ValidatedConfig},
    schema::{camera_group_memberships, camera_groups, cameras, storage_groups, users},
};

use super::Service;

impl Service {
    pub fn apply_config(&self, config: &ValidatedConfig) -> Result<(), super::Error> {
        db_write!(self, "apply_config", |conn| {
            for group in &config.storage_groups {
                upsert_storage_group(conn, group)?;
            }

            for camera in &config.cameras {
                upsert_camera(conn, camera)?;
            }

            for group in &config.camera_groups {
                upsert_camera_group(conn, group)?;
            }

            for user in &config.users {
                upsert_user(conn, user)?;
            }

            Ok(())
        })
    }
}

fn upsert_storage_group(
    conn: &mut SqliteConnection,
    group: &StorageGroup,
) -> Result<(), diesel::result::Error> {
    diesel::insert_into(storage_groups::table)
        .values((
            storage_groups::name.eq(&group.name),
            storage_groups::display_name.eq(&group.display_name),
            storage_groups::storage_path.eq(&group.storage_path),
            storage_groups::max_storage_size.eq(group.max_storage_size),
        ))
        .on_conflict(storage_groups::name)
        .do_update()
        .set((
            storage_groups::display_name.eq(excluded(storage_groups::display_name)),
            storage_groups::storage_path.eq(excluded(storage_groups::storage_path)),
            storage_groups::max_storage_size.eq(excluded(storage_groups::max_storage_size)),
        ))
        .execute(conn)?;
    Ok(())
}

fn upsert_camera(
    conn: &mut SqliteConnection,
    camera: &Camera,
) -> Result<(), diesel::result::Error> {
    diesel::insert_into(cameras::table)
        .values((
            cameras::name.eq(&camera.name),
            cameras::display_name.eq(&camera.display_name),
            cameras::storage_group_name.eq(&camera.storage_group_name),
            cameras::ip.eq(&camera.ip),
            cameras::onvif_port.eq(i32::from(camera.onvif_port)),
            cameras::mac.eq(&camera.mac),
            cameras::username.eq(&camera.username),
            cameras::password.eq(&camera.password),
            cameras::rtsp_url.eq(&camera.rtsp_url),
            cameras::ptz_type.eq(&camera.ptz_type),
            cameras::ptz_profile_token.eq(&camera.ptz_profile_token),
            cameras::enabled.eq(camera.enabled),
            cameras::ptz_x_step_size.eq(camera.ptz_x_step_size),
            cameras::ptz_y_step_size.eq(camera.ptz_y_step_size),
        ))
        .on_conflict(cameras::name)
        .do_update()
        .set((
            cameras::display_name.eq(excluded(cameras::display_name)),
            cameras::storage_group_name.eq(excluded(cameras::storage_group_name)),
            cameras::ip.eq(excluded(cameras::ip)),
            cameras::onvif_port.eq(excluded(cameras::onvif_port)),
            cameras::mac.eq(excluded(cameras::mac)),
            cameras::username.eq(excluded(cameras::username)),
            cameras::password.eq(excluded(cameras::password)),
            cameras::rtsp_url.eq(excluded(cameras::rtsp_url)),
            cameras::ptz_type.eq(excluded(cameras::ptz_type)),
            cameras::ptz_profile_token.eq(excluded(cameras::ptz_profile_token)),
            cameras::enabled.eq(excluded(cameras::enabled)),
            cameras::ptz_x_step_size.eq(excluded(cameras::ptz_x_step_size)),
            cameras::ptz_y_step_size.eq(excluded(cameras::ptz_y_step_size)),
        ))
        .execute(conn)?;
    Ok(())
}

fn upsert_camera_group(
    conn: &mut SqliteConnection,
    group: &CameraGroup,
) -> Result<(), diesel::result::Error> {
    diesel::insert_into(camera_groups::table)
        .values((
            camera_groups::name.eq(&group.name),
            camera_groups::display_name.eq(&group.display_name),
        ))
        .on_conflict(camera_groups::name)
        .do_update()
        .set(camera_groups::display_name.eq(excluded(camera_groups::display_name)))
        .execute(conn)?;

    diesel::delete(
        camera_group_memberships::table
            .filter(camera_group_memberships::camera_group_name.eq(&group.name)),
    )
    .execute(conn)?;

    for (pos, camera_name) in group.members.iter().enumerate() {
        diesel::insert_into(camera_group_memberships::table)
            .values((
                camera_group_memberships::camera_group_name.eq(&group.name),
                camera_group_memberships::camera_name.eq(camera_name),
                camera_group_memberships::display_order
                    .eq(i32::try_from(pos).expect("camera group member position fits in i32")),
            ))
            .execute(conn)?;
    }
    Ok(())
}

fn upsert_user(conn: &mut SqliteConnection, user: &User) -> Result<(), diesel::result::Error> {
    diesel::insert_into(users::table)
        .values((
            users::username.eq(&user.username),
            users::display_name.eq(&user.display_name),
            users::password.eq(&user.password_hash),
        ))
        .on_conflict(users::username)
        .do_update()
        .set((
            users::display_name.eq(excluded(users::display_name)),
            users::password.eq(excluded(users::password)),
        ))
        .execute(conn)?;
    Ok(())
}
