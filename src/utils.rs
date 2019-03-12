use bcrypt::{hash, DEFAULT_COST};
use std::env;

use crate::errors::ServiceError;

/// Hashes plaintext password. The number of rounds is determined by
/// the `HASH_ROUNDS` environmental variable or, if that is not defined,
/// `DEFAULT_COST`.
///
/// # Arguments
///
/// `plain` - plaintext password to be hashed
///
pub fn hash_password(plain: &str) -> Result<String, ServiceError> {
    // get the hashing cost from the env variable or use default
    let hashing_cost: u32 = match env::var("HASH_ROUNDS") {
        Ok(cost) => cost.parse().unwrap_or(DEFAULT_COST),
        _ => DEFAULT_COST,
    };
    hash(plain, hashing_cost).map_err(|_| ServiceError::InternalServerError)
}
