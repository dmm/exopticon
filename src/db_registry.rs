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
