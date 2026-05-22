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

use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl};

use crate::{
    api::{ResourceMetadata, camera_groups::CameraGroupSpec},
    schema::{camera_group_memberships, camera_groups},
};

#[derive(Identifiable, Eq, PartialEq, Debug, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(primary_key(name))]
#[diesel(table_name = camera_groups)]
struct CameraGroup {
    pub name: String,
    pub display_name: String,
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = camera_group_memberships)]
struct CameraGroupMembership {
    pub id: i64,
    pub camera_group_name: String,
    pub camera_name: String,
    display_order: i32,
}

impl super::Service {
    pub fn fetch_camera_group(
        &self,
        group_name: &str,
    ) -> Result<crate::api::camera_groups::CameraGroup, super::Error> {
        let mut conn = self.pool.get()?;
        conn.transaction::<_, super::Error, _>(|conn| {
            let c = camera_groups::dsl::camera_groups
                .find(group_name)
                .get_result::<CameraGroup>(conn)?;

            let mut members = camera_group_memberships::dsl::camera_group_memberships
                .filter(camera_group_memberships::camera_group_name.eq(&c.name))
                .order(camera_group_memberships::display_order.asc())
                .load::<CameraGroupMembership>(conn)?;

            members.sort_by_key(|m| m.display_order);

            Ok(crate::api::camera_groups::CameraGroup {
                metadata: ResourceMetadata {
                    name: c.name,
                    display_name: c.display_name,
                },
                spec: CameraGroupSpec {
                    members: members.into_iter().map(|m| m.camera_name).collect(),
                },
                status: serde_json::json!({}),
            })
        })
    }

    pub fn fetch_all_camera_groups(
        &self,
    ) -> Result<Vec<crate::api::camera_groups::CameraGroup>, super::Error> {
        let mut conn = self.pool.get()?;
        conn.transaction::<_, super::Error, _>(|conn| {
            let groups = camera_groups::dsl::camera_groups.load::<CameraGroup>(conn)?;

            let mut groups2 = Vec::new();
            for c in groups {
                let members = camera_group_memberships::dsl::camera_group_memberships
                    .filter(camera_group_memberships::camera_group_name.eq(&c.name))
                    .order(camera_group_memberships::display_order.asc())
                    .load::<CameraGroupMembership>(conn)?;

                groups2.push(crate::api::camera_groups::CameraGroup {
                    metadata: ResourceMetadata {
                        name: c.name,
                        display_name: c.display_name,
                    },
                    spec: CameraGroupSpec {
                        members: members.into_iter().map(|m| m.camera_name).collect(),
                    },
                    status: serde_json::json!({}),
                });
            }

            Ok(groups2)
        })
    }
}
