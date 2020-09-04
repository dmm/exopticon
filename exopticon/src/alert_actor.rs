use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::time::Instant;

use actix::prelude::*;
use actix_interop::{critical_section, with_ctx, FutureInterop};
use url::Url;

use crate::db_registry;
use crate::models::{AlertRule, FetchAllAlertRule, Observation};
use crate::notifier_supervisor::{NotifierSupervisor, SendNotification};
use crate::ws_camera_server::{
    CameraFrame, FrameSource, Subscribe, SubscriptionSubject, WsCameraServer,
};

/// Actor that implements observation alerts
pub struct AlertActor {
    /// Alert Rules with analysis_instance_id as key
    pub alert_rules: HashMap<i32, HashSet<AlertRule>>,
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
    fn rule_matches(rule: &AlertRule, obs: &Observation) -> bool {
        obs.tag == rule.tag && obs.details == rule.details && obs.score >= rule.min_score
    }

    /// returns true if the given alert rule is active and ready to fire
    fn rule_active(&self, rule: &AlertRule, new_times: &HashMap<i32, Instant>) -> bool {
        //          !! IMPLEMENT CLUSTERING !!!
        let cluster_present = true;
        let new_ready = match new_times.get(&rule.id) {
            Some(instant) => {
                let micros_since = Instant::now().duration_since(*instant).as_micros();
                micros_since >= u128::try_from(rule.cool_down_time).expect("i64 to u128 failed")
            }
            None => true,
        };
        let ready = match self.fire_times.get(&rule.id) {
            Some(instant) => {
                let micros_since = Instant::now().duration_since(*instant).as_micros();
                micros_since >= u128::try_from(rule.cool_down_time).expect("i64 to u128 failed")
            }
            None => true,
        };

        new_ready && ready && cluster_present
    }

    /// Adds a rule to the `AlertActor`
    fn add_rule(&mut self, rule: AlertRule) {
        if let Some(set) = self.alert_rules.get_mut(&rule.id) {
            set.insert(rule);
        } else {
            let mut set = HashSet::new();
            let id = rule.analysis_instance_id;
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
}

impl Handler<CameraFrame> for AlertActor {
    type Result = ();

    fn handle(&mut self, msg: CameraFrame, _ctx: &mut Self::Context) -> Self::Result {
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
                debug!("Checking if rule {} is active", r.id);
                if self.rule_active(r, &new_fire_times) {
                    debug!("rule is active!");
                    for o in &msg.observations {
                        debug!("Checking if {} {} {} matches...", o.tag, o.details, o.score);
                        if Self::rule_matches(r, o) {
                            new_fire_times.insert(r.id, Instant::now());
                            debug!("Alert! Alert!");
                            let url = match Self::generate_observation_url(o) {
                                Some(url) => url.to_string(),
                                None => "".to_string(),
                            };
                            NotifierSupervisor::from_registry().do_send(SendNotification {
                                notifier_id: r.notifier_id,
                                topic: "/home/exopticon/alert".to_string(),
                                payload: format!("{} {} {} {}", o.tag, o.details, o.score, url),
                            });
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
                        let subject = SubscriptionSubject::AnalysisEngine(r.analysis_instance_id);
                        WsCameraServer::from_registry().do_send(Subscribe {
                            subject,
                            client: ctx.address().recipient(),
                        });
                        debug!("Adding alert rule: {}", r.id);
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
