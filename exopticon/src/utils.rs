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
        Err(_) => DEFAULT_COST,
    };
    hash(plain, hashing_cost).map_err(|_| ServiceError::InternalServerError)
}
