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

use actix::{Handler, Message};
use chrono::NaiveDateTime;
use diesel::*;
use diesel::{dsl::any, sql_types::Integer};

use crate::analysis_routes::{AnalysisConfiguration, AnalysisType, FetchAnalysisConfiguration};
use crate::errors::ServiceError;
use crate::models::{
    AnalysisEngine, AnalysisInstanceChangeset, AnalysisInstanceModel, AnalysisSubscriptionModel,
    Camera, CreateAnalysisEngine, CreateAnalysisInstanceModel, DbExecutor, DeleteAnalysisEngine,
    DeleteAnalysisInstanceModel, FetchAllAnalysisModel, FetchAnalysisEngine,
    FetchAnalysisInstanceModel, SubscriptionMask, UpdateAnalysisEngine,
    UpdateAnalysisInstanceModel,
};
use crate::ws_camera_server::{FrameResolution, SubscriptionSubject};

impl Message for CreateAnalysisEngine {
    type Result = Result<AnalysisEngine, ServiceError>;
}

impl Handler<CreateAnalysisEngine> for DbExecutor {
    type Result = Result<AnalysisEngine, ServiceError>;

    fn handle(&mut self, msg: CreateAnalysisEngine, _: &mut Self::Context) -> Self::Result {
        use crate::schema::analysis_engines::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(analysis_engines)
            .values(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for FetchAnalysisEngine {
    type Result = Result<AnalysisEngine, ServiceError>;
}

impl Handler<FetchAnalysisEngine> for DbExecutor {
    type Result = Result<AnalysisEngine, ServiceError>;

    fn handle(&mut self, msg: FetchAnalysisEngine, _: &mut Self::Context) -> Self::Result {
        use crate::schema::analysis_engines::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let c = analysis_engines
            .find(msg.id)
            .get_result::<AnalysisEngine>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        Ok(c)
    }
}

impl Message for UpdateAnalysisEngine {
    type Result = Result<AnalysisEngine, ServiceError>;
}

impl Handler<UpdateAnalysisEngine> for DbExecutor {
    type Result = Result<AnalysisEngine, ServiceError>;

    fn handle(&mut self, msg: UpdateAnalysisEngine, _: &mut Self::Context) -> Self::Result {
        use crate::schema::analysis_engines::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::update(analysis_engines.filter(id.eq(msg.id)))
            .set(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for DeleteAnalysisEngine {
    type Result = Result<(), ServiceError>;
}

impl Handler<DeleteAnalysisEngine> for DbExecutor {
    type Result = Result<(), ServiceError>;

    fn handle(&mut self, msg: DeleteAnalysisEngine, _: &mut Self::Context) -> Self::Result {
        use crate::schema::analysis_engines::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        let _rows_deleted = diesel::delete(analysis_engines.filter(id.eq(msg.id)))
            .execute(conn)
            .map_err(|_error| ServiceError::InternalServerError);

        Ok(())
    }
}

/// function to delete subscriptions owned by given analysis instance id
fn delete_subscriptions(analysis_id: i32, conn: &PgConnection) -> Result<(), ServiceError> {
    use crate::schema::analysis_subscriptions::dsl::*;
    use crate::schema::subscription_masks::dsl::*;

    let subs = analysis_subscriptions
        .filter(analysis_instance_id.eq(analysis_id))
        .load::<(
            i32,
            i32,
            Option<i32>,
            Option<i32>,
            NaiveDateTime,
            NaiveDateTime,
        )>(conn)
        .map_err(|_error| ServiceError::InternalServerError)?;

    for s in subs {
        // delete child masks
        diesel::delete(subscription_masks.filter(analysis_subscription_id.eq(s.0)))
            .execute(conn)?;
        diesel::delete(
            analysis_subscriptions.filter(crate::schema::analysis_subscriptions::dsl::id.eq(s.0)),
        )
        .execute(conn)?;
    }

    Ok(())
}

/// function to insert subscriptions and associated child tables.
fn insert_subscriptions(
    analysis_id: i32,
    subscriptions: &[AnalysisSubscriptionModel],
    conn: &PgConnection,
) -> Result<(), ServiceError> {
    use crate::schema::analysis_subscriptions::dsl::*;
    use crate::schema::subscription_masks::dsl::*;

    // Insert subscriptions
    for s in subscriptions {
        let ids = match s.source {
            SubscriptionSubject::Camera(cid, _) => (Some(cid), None),
            SubscriptionSubject::AnalysisEngine(analysis_id) => (None, Some(analysis_id)),
            SubscriptionSubject::Playback(_, _, _) => {
                return Err(ServiceError::BadRequest(
                    "Playback is an invalid subscription source".to_string(),
                ))
            }
        };
        let sub_model = diesel::insert_into(analysis_subscriptions)
            .values((
                analysis_instance_id.eq(analysis_id),
                camera_id.eq(ids.0),
                source_analysis_instance_id.eq(ids.1),
            ))
            .get_result::<(
                i32,
                i32,
                Option<i32>,
                Option<i32>,
                NaiveDateTime,
                NaiveDateTime,
            )>(conn)
            .map_err(|error| {
                error!("Error inserting analysis subscription: {}", error);
                ServiceError::InternalServerError
            })?;
        // Insert masks
        for m in &s.masks {
            diesel::insert_into(subscription_masks)
                .values((
                    analysis_subscription_id.eq(sub_model.0),
                    ul_x.eq(m.ul_x),
                    ul_y.eq(m.ul_y),
                    lr_x.eq(m.lr_x),
                    lr_y.eq(m.lr_y),
                ))
                .execute(conn)
                .map_err(|error| {
                    error!("Error inserting subscription masks! {}", error);
                    ServiceError::InternalServerError
                })?;
        }
    }
    Ok(())
}

impl Message for CreateAnalysisInstanceModel {
    type Result = Result<AnalysisInstanceModel, ServiceError>;
}

impl Handler<CreateAnalysisInstanceModel> for DbExecutor {
    type Result = Result<AnalysisInstanceModel, ServiceError>;

    fn handle(&mut self, msg: CreateAnalysisInstanceModel, _: &mut Self::Context) -> Self::Result {
        use crate::schema::analysis_instances::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        conn.transaction(|| {
            // Insert analysis instance
            let instance = diesel::insert_into(analysis_instances)
                .values((
                    analysis_engine_id.eq(msg.analysis_engine_id),
                    name.eq(msg.name),
                    max_fps.eq(msg.max_fps),
                    enabled.eq(msg.enabled),
                ))
                .get_result::<(i32, i32, String, i32, bool, NaiveDateTime, NaiveDateTime)>(conn)
                .map_err(|_error| ServiceError::InternalServerError)?;

            // Insert subscriptions
            insert_subscriptions(instance.0, &msg.subscriptions, conn)?;

            Ok(AnalysisInstanceModel {
                id: instance.0,
                analysis_engine_id: instance.1,
                name: instance.2,
                max_fps: instance.3,
                enabled: instance.4,
                subscriptions: msg.subscriptions,
            })
        })
    }
}

impl Message for UpdateAnalysisInstanceModel {
    type Result = Result<AnalysisInstanceModel, ServiceError>;
}

impl Handler<UpdateAnalysisInstanceModel> for DbExecutor {
    type Result = Result<AnalysisInstanceModel, ServiceError>;

    fn handle(&mut self, msg: UpdateAnalysisInstanceModel, _: &mut Self::Context) -> Self::Result {
        use crate::schema::analysis_instances::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        conn.transaction(|| {
            // Update analysis instance
            diesel::update(
                analysis_instances.filter(crate::schema::analysis_instances::dsl::id.eq(msg.id)),
            )
            .set(&AnalysisInstanceChangeset {
                id: msg.id,
                analysis_engine_id: msg.analysis_engine_id,
                name: msg.name,
                max_fps: msg.max_fps,
                enabled: msg.enabled,
            })
            .execute(conn)?;

            if let Some(new_subs) = msg.subscriptions {
                // Update subscriptions
                // delete all subscriptions for analysis engine
                delete_subscriptions(msg.id, conn)?;
                insert_subscriptions(msg.id, &new_subs, conn)?;
            }
            fetch_analysis_instance(msg.id, conn)
        })
    }
}

/// fetch subscriptions that are owned by the given analysis instance
/// id
fn fetch_subscriptions(
    analysis_id: i32,
    conn: &PgConnection,
) -> Result<Vec<AnalysisSubscriptionModel>, ServiceError> {
    use crate::schema::analysis_subscriptions::dsl::*;
    use crate::schema::subscription_masks::dsl::*;

    // load subscriptions
    let mut subscriptions = Vec::new();
    let subs = analysis_subscriptions
        .filter(analysis_instance_id.eq(analysis_id))
        .load::<(
            i32,
            i32,
            Option<i32>,
            Option<i32>,
            NaiveDateTime,
            NaiveDateTime,
        )>(conn)
        .map_err(|_error| ServiceError::InternalServerError)?;

    for s in subs {
        let m = subscription_masks
            .filter(analysis_subscription_id.eq(s.0))
            .load::<(i32, i32, i16, i16, i16, i16, NaiveDateTime, NaiveDateTime)>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?
            .into_iter()
            .map(|(_, _, ulx, uly, lrx, lry, _, _)| SubscriptionMask {
                ul_x: ulx,
                ul_y: uly,
                lr_x: lrx,
                lr_y: lry,
            })
            .collect();

        let source = match s.2 {
            Some(cid) => SubscriptionSubject::Camera(cid, FrameResolution::SD),
            None => SubscriptionSubject::AnalysisEngine(s.3.expect("Referential integrity!")),
        };

        subscriptions.push(AnalysisSubscriptionModel { source, masks: m });
    }

    Ok(subscriptions)
}

/// fetches analysis instance identified by the given analysis instance id
fn fetch_analysis_instance(
    analysis_id: i32,
    conn: &PgConnection,
) -> Result<AnalysisInstanceModel, ServiceError> {
    use crate::schema::analysis_instances::dsl::*;

    // load analysis instance
    let a = analysis_instances
        .filter(crate::schema::analysis_instances::dsl::id.eq(analysis_id))
        .first::<(i32, i32, String, i32, bool, NaiveDateTime, NaiveDateTime)>(conn)
        .map_err(|_error| ServiceError::InternalServerError)?;

    let subscriptions = fetch_subscriptions(a.0, conn)?;
    Ok(AnalysisInstanceModel {
        id: a.0,
        analysis_engine_id: a.1,
        name: a.2,
        max_fps: a.3,
        enabled: a.4,
        subscriptions,
    })
}

/// fetches analysis instances belonging to the specified analysis engine
fn fetch_analysis_instances_belonging(
    parent_analysis_engine_id: i32,
    conn: &PgConnection,
) -> Result<Vec<AnalysisInstanceModel>, ServiceError> {
    use crate::schema::analysis_instances::dsl::*;

    let children = analysis_instances
        .filter(analysis_engine_id.eq(parent_analysis_engine_id))
        .load::<(i32, i32, String, i32, bool, NaiveDateTime, NaiveDateTime)>(conn)
        .map_err(|_error| ServiceError::InternalServerError)?;

    let mut instances = Vec::new();
    for c in children {
        let subscriptions = fetch_subscriptions(c.0, conn)?;
        instances.push(AnalysisInstanceModel {
            id: c.0,
            analysis_engine_id: c.1,
            name: c.2,
            max_fps: c.3,
            enabled: c.4,
            subscriptions,
        });
    }

    Ok(instances)
}

impl Message for FetchAnalysisInstanceModel {
    type Result = Result<AnalysisInstanceModel, ServiceError>;
}

impl Handler<FetchAnalysisInstanceModel> for DbExecutor {
    type Result = Result<AnalysisInstanceModel, ServiceError>;

    fn handle(&mut self, msg: FetchAnalysisInstanceModel, _: &mut Self::Context) -> Self::Result {
        let conn: &PgConnection = &self.0.get().unwrap();

        conn.transaction(|| fetch_analysis_instance(msg.id, conn))
    }
}

impl Message for FetchAllAnalysisModel {
    type Result = Result<Vec<(AnalysisEngine, Vec<AnalysisInstanceModel>)>, ServiceError>;
}

impl Handler<FetchAllAnalysisModel> for DbExecutor {
    type Result = Result<Vec<(AnalysisEngine, Vec<AnalysisInstanceModel>)>, ServiceError>;

    fn handle(&mut self, _msg: FetchAllAnalysisModel, _: &mut Self::Context) -> Self::Result {
        use crate::schema::analysis_engines::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        conn.transaction(|| {
            let engines = analysis_engines.load::<AnalysisEngine>(conn)?;

            let mut analysis_groups = Vec::new();

            for eng in engines {
                let instances = fetch_analysis_instances_belonging(eng.id, conn)?;
                analysis_groups.push((eng, instances));
            }

            Ok(analysis_groups)
        })
    }
}

impl Message for DeleteAnalysisInstanceModel {
    type Result = Result<(), ServiceError>;
}

impl Handler<DeleteAnalysisInstanceModel> for DbExecutor {
    type Result = Result<(), ServiceError>;

    fn handle(&mut self, msg: DeleteAnalysisInstanceModel, _: &mut Self::Context) -> Self::Result {
        use crate::schema::analysis_instances::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        // Delete children
        delete_subscriptions(msg.id, conn)?;

        // delete analysis instance
        diesel::delete(analysis_instances.filter(id.eq(msg.id))).execute(conn)?;

        Ok(())
    }
}

impl Message for FetchAnalysisConfiguration {
    type Result = Result<AnalysisConfiguration, ServiceError>;
}

impl Handler<FetchAnalysisConfiguration> for DbExecutor {
    type Result = Result<AnalysisConfiguration, ServiceError>;

    fn handle(&mut self, msg: FetchAnalysisConfiguration, _: &mut Self::Context) -> Self::Result {
        use crate::schema::analysis_instances::dsl::*;
        use crate::schema::analysis_subscriptions::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        conn.transaction(|| {
            let motion_id: Option<i32> = analysis_instances
                .inner_join(
                    analysis_subscriptions
                        .on(analysis_instance_id.eq(crate::schema::analysis_instances::dsl::id)),
                )
                .select(crate::schema::analysis_instances::id)
                .filter(analysis_engine_id.eq(AnalysisType::Motion as i32))
                .filter(camera_id.eq(msg.camera_id))
                .get_result(conn)
                .optional()
                .map_err(|_error| ServiceError::InternalServerError)?;

            // Fetch Coral and Yolo analysis instance ids
            let coral_id: i32 = analysis_instances
                .select(crate::schema::analysis_instances::dsl::id)
                .filter(analysis_engine_id.eq(AnalysisType::Coral as i32))
                .get_result(conn)
                .map_err(|error| {
                    error!("Failed to fetch coral analysis instance: {}", error);
                    ServiceError::InternalServerError
                })?;
            let yolo_id: i32 = analysis_instances
                .select(crate::schema::analysis_instances::dsl::id)
                .filter(analysis_engine_id.eq(AnalysisType::Yolo as i32))
                .get_result(conn)
                .map_err(|error| {
                    error!("Failed to fetch yolo analysis instance: {}", error);
                    ServiceError::InternalServerError
                })?;

            debug!(
                "Motion id: {:?} coral id: {:?} Yolo id: {:?}",
                motion_id, coral_id, yolo_id
            );
            let obj_id: Option<i32> =
                match motion_id {
                    Some(motion_id) => analysis_instances
                        .inner_join(analysis_subscriptions.on(
                            analysis_instance_id.eq(crate::schema::analysis_instances::dsl::id),
                        ))
                        .select(crate::schema::analysis_instances::dsl::id)
                        .filter(
                            analysis_engine_id
                                .eq(AnalysisType::Coral as i32)
                                .or(analysis_engine_id.eq(AnalysisType::Yolo as i32)),
                        )
                        .filter(source_analysis_instance_id.eq(motion_id))
                        .get_result(conn)
                        .optional()
                        .map_err(|_error| ServiceError::InternalServerError)?,

                    None => None,
                };

            debug!("Object id: {:?}", obj_id);

            Ok(AnalysisConfiguration {
                camera_id: msg.camera_id,
                analysis_type: match (motion_id, obj_id) {
                    (None, None) => AnalysisType::None,
                    (Some(_), None) => AnalysisType::Motion,
                    (Some(_), Some(obj_detection_id)) => {
                        if obj_detection_id == yolo_id {
                            AnalysisType::Yolo
                        } else if obj_detection_id == coral_id {
                            AnalysisType::Coral
                        } else {
                            error!(
                                "Invalid object detection engine selected! {}",
                                obj_detection_id
                            );
                            return Err(ServiceError::InternalServerError);
                        }
                    }
                    (_, _) => return Err(ServiceError::InternalServerError),
                },
            })
        })
    }
}

#[derive(Debug, QueryableByName)]
struct UnusedInstance {
    #[sql_type = "Integer"]
    pub analysis_instance_id: i32,
}

impl Message for AnalysisConfiguration {
    type Result = Result<Self, ServiceError>;
}

impl Handler<AnalysisConfiguration> for DbExecutor {
    type Result = Result<AnalysisConfiguration, ServiceError>;

    #[allow(clippy::too_many_lines)]
    fn handle(&mut self, msg: AnalysisConfiguration, _: &mut Self::Context) -> Self::Result {
        use crate::schema::analysis_instances::dsl::*;
        use crate::schema::analysis_subscriptions::dsl::*;
        use crate::schema::cameras::dsl::*;
        use crate::schema::subscription_masks::dsl::{
            analysis_subscription_id, subscription_masks,
        };
        let conn: &PgConnection = &self.0.get().unwrap();

        conn.transaction(|| {
            debug!("one!");
            let motion_id: Option<i32> = analysis_instances
                .inner_join(
                    analysis_subscriptions
                        .on(analysis_instance_id.eq(crate::schema::analysis_instances::dsl::id)),
                )
                .select(crate::schema::analysis_instances::dsl::id)
                .filter(analysis_engine_id.eq(AnalysisType::Motion as i32))
                .filter(camera_id.eq(msg.camera_id))
                .get_result(conn)
                .optional()
                .map_err(|error| {
                    error!("Failed to fetch motion analysis instance: {}", error);
                    ServiceError::InternalServerError
                })?;
            debug!("two!");
            if let Some(id2) = motion_id {
                // delete existing motion analysis instance
                let subs = analysis_subscriptions
                    .filter(analysis_instance_id.eq(id2))
                    .load::<(
                        i32,
                        i32,
                        Option<i32>,
                        Option<i32>,
                        NaiveDateTime,
                        NaiveDateTime,
                    )>(conn)
                    .map_err(|error| {
                        error!(
                            "Failed to fetch subscriptions for motion analysis instance: {}",
                            error
                        );
                        ServiceError::InternalServerError
                    })?;
                debug!("three");
                for s in subs {
                    // delete child masks
                    diesel::delete(subscription_masks.filter(analysis_subscription_id.eq(s.0)))
                        .execute(conn)?;
                    diesel::delete(
                        analysis_subscriptions
                            .filter(crate::schema::analysis_subscriptions::dsl::id.eq(s.0)),
                    )
                    .execute(conn)?;
                }
                debug!("four");
                // delete object detection subscriptions
                let obj_subs: Option<i32> = analysis_subscriptions
                    .filter(source_analysis_instance_id.eq(id2))
                    .select(crate::schema::analysis_subscriptions::dsl::id)
                    .get_result(conn)
                    .optional()
                    .map_err(|_error| ServiceError::InternalServerError)?;

                if let Some(sub_id) = obj_subs {
                    diesel::delete(subscription_masks.filter(analysis_subscription_id.eq(sub_id)))
                        .execute(conn)?;
                    diesel::delete(
                        analysis_subscriptions
                            .filter(crate::schema::analysis_subscriptions::dsl::id.eq(sub_id)),
                    )
                    .execute(conn)?;
                }
                debug!("five");
                diesel::delete(
                    analysis_instances.filter(crate::schema::analysis_instances::dsl::id.eq(id2)),
                )
                .execute(conn)?;
            }

            // Now create new analysis configuration
            if msg.analysis_type != AnalysisType::None {
                // create motion analysis
                let camera = cameras
                    .find(msg.camera_id)
                    .get_result::<Camera>(conn)
                    .map_err(|error| {
                        error!("Failed to fetch camera: {}", error);
                        ServiceError::InternalServerError
                    })?;
                debug!("six");
                let motion_instance = diesel::insert_into(analysis_instances)
                    .values((
                        analysis_engine_id.eq(AnalysisType::Motion as i32),
                        crate::schema::analysis_instances::dsl::name
                            .eq(format!("{} motion detection", camera.name)),
                        max_fps.eq(10),
                        crate::schema::analysis_instances::dsl::enabled.eq(true),
                    ))
                    .get_result::<(i32, i32, String, i32, bool, NaiveDateTime, NaiveDateTime)>(conn)
                    .map_err(|error| {
                        error!("Failed to insert new motion instance: {}", error);
                        ServiceError::InternalServerError
                    })?;
                debug!("seven");
                // Insert motion analysis subscription
                insert_subscriptions(
                    motion_instance.0,
                    &[AnalysisSubscriptionModel {
                        source: SubscriptionSubject::Camera(msg.camera_id, FrameResolution::SD),

                        masks: Vec::new(),
                    }],
                    conn,
                )?;

                if msg.analysis_type == AnalysisType::Coral {
                    // fetch coral instance
                    let coral_id: i32 = analysis_instances
                        .select(crate::schema::analysis_instances::dsl::id)
                        .filter(analysis_engine_id.eq(AnalysisType::Coral as i32))
                        .get_result(conn)
                        .map_err(|error| {
                            error!("Failed to fetch coral analysis instance: {}", error);
                            ServiceError::InternalServerError
                        })?;

                    // Insert coral analysis subscription
                    insert_subscriptions(
                        coral_id,
                        &[AnalysisSubscriptionModel {
                            source: SubscriptionSubject::AnalysisEngine(motion_instance.0),
                            masks: Vec::new(),
                        }],
                        conn,
                    )?;
                } else if msg.analysis_type == AnalysisType::Yolo {
                    // fetch yolo instance
                    let yolo_id: i32 = analysis_instances
                        .select(crate::schema::analysis_instances::dsl::id)
                        .filter(analysis_engine_id.eq(AnalysisType::Yolo as i32))
                        .get_result(conn)
                        .map_err(|error| {
                            error!("Failed to fetch yolo analysis instance: {}", error);
                            ServiceError::InternalServerError
                        })?;

                    // Insert coral analysis subscription
                    insert_subscriptions(
                        yolo_id,
                        &[AnalysisSubscriptionModel {
                            source: SubscriptionSubject::AnalysisEngine(motion_instance.0),
                            masks: Vec::new(),
                        }],
                        conn,
                    )?;
                }
            }

            // Disable analysis instances without subscriptions
            let query = r#"
                SELECT ai.id AS analysis_instance_id
                FROM analysis_instances ai
                LEFT OUTER JOIN analysis_subscriptions sub
                  ON ai.id = sub.analysis_instance_id
                WHERE sub.id IS NULL;
                "#;

            let unused_instances: Vec<UnusedInstance> =
                diesel::sql_query(query).load(conn).map_err(|error| {
                    error!("Failed to fetch unused analysis instances {}", error);
                    ServiceError::InternalServerError
                })?;
            let unused_instance_ids: Vec<i32> = unused_instances
                .into_iter()
                .map(|a| a.analysis_instance_id)
                .collect();

            diesel::update(analysis_instances)
                .filter(crate::schema::analysis_instances::dsl::id.ne(any(&unused_instance_ids)))
                .set(crate::schema::analysis_instances::dsl::enabled.eq(true))
                .execute(conn)
                .map_err(|error| {
                    error!("Failed to update analysis instances {}", error);
                    ServiceError::InternalServerError
                })?;

            diesel::update(analysis_instances)
                .filter(crate::schema::analysis_instances::dsl::id.eq(any(&unused_instance_ids)))
                .set(crate::schema::analysis_instances::dsl::enabled.eq(false))
                .execute(conn)
                .map_err(|error| {
                    error!("Failed to update analysis instances {}", error);
                    ServiceError::InternalServerError
                })?;

            Ok(msg)
        })
    }
}
