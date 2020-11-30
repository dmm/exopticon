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

use crate::errors::ServiceError;
use crate::models::{
    CreateNotifier, DbExecutor, DeleteNotifier, FetchAllNotifier, FetchNotificationContactsByGroup,
    NotificationContact, Notifier,
};
use actix::{Handler, Message};
use diesel::*;

impl Message for CreateNotifier {
    type Result = Result<Notifier, ServiceError>;
}

impl Handler<CreateNotifier> for DbExecutor {
    type Result = Result<Notifier, ServiceError>;

    fn handle(&mut self, msg: CreateNotifier, _: &mut Self::Context) -> Self::Result {
        use crate::schema::notifiers::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(notifiers)
            .values(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for DeleteNotifier {
    type Result = Result<(), ServiceError>;
}

impl Handler<DeleteNotifier> for DbExecutor {
    type Result = Result<(), ServiceError>;

    fn handle(&mut self, msg: DeleteNotifier, _: &mut Self::Context) -> Self::Result {
        use crate::schema::notifiers::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::delete(notifiers.filter(id.eq(msg.id)))
            .execute(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        Ok(())
    }
}

impl Message for FetchAllNotifier {
    type Result = Result<Vec<Notifier>, ServiceError>;
}

impl Handler<FetchAllNotifier> for DbExecutor {
    type Result = Result<Vec<Notifier>, ServiceError>;

    fn handle(&mut self, _msg: FetchAllNotifier, _: &mut Self::Context) -> Self::Result {
        use crate::schema::notifiers::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        notifiers
            .load::<Notifier>(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for FetchNotificationContactsByGroup {
    type Result = Result<Vec<NotificationContact>, ServiceError>;
}

impl Handler<FetchNotificationContactsByGroup> for DbExecutor {
    type Result = Result<Vec<NotificationContact>, ServiceError>;

    fn handle(
        &mut self,
        msg: FetchNotificationContactsByGroup,
        _: &mut Self::Context,
    ) -> Self::Result {
        use crate::schema::notification_contacts::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        notification_contacts
            .filter(group_name.eq(msg.group_name))
            .limit(1000)
            .load(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}
