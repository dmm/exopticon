use crate::errors::ServiceError;
use crate::models::{AlertRule, CreateAlertRule, DbExecutor, FetchAllAlertRule};
use actix::{Handler, Message};
use diesel::*;

impl Message for CreateAlertRule {
    type Result = Result<AlertRule, ServiceError>;
}

impl Handler<CreateAlertRule> for DbExecutor {
    type Result = Result<AlertRule, ServiceError>;

    fn handle(&mut self, msg: CreateAlertRule, _: &mut Self::Context) -> Self::Result {
        use crate::schema::alert_rules::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        diesel::insert_into(alert_rules)
            .values(&msg)
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}

impl Message for FetchAllAlertRule {
    type Result = Result<Vec<AlertRule>, ServiceError>;
}

impl Handler<FetchAllAlertRule> for DbExecutor {
    type Result = Result<Vec<AlertRule>, ServiceError>;

    fn handle(&mut self, _msg: FetchAllAlertRule, _: &mut Self::Context) -> Self::Result {
        use crate::schema::alert_rules::dsl::*;
        let conn: &PgConnection = &self.0.get().unwrap();

        alert_rules
            .load::<AlertRule>(conn)
            .map_err(|_error| ServiceError::InternalServerError)
    }
}
