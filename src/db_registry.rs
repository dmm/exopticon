use actix::Addr;
use std::sync::Mutex;

use crate::models::DbExecutor;

lazy_static::lazy_static! {
    static ref DBREG: Mutex<Vec<Addr<DbExecutor>>> = Mutex::new(Vec::new());
}

pub fn get_db() -> Addr<DbExecutor> {
    DBREG
        .lock()
        .unwrap()
        .last()
        .expect("DB address not set!")
        .clone()
}

pub fn set_db(db_address: Addr<DbExecutor>) {
    DBREG.lock().unwrap().push(db_address);
}
