use crate::errors::ServiceError;
use crate::models::{CreateNotifier, DbExecutor, DeleteNotifier, FetchAllNotifier, Notifier};
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
