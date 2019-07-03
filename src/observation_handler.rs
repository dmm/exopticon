use crate::errors::ServiceError;
use crate::models::{CreateObservations, DbExecutor};
use actix::{Handler, Message};
use diesel::{self, prelude::*};

impl Message for CreateObservations {
    type Result = Result<usize, ServiceError>;
}

impl Handler<CreateObservations> for DbExecutor {
    type Result = Result<usize, ServiceError>;

    fn handle(&mut self, msg: CreateObservations, _: &mut Self::Context) -> Self::Result {
        use crate::schema::observations::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(observations)
            .values(&msg.observations)
            .execute(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}
