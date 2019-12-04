use actix::{Handler, Message};
use bcrypt::verify;
use diesel::prelude::*;

use crate::errors::ServiceError;
use crate::models::{DbExecutor, SlimUser, User};

/// Represents data for an authentication attempt
#[derive(Debug, Deserialize)]
pub struct AuthData {
    /// username
    pub username: String,
    /// plaintext password
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
            .map_err(|error| {
                error!("Unable to load users! {}", error);
                ServiceError::InternalServerError
            })?;

        if let Some(user) = items.pop() {
            if let Ok(matching) = verify(&msg.password, &user.password) {
                if matching {
                    return Ok(user.into());
                }
            } else {
                return mismatch_error;
            }
        }
        mismatch_error
    }
}
