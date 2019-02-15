// main.rs
// to avoid the warning from diesel macros
#![allow(proc_macro_derive_resolution_fallback)]
#![warn(clippy::all, clippy::restriction, clippy::pedantic, clippy::cargo)]

extern crate actix;
extern crate actix_web;
extern crate askama;
extern crate base64;
#[macro_use]
extern crate base64_serde;
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
extern crate rand;
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
mod auth_handler;
mod auth_routes;
mod camera_group_handler;
mod camera_group_routes;
mod camera_handler;
mod camera_routes;
mod capture_actor;
mod capture_supervisor;
mod errors;
mod file_deletion_actor;
mod file_deletion_supervisor;
mod models;
mod register_handler;
mod root_supervisor;
mod schema;
mod static_routes;
mod user_routes;
mod utils;
mod video_file_handler;
mod video_unit_handler;
mod video_unit_routes;
mod ws_camera_server;
mod ws_session;

use crate::models::DbExecutor;
use actix::prelude::*;
use actix_web::server;
use base64::encode;
use rand::Rng;
use dialoguer::{Input, PasswordInput};
use diesel::{r2d2::ConnectionManager, PgConnection};
use dotenv::dotenv;
use std::env;

use crate::models::{CreateCameraGroup, CreateUser};
use crate::root_supervisor::{ExopticonMode, RootSupervisor};

fn add_user(
    sys: &mut actix::SystemRunner,
    address: &Addr<DbExecutor>,
) -> Result<bool, std::io::Error> {
    let username = Input::new()
        .with_prompt("Enter username for initial user")
        .interact()?;

    let password = PasswordInput::new()
        .with_prompt("Enter password for initial user")
        .interact()?;

    let fut2 = address.send(CreateUser {
        username: username,
        password: password,
        timezone: String::from("UTC"),
    });

    match sys.block_on(fut2) {
        Ok(_) => (),
        Err(err) => {
            error!("Error creating user! {}", err);
        }
    }
    println!("Created user!");
    Ok(true)
}

fn add_camera_group(
    sys: &mut actix::SystemRunner,
    address: &Addr<DbExecutor>,
) -> Result<bool, std::io::Error> {
    let storage_path = Input::new()
        .with_prompt("Enter storage path for recorded video")
        .interact()?;

    let max_storage_size: i64 = Input::new()
        .with_prompt("Enter max space used at this path, in megabytes")
        .interact()?;

    let fut = address.send(CreateCameraGroup {
        name: String::from("default"),
        storage_path: storage_path,
        max_storage_size: max_storage_size,
    });
    match sys.block_on(fut) {
        Ok(_) => (),
        Err(err) => {
            error!("Error creating camera group! {}", err);
        }
    }
    println!("Created camera group!");
    Ok(true)
}

fn main() {
    env_logger::init();

    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut sys = actix::System::new("Exopticon");

    // create db connection pool
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    let address: Addr<DbExecutor> = SyncArbiter::start(4, move || DbExecutor(pool.clone()));

    let db_address = address.clone();
    let setup_address = address.clone();
    // secret is a random 32 character long base 64 string
    let secret: String = env::var("SECRET_KEY").unwrap_or_else(|_| encode(&rand::thread_rng().gen::<[u8; 24]>()));

    server::new(move || app::create_app(address.clone(), &secret))
        .bind("0.0.0.0:3000")
        .expect("Can not bind to '0.0.0.0:3000'")
        .start();

    let mut mode = ExopticonMode::Run;
    let mut add_user_flag = false;
    let mut add_camera_group_flag = false;
    // Prints each argument on a separate line
    for argument in env::args() {
        match argument.as_ref() {
            "--standby" => {
                info!("Runtime mode is standby...");
                mode = ExopticonMode::Standby;
            }
            "--add-user" => {
                add_user_flag = true;
            }
            "--add-camera-group" => {
                add_camera_group_flag = true;
            }
            _ => (),
        }
    }

    let root_supervisor = RootSupervisor::new(mode, db_address);

    root_supervisor.start();

    if add_user_flag {
        match add_user(&mut sys, &setup_address) {
            Err(_) => {
                println!("Error creating user!");
                return;
            }
            _ => (),
        }
    }

    if add_camera_group_flag {
        match add_camera_group(&mut sys, &setup_address) {
            Err(_) => {
                println!("Error creating camera group!");
                return;
            }
            _ => (),
        }
    }

    sys.run();
}
