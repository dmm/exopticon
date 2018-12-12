use actix::{Handler, Message};
use diesel::prelude::*;

use crate::errors::ServiceError;
use crate::models::{CreateUser, DbExecutor, SlimUser, User};
use crate::utils::hash_password;

// UserData is used to extract data from a post request by the client
#[derive(Debug, Deserialize)]
pub struct UserData {
    pub password: String,
}

impl Message for CreateUser {
    type Result = Result<SlimUser, ServiceError>;
}

impl Handler<CreateUser> for DbExecutor {
    type Result = Result<SlimUser, ServiceError>;
    fn handle(&mut self, msg: CreateUser, _: &mut Self::Context) -> Self::Result {
        use crate::schema::users::dsl::users;
        let conn: &PgConnection = &self.0.get().unwrap();

        let password: String = hash_password(&msg.password)?;
        let user: User = diesel::insert_into(users)
            .values(CreateUser {
                username: msg.username,
                password: password,
                timezone: msg.timezone,
            })
            .get_result(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        return Ok(user.into());
    }
}
