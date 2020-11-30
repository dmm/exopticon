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

//! Implements registry for `DbExecutor`

use actix::Addr;
use std::sync::Mutex;

use crate::models::DbExecutor;

lazy_static::lazy_static! {
    static ref DBREG: Mutex<Vec<Addr<DbExecutor>>> = Mutex::new(Vec::new());
}

/// Returns address of `DbExecutor`
///
/// # Panics
///
/// Panics if called before `set_db()`.
///
pub fn get_db() -> Addr<DbExecutor> {
    DBREG
        .lock()
        .expect("DbRegistry: Unable to lock db registry for retrieval.")
        .last()
        .expect("DbRegistry: DB address not set!")
        .clone()
}

/// Set address of `DbExecutor`. Must be called before `get_db()`.
pub fn set_db(db_address: Addr<DbExecutor>) {
    DBREG
        .lock()
        .expect("DbRegistry: unable to set address")
        .push(db_address);
}
