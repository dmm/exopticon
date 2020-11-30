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

use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::time::Instant;

use actix::fut::wrap_future;
use actix::prelude::*;
use actix_interop::{critical_section, with_ctx, FutureInterop};
use tokio::time::{delay_for, Duration};
use url::Url;

use crate::db_registry;
use crate::models::{AlertRule, AlertRuleModel, FetchAllAlertRule, Observation};
use crate::notifier_supervisor::{NotifierSupervisor, SendNotification};
use crate::observation_routes::fetch_observation_image;
use crate::ws_camera_server::{
    CameraFrame, FrameSource, Subscribe, SubscriptionSubject, WsCameraServer,
};

/// Actor that implements observation alerts
pub struct AlertActor {
    /// Alert Rules with analysis_instance_id as key
    pub alert_rules: HashMap<i32, HashSet<AlertRuleModel>>,
    /// fire times with rule_id as key
    pub fire_times: HashMap<i32, Instant>,
}

impl Actor for AlertActor {
    type Context = Context<Self>;
}

impl Default for AlertActor {
    /// defaulter for `AlertActor`
    fn default() -> Self {
        Self {
            alert_rules: HashMap::new(),
            fire_times: HashMap::new(),
        }
    }
}

impl Supervised for AlertActor {}

impl SystemService for AlertActor {
    /// Start handler for `AlertActor`, ensures actor is initialized
    fn service_started(&mut self, ctx: &mut Context<Self>) {
        debug!("Service started");
        ctx.notify(SyncAlertRules {});
    }
}

impl AlertActor {
    /// returns true if an alert rule matches the given observation
    fn rule_matches(rule: &AlertRuleModel, camera_id: i32, obs: &Observation) -> bool {
        rule.matches_camera_id(camera_id)
            && obs.tag == rule.rule().tag
            && obs.details == rule.rule().details
            && obs.score >= rule.rule().min_score
    }

    /// returns true if the given alert rule is active and ready to fire
    fn rule_active(&self, rule: &AlertRuleModel, new_times: &HashMap<i32, Instant>) -> bool {
        //          !! IMPLEMENT CLUSTERING !!!
        let cluster_present = true;
        let new_ready = match new_times.get(&rule.rule().id) {
            Some(instant) => {
                let micros_since = Instant::now().duration_since(*instant).as_micros();
                micros_since
                    >= u128::try_from(rule.rule().cool_down_time).expect("i64 to u128 failed")
            }
            None => true,
        };
        let ready = match self.fire_times.get(&rule.rule().id) {
            Some(instant) => {
                let micros_since = Instant::now().duration_since(*instant).as_micros();
                micros_since
                    >= u128::try_from(rule.rule().cool_down_time).expect("i64 to u128 failed")
            }
            None => true,
        };

        new_ready && ready && cluster_present
    }

    /// Adds a rule to the `AlertActor`
    fn add_rule(&mut self, rule: AlertRuleModel) {
        if let Some(set) = self.alert_rules.get_mut(&rule.rule().analysis_instance_id) {
            set.insert(rule);
        } else {
            let mut set = HashSet::new();
            let id = rule.rule().analysis_instance_id;
            set.insert(rule);
            self.alert_rules.insert(id, set);
        }
    }

    /// generates the url to view the observation
    fn generate_observation_url(obs: &Observation) -> Option<Url> {
        let base_url = match Url::parse(
            &dotenv::var("ROOT_URL").unwrap_or_else(|_| "http://localhost/".to_string()),
        ) {
            Ok(url) => url,
            Err(err) => {
                error!("Error parsing base url: {}", err);
                return None;
            }
        };
        let path = format!("/alerts/{}", obs.id);
        let url = match base_url.join(&path) {
            Ok(url) => url,
            Err(err) => {
                error!("Error joining url: {}", err);
                return None;
            }
        };
        Some(url)
    }

