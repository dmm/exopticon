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

use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use uuid::Uuid;

use crate::schema::{camera_group_memberships, camera_groups};

use super::Service;

//  What does the db infrastructure do?
//
// 1. Query building/filtering
// 2. Model binding
// 3. Mapping errors

// Models

#[derive(Identifiable, Eq, PartialEq, Debug, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = camera_groups)]
struct CameraGroup {
    pub id: Uuid,
    pub name: String,
}

#[derive(
    Identifiable, Eq, PartialEq, Associations, Debug, Serialize, Deserialize, Queryable, Insertable,
)]
#[diesel(belongs_to(CameraGroup))]
#[diesel(table_name = camera_group_memberships)]
struct CameraGroupMembership {
    pub id: Uuid,
    pub camera_group_id: Uuid,
    pub camera_id: Uuid,
    display_order: i32,
}

impl Service {
    pub fn create_camera_group(
        &self,
        group: crate::business::camera_groups::CameraGroup,
    ) -> Result<crate::api::camera_groups::CameraGroup, super::Error> {
        let mut conn = self.pool.get()?;
        let new_group = CameraGroup {
            id: Uuid::now_v7(),
            name: group.name.clone(),
        };
        conn.build_transaction()
            .serializable()
            .run::<_, super::Error, _>(|conn| {
                let new_camera_group = diesel::insert_into(camera_groups::table)
                    .values(&new_group)
                    .get_result::<CameraGroup>(conn)?;

                for (pos, m) in group.members.iter().enumerate() {
                    diesel::insert_into(camera_group_memberships::table)
                        .values(
                            &(vec![(
                                camera_group_memberships::dsl::id.eq(Uuid::now_v7()),
                                camera_group_memberships::dsl::camera_group_id
                                    .eq(new_camera_group.id),
                                camera_group_memberships::dsl::camera_id.eq(m),
                                camera_group_memberships::dsl::display_order.eq(i32::try_from(pos)
                                    .expect("Failed to convert the member position to i32")),
                            )]),
                        )
                        .execute(conn)?;
                }

                Ok(crate::api::camera_groups::CameraGroup {
                    id: new_camera_group.id,
                    name: new_camera_group.name,
                    members: group.members.clone(),
                })
            })
    }

    pub fn update_camera_group(
        &self,
        id: Uuid,
        group: crate::business::camera_groups::CameraGroup,
    ) -> Result<crate::api::camera_groups::CameraGroup, super::Error> {
        let mut conn = self.pool.get()?;
        conn.build_transaction()
            .serializable()
            .run::<_, super::Error, _>(|conn| {
                let new_camera_group: (Uuid, String) =
                    diesel::update(camera_groups::table.filter(camera_groups::dsl::id.eq(id)))
                        .set(camera_groups::dsl::name.eq(group.name))
                        .get_result(conn)?;

                diesel::delete(camera_group_memberships::table)
                    .filter(camera_group_memberships::dsl::camera_group_id.eq(id))
                    .execute(conn)?;

                for (pos, m) in group.members.iter().enumerate() {
                    diesel::insert_into(camera_group_memberships::table)
                        .values(
                            &(vec![(
                                camera_group_memberships::dsl::camera_group_id.eq(id),
                                camera_group_memberships::dsl::camera_id.eq(m),
                                camera_group_memberships::dsl::display_order.eq(i32::try_from(pos)
                                    .expect("Failed to convert the member position to i32")),
                            )]),
                        )
                        .execute(conn)?;
                }

                Ok(crate::api::camera_groups::CameraGroup {
                    id: new_camera_group.0,
                    name: new_camera_group.1,
                    members: group.members.clone(),
                })
            })
    }

    pub fn delete_camera_group(&self, id: Uuid) -> Result<(), super::Error> {
        let mut conn = self.pool.get()?;
        conn.build_transaction()
            .serializable()
            .run::<_, super::Error, _>(|conn| {
                // Delete group memberships
                diesel::delete(
                    crate::schema::camera_group_memberships::dsl::camera_group_memberships
                        .filter(camera_group_memberships::dsl::camera_group_id.eq(id)),
                )
                .execute(conn)?;

                // Delete camera group
                diesel::delete(
                    crate::schema::camera_groups::dsl::camera_groups
                        .filter(camera_groups::dsl::id.eq(id)),
                )
                .execute(conn)?;
                Ok(())
            })
    }

    pub fn fetch_camera_group(
        &self,
        id: Uuid,
    ) -> Result<crate::api::camera_groups::CameraGroup, super::Error> {
        let mut conn = self.pool.get()?;
        conn.build_transaction()
            .serializable()
            .run::<_, super::Error, _>(|conn| {
                let c = crate::schema::camera_groups::dsl::camera_groups
                    .find(id)
                    .get_result::<CameraGroup>(conn)?;

                let members =
                    crate::schema::camera_group_memberships::dsl::camera_group_memberships
                        .filter(camera_group_memberships::camera_group_id.eq(c.id))
                        .load::<CameraGroupMembership>(conn)?;

                Ok(crate::api::camera_groups::CameraGroup {
                    id: c.id,
                    name: c.name,
                    members: members.iter().map(|m| m.camera_id).collect(),
                })
            })
    }

    pub fn fetch_all_camera_groups(
        &self,
    ) -> Result<Vec<crate::api::camera_groups::CameraGroup>, super::Error> {
        let mut conn = self.pool.get()?;
        conn.build_transaction()
            .serializable()
            .run::<_, super::Error, _>(|conn| {
                let groups =
                    crate::schema::camera_groups::dsl::camera_groups.load::<CameraGroup>(conn)?;

                let mut groups2 = Vec::new();
                for c in &groups {
                    let members =
                        crate::schema::camera_group_memberships::dsl::camera_group_memberships
                            .filter(camera_group_memberships::camera_group_id.eq(c.id))
                            .load::<CameraGroupMembership>(conn)?;

                    groups2.push(crate::api::camera_groups::CameraGroup {
                        id: c.id,
                        name: c.name.to_string(),
                        members: members.iter().map(|m| m.camera_id).collect(),
                    });
                }

                Ok(groups2)
            })
    }
}
