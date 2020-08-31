use std::collections::HashMap;

use actix::prelude::*;
use actix_interop::{critical_section, with_ctx, FutureInterop};

use crate::db_registry;
use crate::models::FetchAllAlertRule;

pub struct AlertSupervisor {
    workers: HashMap<i32, Addr<MqttActor>>,
}

impl Actor for NotifierSupervisor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        ctx.notify(SyncNotifiers {});
    }
}

impl Default for NotifierSupervisor {
    fn default() -> Self {
        Self {
            workers: HashMap::new(),
        }
    }
}

impl Supervised for NotifierSupervisor {}

impl SystemService for NotifierSupervisor {}

pub struct SyncNotifiers {}

impl Message for SyncNotifiers {
    type Result = ();
}

impl Handler<SyncNotifiers> for NotifierSupervisor {
    type Result = ();

    fn handle(&mut self, _msg: SyncNotifiers, _ctx: &mut Context<Self>) -> Self::Result {
        // fetch notifiers
        async move {
            critical_section::<Self, _>(async move {
                // remove all references to Notifier workers
                with_ctx(|actor: &mut Self, _| {
                    actor.workers.clear();
                });

                // Fetch notifiers
                let notifiers = match db_registry::get_db().send(FetchAllNotifier {}).await {
                    Ok(Ok(n)) => n,
                    Ok(Err(_)) | Err(_) => return,
                };

                for n in notifiers {
                    let address = MqttActor::new(
                        n.hostname.clone(),
                        n.port as u16,
                        n.username.clone(),
                        n.password.clone(),
                    )
                    .start();
                    with_ctx(|actor: &mut Self, _| {
                        actor.workers.insert(n.id, address);
                    });
                }
            })
            .await;
        }
        .interop_actor(self);
    }
}
