//use std::time::Duration;

use actix::prelude::*;
//use actix_interop::{critical_section, with_ctx, FutureInterop};
use actix_interop::{with_ctx, FutureInterop};
use log::debug;
use telegram_bot::{Api, InputFileUpload, SendMessage, SendPhoto, UserId};

use crate::db_registry;
use crate::models::FetchNotificationContactsByGroup;
use crate::notifier_supervisor::SendNotification;

/// Actor implementing Telegram bot
pub struct TelegramActor {
    /// Telegram api
    pub api: Api,
}

impl Actor for TelegramActor {
    type Context = Context<Self>;
}

impl TelegramActor {
    /// Creates a new `TelegramActor` taking an auth token as the only
    /// argument.
    pub fn new(telegram_token: String) -> Self {
        Self {
            api: Api::new(telegram_token),
        }
    }
}

impl Handler<SendNotification> for TelegramActor {
    type Result = ();

    fn handle(&mut self, msg: SendNotification, ctx: &mut Context<Self>) -> Self::Result {
        let fut = async move {
            debug!("Got SendNotification!");
            let contacts = match db_registry::get_db()
                .send(FetchNotificationContactsByGroup {
                    group_name: msg.contact_group,
                })
                .await
            {
                Ok(Ok(contacts)) => contacts,
                Ok(Err(_)) | Err(_) => {
                    error!("Failed to load contacts!");
                    return;
                }
            };

            for c in contacts {
                let user_id = match c.username.parse::<i64>() {
                    Ok(user_id) => UserId::new(user_id),
                    Err(err) => {
                        error!(
                            "Failed to parse telegram user id: {}, {:?}",
                            c.username, err
                        );
                        continue;
                    }
                };

                if let Some(ref message) = msg.message {
                    match with_ctx(|actor: &mut Self, _| {
                        actor.api.send(SendMessage::new(user_id, message))
                    })
                    .await
                    {
                        Ok(result) => {
                            debug!("Telegram notification sent! Response: {:?}", result);
                        }
                        Err(err) => {
                            error!("Failed to send telegram notification: {:?}", err);
                        }
                    }
                }

                if let Some(image) = msg.attachment.clone() {
                    let fileref = InputFileUpload::with_data(image, "snap.jpg");
                    match with_ctx(|actor: &mut Self, _| {
                        actor.api.send(SendPhoto::new(user_id, fileref))
                    })
                    .await
                    {
                        Ok(result) => {
                            debug!("Telegram notification sent! {:?}", result);
                        }
                        Err(err) => {
                            error!("Failed to send telegram notification: {:?}", err);
                        }
                    }
                }
            }
        }
        .interop_actor(self);
        ctx.spawn(fut);
    }
}
