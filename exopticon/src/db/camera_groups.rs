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

use super::{Service, ServiceKind};

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
        match &self.pool {
            ServiceKind::Real(pool) => {
                let mut conn = pool.get()?;
                conn.build_transaction()
                    .serializable()
                    .run::<_, super::Error, _>(|conn| {
                        let new_camera_group = diesel::insert_into(camera_groups::table)
                            .values(&(vec![camera_groups::dsl::name.eq(group.name)]))
                            .get_result::<CameraGroup>(conn)?;

                        for (pos, m) in group.members.iter().enumerate() {
                            diesel::insert_into(camera_group_memberships::table)
                                .values(
                                    &(vec![(
                                        camera_group_memberships::dsl::camera_group_id
                                            .eq(new_camera_group.id),
                                        camera_group_memberships::dsl::camera_id.eq(m),
                                        camera_group_memberships::dsl::display_order.eq(
                                            i32::try_from(pos).expect(
                                                "Failed to convert the member position to i32",
                                            ),
                                        ),
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
            ServiceKind::Null(db) => {
                db.lock().unwrap().camera_groups = vec![];
                Ok(crate::api::camera_groups::CameraGroup {
                    id: Uuid::now_v7(),
                    name: group.name,
                    members: group.members,
                })
            }
        }
    }

    pub fn update_camera_group(
        &self,
        id: Uuid,
        group: crate::business::camera_groups::CameraGroup,
    ) -> Result<crate::api::camera_groups::CameraGroup, super::Error> {
        match &self.pool {
            ServiceKind::Real(pool) => {
                let mut conn = pool.get()?;
                conn.build_transaction()
                    .serializable()
                    .run::<_, super::Error, _>(|conn| {
                        let new_camera_group: (Uuid, String) = diesel::update(
                            camera_groups::table.filter(camera_groups::dsl::id.eq(id)),
                        )
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
                                        camera_group_memberships::dsl::display_order.eq(
                                            i32::try_from(pos).expect(
                                                "Failed to convert the member position to i32",
                                            ),
                                        ),
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
            ServiceKind::Null(db) => {
                for g in &mut db.lock().unwrap().camera_groups {
                    if id == g.id {
                        g.name.clone_from(&group.name);
                        g.members.clone_from(&group.members);
                        return Ok(crate::api::camera_groups::CameraGroup {
                            id,
                            name: group.name.clone(),
                            members: group.members,
                        });
                    }
                }
                Err(super::Error::NotFound)
            }
        }
    }

    pub fn delete_camera_group(&self, id: Uuid) -> Result<(), super::Error> {
        match &self.pool {
            ServiceKind::Real(pool) => {
                let mut conn = pool.get()?;
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

            ServiceKind::Null(db) => {
                let mut data = db.lock().unwrap();

                data.camera_groups
                    .iter()
                    .position(|g| g.id == id)
                    .map_or_else(
                        || Err(super::Error::NotFound),
                        |i| {
                            data.camera_groups.remove(i);
                            Ok(())
                        },
                    )
            }
        }
    }

    pub fn fetch_camera_group(
        &self,
        id: Uuid,
    ) -> Result<crate::api::camera_groups::CameraGroup, super::Error> {
        match &self.pool {
            ServiceKind::Real(pool) => {
                let mut conn = pool.get()?;
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
            ServiceKind::Null(pool) => {
                for group in &pool.lock().unwrap().camera_groups {
                    if group.id == id {
                        return Ok(group.clone());
                    }
                }
                Err(super::Error::NotFound)
            }
        }
    }

    pub fn fetch_all_camera_groups(
        &self,
    ) -> Result<Vec<crate::api::camera_groups::CameraGroup>, super::Error> {
        match &self.pool {
            ServiceKind::Real(pool) => {
                let mut conn = pool.get()?;
                conn.build_transaction()
                    .serializable()
                    .run::<_, super::Error, _>(|conn| {
                        let groups = crate::schema::camera_groups::dsl::camera_groups
                            .load::<CameraGroup>(conn)?;

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
            ServiceKind::Null(pool) => {
                let db = pool.lock().expect("unable to lock null db");

                Ok(db.camera_groups.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::db::{NullBuilder, Service};

    use crate::api::camera_groups::CameraGroup;

    #[test]
    fn test_delete_nonexistant_group() {
        // Arrange
        let db = Service::new_null(None);

        // Act
        let res = db.delete_camera_group(1);

        // Assert
        assert!(res.is_err());
        if let Err(crate::db::Error::NotFound) = res {
        } else {
            panic!("Expected db::Error::NotFound");
        }
    }

    #[test]
    fn test_delete_null_camera_group() {
        // Arrange
        let delete_id = Uuid::now_v7();
        let keep_id = 2;
        let mut builder = NullBuilder::new();
        builder.camera_groups(&vec![
            CameraGroup {
                id: delete_id,
                name: String::from("Group1"),
                members: Vec::new(),
            },
            CameraGroup {
                id: keep_id,
                name: String::from("Group2"),
                members: Vec::new(),
            },
        ]);
        let db = Service::new_null(Some(builder.build()));

        // Act
        db.delete_camera_group(delete_id).unwrap();
        let all_groups = db.fetch_all_camera_groups().unwrap();

        // Assert
        assert_eq!(1, all_groups.len());
        assert_eq!(2, all_groups[0].id);
    }

    #[test]
    fn test_update_name() {
        // Arrange
        let id = 42;
        let mut builder = NullBuilder::new();
        builder.camera_groups(&vec![CameraGroup {
            id,
            name: String::from("Group1"),
            members: Vec::new(),
        }]);
        let db = Service::new_null(Some(builder.build()));

        // Act
        db.update_camera_group(
            id,
            crate::business::camera_groups::CameraGroup {
                name: String::from("Group2"),
                members: Vec::new(),
            },
        )
        .unwrap();
        let new_group = db.fetch_camera_group(id).unwrap();

        // Assert
        assert_eq!(String::from("Group2"), new_group.name);
    }
}

#[cfg(test)]
mod integration_tests {

    use diesel::RunQueryDsl;

    use crate::db::{tests::run_db_test, Error, Service, ServiceKind};

    fn populate_db(db: &Service) -> (i32, Vec<i32>) {
        if let ServiceKind::Real(pool) = &db.pool {
            let storage_group: crate::db::storage_groups::StorageGroup =
                diesel::insert_into(crate::schema::storage_groups::table)
                    .values(crate::db::storage_groups::CreateStorageGroup {
                        name: "testgroup".to_string(),
                        storage_path: "/test/path".to_string(),
                        max_storage_size: 100,
                    })
                    .get_result(&pool.get().unwrap())
                    .unwrap();

            let mut cameras = Vec::new();

            for n in 1..10 {
                let camera: crate::db::cameras::Camera =
                    diesel::insert_into(crate::schema::cameras::table)
                        .values(crate::db::cameras::CreateCamera {
                            storage_group_id: storage_group.id,
                            name: format!("TestCamera{}", n),
                            ip: "".to_string(),
                            onvif_port: 0,
                            mac: "".to_string(),
                            username: "".to_string(),
                            password: "".to_string(),
                            rtsp_url: "".to_string(),
                            ptz_type: "".to_string(),
                            ptz_profile_token: "".to_string(),
                            ptz_x_step_size: 0,
                            ptz_y_step_size: 0,
                            enabled: true,
                        })
                        .get_result(&pool.get().unwrap())
                        .unwrap();
                cameras.push(camera);
            }
            (storage_group.id, cameras.iter().map(|c| c.id).collect())
        } else {
            panic!("Need a real connection for integration tests!");
        }
    }

    #[test]
    #[ignore]
    fn test_fetch_non_existant_camera_group() {
        run_db_test(|db| {
            // arrange

            // act
            let res = db.fetch_camera_group(31337);

            // assert
            assert!(res.is_err());
            if let Result::Err(Error::NotFound) = res {
            } else {
                panic!("Expected NotFound error!");
            }
        })
    }

    #[test]
    #[ignore]
    fn test_create_empty_camera_group() {
        run_db_test(|db| {
            // arrange
            let group_name = "new_group_1#";
            let new_group =
                crate::business::camera_groups::CameraGroup::new(group_name, Vec::new()).unwrap();

            // act
            let camera_group_response = db.create_camera_group(new_group);

            // assert
            let camera_group = camera_group_response.unwrap();
            assert_eq!(group_name, camera_group.name);
        });
    }

    #[test]
    #[ignore]
    fn test_create_full_camera_group() {
        run_db_test(|db| {
            // arrange
            let ids = populate_db(db);
            let group_name = "new_group_1";
            let new_group =
                crate::business::camera_groups::CameraGroup::new(group_name, ids.1.clone())
                    .unwrap();

            // act
            db.create_camera_group(new_group).unwrap();
            let stored_group = db.fetch_camera_group(ids.0);

            // assert
            assert_eq!(ids.1, stored_group.unwrap().members);
        })
    }

    #[test]
    #[ignore]
    fn test_create_camera_group_with_duplicate_members() {
        run_db_test(|db| {
            // arrange
            let ids = populate_db(db);
            let group_name = "new_group_1";
            let dup_ids = vec![ids.1[0], ids.1[0]];
            let new_group =
                crate::business::camera_groups::CameraGroup::new(group_name, dup_ids).unwrap();

            // act
            let camera_group_response = db.create_camera_group(new_group);

            assert!(camera_group_response.is_err());
        })
    }

    #[test]
    #[ignore]
    fn test_delete_camera_group() {
        run_db_test(|db| {
            // arrange
            let ids = populate_db(db);
            let new_group =
                crate::business::camera_groups::CameraGroup::new("new_group_1", ids.1).unwrap();

            // act
            let group = db.create_camera_group(new_group).unwrap();
            db.delete_camera_group(group.id).unwrap();

            let groups = db.fetch_all_camera_groups().unwrap();

            // assert
            assert_eq!(0, groups.len());
        })
    }

    #[test]
    #[ignore]
    fn test_fetch_all_camera_groups() {
        run_db_test(|db| {
            // arrange
            let ids = populate_db(db);
            let name = "new_group_1";
            let new_group_1 =
                crate::business::camera_groups::CameraGroup::new("new_group_1", ids.1.clone())
                    .unwrap();
            let new_group_2 =
                crate::business::camera_groups::CameraGroup::new("new_group_2", ids.1).unwrap();

            // act
            db.create_camera_group(new_group_1).unwrap();
            db.create_camera_group(new_group_2).unwrap();
            let groups = db.fetch_all_camera_groups().unwrap();

            // assert
            assert_eq!(2, groups.len());
            assert_eq!(name, groups[0].name);
            assert_eq!("new_group_2", groups[1].name);
        })
    }
}
