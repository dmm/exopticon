use actix::{Handler, Message};
use actix_web::{middleware::identity::RequestIdentity, FromRequest, HttpRequest};
use bcrypt::verify;
use diesel::prelude::*;

use crate::errors::ServiceError;
use crate::models::{DbExecutor, SlimUser, User};

#[derive(Debug, Deserialize)]
pub struct AuthData {
    pub username: String,
    pub password: String,
}

impl Message for AuthData {
    type Result = Result<SlimUser, ServiceError>;
}

impl Handler<AuthData> for DbExecutor {
    type Result = Result<SlimUser, ServiceError>;
    fn handle(&mut self, msg: AuthData, _: &mut Self::Context) -> Self::Result {
        use crate::schema::users::dsl::{username, users};
        let conn: &PgConnection = &self.0.get().unwrap();
        let mismatch_error = Err(ServiceError::BadRequest(
            "Username and Password don't match".into(),
        ));

        let mut items = users
            .filter(username.eq(&msg.username))
            .load::<User>(conn)
            .map_err(|_error| ServiceError::InternalServerError)?;

        if let Some(user) = items.pop() {
            match verify(&msg.password, &user.password) {
                Ok(matching) => {
                    if matching {
                        return Ok(user.into());
                    } else {
                        return mismatch_error;
                    }
                }
                Err(_) => {
                    return mismatch_error;
                }
            }
        }
        mismatch_error
    }
}

pub type LoggedUser = SlimUser;