    /// Send a notification
    async fn send_notification(rule: AlertRule, o: Observation) {
        debug!("Sending notification for observation: {}", o.id);
        let url = match Self::generate_observation_url(&o) {
            Some(url) => url.to_string(),
            None => "".to_string(),
        };

        let message = Some(format!(
            "Alert! Alert!\n {} detected with {}% certainty {}",
            o.details, o.score, url
        ));

        NotifierSupervisor::from_registry().do_send(SendNotification {
            notifier_id: rule.notifier_id,
            contact_group: rule.contact_group.clone(),
            message: message.clone(),
            attachment: None,
        });

        // Wait for one second then fetch image. We do this because
        // the frame may not have been written yet.  This has to be
        // fixed to allow trick playing live video as well.
        delay_for(Duration::new(2, 0)).await;
        let image = match fetch_observation_image(o.id).await {
            Ok(img) => Some(img),
            Err(_) => None,
        };
        NotifierSupervisor::from_registry().do_send(SendNotification {
            notifier_id: rule.notifier_id,
            contact_group: rule.contact_group.clone(),
            message: None,
            attachment: image,
        });
    }
}

impl Handler<CameraFrame> for AlertActor {
    type Result = ();

    fn handle(&mut self, msg: CameraFrame, ctx: &mut Self::Context) -> Self::Result {
        // Find alert rules for the source of this frame
        debug!("Got a frame...");
        let analysis_instance_id = match msg.source {
            FrameSource::Camera {
                camera_id: _camera_id,
            } => {
                // We shouldn't get frames from a camera...
                error!(
                    "Received frame from invalid camera source! {:?}",
                    msg.source
                );
                return;
            }
            FrameSource::Playback { id: _id } => {
                // We shouldn't get frames from a playback actor...
                error!(
                    "Received frame from invalid playback source! {:?}",
                    msg.source
                );
                return;
            }
            FrameSource::AnalysisEngine {
                analysis_engine_id,
                tag: _tag,
            } => analysis_engine_id,
        };

        let mut new_fire_times = HashMap::new();
        if let Some(rules) = self.alert_rules.get(&analysis_instance_id) {
            for r in rules.iter() {
                debug!("Checking if rule {} is active", r.rule().id);
                if self.rule_active(r, &new_fire_times) {
                    debug!("rule is active!");
                    for o in &msg.observations {
                        debug!("Checking if {} {} {} matches...", o.tag, o.details, o.score);
                        if Self::rule_matches(r, msg.camera_id, o) {
                            new_fire_times.insert(r.rule().id, Instant::now());
                            debug!("Alert! Alert!");
                            ctx.spawn(wrap_future(Self::send_notification(
                                r.rule().clone(),
                                o.clone(),
                            )));
                        }
                    }
                }
            }
        }

        for (rule_id, time) in &new_fire_times {
            self.fire_times.insert(*rule_id, *time);
        }
    }
}

/// Message requesting `AlertActor` reload rules from database
pub struct SyncAlertRules {}

impl Message for SyncAlertRules {
    type Result = ();
}

impl Handler<SyncAlertRules> for AlertActor {
    type Result = ();

    fn handle(&mut self, _msg: SyncAlertRules, ctx: &mut Context<Self>) -> Self::Result {
        // reload alert rules
        debug!("Syncing alert rules...");
        let fut = async move {
            debug!("In block 1");
            critical_section::<Self, _>(async move {
                debug!("In block 2");
                // remove all references to Notifier workers
                with_ctx(|actor: &mut Self, _| {
                    debug!("In block 3");
                    actor.alert_rules.clear();
                    actor.fire_times.clear();
                });
                debug!("Fetching...");
                // Fetch alert rules
                let alert_rules = match db_registry::get_db().send(FetchAllAlertRule {}).await {
                    Ok(Ok(n)) => n,
                    Ok(Err(_)) | Err(_) => {
                        error!("Failed to load alert rules!");
                        return;
                    }
                };

                debug!("Fetched {} alert rules!", alert_rules.len());

                for r in alert_rules {
                    with_ctx(|actor: &mut Self, ctx: &mut Context<Self>| {
                        let subject =
                            SubscriptionSubject::AnalysisEngine(r.rule().analysis_instance_id);
                        WsCameraServer::from_registry().do_send(Subscribe {
                            subject,
                            client: ctx.address().recipient(),
                        });
                        debug!("Adding alert rule: {}", r.rule().id);
                        actor.add_rule(r);
                    });
                }
            })
            .await;
        }
        .interop_actor(self);
        ctx.spawn(fut);
    }
}
