// main.rs
// to avoid the warning from diesel macros
#![allow(proc_macro_derive_resolution_fallback)]

extern crate actix;
extern crate actix_web;
#[macro_use]
extern crate askama;
extern crate bytes;
extern crate chrono;
extern crate dotenv;
extern crate env_logger;
extern crate futures;
extern crate r2d2;
extern crate serde;
extern crate uuid;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;
extern crate mime_guess;
extern crate rmp;
extern crate rmp_serde;
#[macro_use]
extern crate rust_embed;
extern crate serde_bytes;
extern crate tokio;
extern crate tokio_codec;
extern crate tokio_io;
extern crate tokio_process;

mod app;
mod camera_group_handler;
mod camera_group_routes;
mod camera_handler;
mod camera_routes;
mod capture_actor;
mod capture_supervisor;
mod errors;
mod models;
mod root_supervisor;
mod schema;
mod static_routes;
mod video_file_handler;
mod video_unit_handler;
mod video_unit_routes;
mod ws_camera_server;
mod ws_session;

use actix::prelude::*;
use actix_web::server;
use diesel::{r2d2::ConnectionManager, PgConnection};
use dotenv::dotenv;
use models::DbExecutor;
use std::env;

use root_supervisor::{ExopticonMode, RootSupervisor};

fn main() {
    env_logger::init();

    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let sys = actix::System::new("Exopticon");

    // create db connection pool
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let address: Addr<DbExecutor> = SyncArbiter::start(4, move || DbExecutor(pool.clone()));

    let db_address = address.clone();

    server::new(move || app::create_app(address.clone()))
        .bind("0.0.0.0:3000")
        .expect("Can not bind to '0.0.0.0:3000'")
        .start();

    let root_supervisor = RootSupervisor::new(ExopticonMode::Run, db_address);

    root_supervisor.start();

    sys.run();
}
