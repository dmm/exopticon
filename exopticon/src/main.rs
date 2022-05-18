/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020-2022 David Matthew Mattli <dmm@mattli.us>
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

//! Exopticon is a free video surveillance system

// to avoid the warning from diesel macros
#![allow(proc_macro_derive_resolution_fallback)]
#![deny(
    nonstandard_style,
    warnings,
    rust_2018_idioms,
    unused,
    future_incompatible,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]
#![allow(clippy::integer_arithmetic)]
#![allow(clippy::integer_division)]
#![allow(clippy::missing_inline_in_public_items)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::implicit_return)]
#![allow(clippy::print_stdout)]
#![allow(clippy::expect_used)]
#![allow(clippy::future_not_send)]
#![allow(clippy::missing_errors_doc)] // TODO: Fix this one
#![allow(clippy::wildcard_imports)] // TODO: Fix DB handlers

#[macro_use]
extern crate base64_serde;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;

/// Alert rule actor
mod alert_actor;

/// Alert rule db handlers
mod alert_rule_handler;

/// Alert rule routes
mod alert_rule_routes;

/// Actix route specification
mod app;

/// Implements authentication logic
mod auth_handler;

/// Implements auth routes
mod auth_routes;

/// Implements analysis actor
mod analysis_actor;

/// Implements db handlers for analysis
mod analysis_handler;

/// Implements analysis routes
mod analysis_routes;

/// Implements analysis supervisor
mod analysis_supervisor;

/// implements storage group api logic
mod storage_group_handler;

/// Implements storage group routes
mod storage_group_routes;

/// Implements camera api logic
mod camera_handler;

/// Implements camera api routes
mod camera_routes;

/// Actor that captures video from a camera
mod capture_actor;

/// Actor that supervises capture actors
mod capture_supervisor;

mod db_registry;

/// Error type
mod errors;

/// `FairQueue` implementation
mod fair_queue;

/// Actor that deletes excess files for a storage group
mod file_deletion_actor;

/// Actor that supervises files deletion workers
mod file_deletion_supervisor;

/// Implements handler for file io
mod file_handler;

/// Actor message structs
mod models;

/// Notifier db handlers
mod notifier_handler;

/// Implemenents `DbExecutor` handler for creating and querying observations.
mod observation_handler;

///
mod observation_routes;

/// Implements playback actor
mod playback_actor;

/// Implements playback supervisor
mod playback_supervisor;

/// Implements prometheus registry
mod prom_registry;

/// Implements `DbExecutor` handler for creating users
mod register_handler;

/// Root supervisor, lauches `CaptureSupervisor` and `DeletionSupervisor`
mod root_supervisor;

/// Struct map writer so rmps-serde will output maps
mod struct_map_writer;

/// Database schema, generated by diesel
mod schema;

/// Routes for handling static files
mod static_routes;

/// Routes for handling users
mod user_routes;

/// Utility functions
mod utils;

/// Implements handlers for `DbExecutor` concerning `VideoFile`s
mod video_file_handler;

/// Implements handlers for `DbExecutor` concerning `VideoUnit`s
mod video_unit_handler;

/// Implements routes for video units
mod video_unit_routes;

/// Implements camera frame pub/sub
mod ws_camera_server;

/// Implements a websocket session
mod ws_session;

use crate::models::DbExecutor;
use actix::prelude::*;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::cookie::SameSite;
use actix_web::web::Data;
use actix_web::{middleware::Logger, App, HttpServer};
use actix_web_prom::PrometheusMetricsBuilder;
use dialoguer::{Input, PasswordInput};
use diesel::{r2d2::ConnectionManager, PgConnection};
use dotenv::dotenv;
use time::Duration;

use std::collections::HashMap;
use std::env;

use crate::app::RouteState;
use crate::models::{CreateStorageGroup, CreateUser};
use crate::root_supervisor::{ExopticonMode, RootSupervisor};

embed_migrations!("migrations/");

/// Interactively prompts the operator and adds a new user with the
/// details provided. This is for bootstrapping users on a new
/// install. It should be run before the main system is started.
///
/// # Arguments
///
/// * `sys` - The actix system runner
/// * `address` - The address of the `DbExecutor`
///

async fn add_user(address: &Addr<DbExecutor>) -> Result<bool, std::io::Error> {
    let username = Input::new()
        .with_prompt("Enter username for initial user")
        .interact()?;

    let password = PasswordInput::new()
        .with_prompt("Enter password for initial user")
        .with_confirmation("Confirm password", "Passwords mismatching")
        .interact()?;

    let fut2 = address.send(CreateUser {
        username,
        password,
        timezone: String::from("UTC"),
    });

    match fut2.await {
        Ok(_) => (),
        Err(err) => {
            error!("Error creating user! {}", err);
        }
    }
    println!("Created User!");
    Ok(true)
}

/// Adds a storage group. This is really only for setting up initial
/// storage groups for bootstrapping.. It should be run before the full
/// system is started.
///
/// # Arguments
///
/// * `sys` - The actix system runner
/// * `address` - The address of the `DbExecutor`
///

async fn add_storage_group(address: &Addr<DbExecutor>) -> Result<bool, std::io::Error> {
    let storage_path = Input::new()
        .with_prompt("Enter storage path for recorded video")
        .interact()?;

    let max_storage_size: i64 = Input::new()
        .with_prompt("Enter max space used at this path, in megabytes")
        .interact()?;

    let fut = address.send(CreateStorageGroup {
        name: String::from("default"),
        storage_path,
        max_storage_size,
    });
    match fut.await {
        Ok(_) => (),
        Err(err) => {
            error!("Error creating storage group! {}", err);
        }
    }
    println!("Created storage group!");
    Ok(true)
}

#[actix_web::main]
async fn main() {
    env_logger::init();

    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // create db connection pool
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");

    // Run migrations
    info!("Running migrations...");
    embedded_migrations::run_with_output(
        &pool.get().expect("migration connection failed"),
        &mut std::io::stdout(),
    )
    .expect("migrations failed!");

    let address: Addr<DbExecutor> = SyncArbiter::start(4, move || DbExecutor(pool.clone()));

    db_registry::set_db(address.clone());

    let db_address = address.clone();
    let route_db_address = address.clone();
    let setup_address = address;
    let secret: [u8; 32] = [0; 32];

    // Initialize prometheus metrics
    let hostname = env::var("DOMAIN").unwrap_or_else(|_| "exopticon".to_string());
    let mut labels = HashMap::new();
    labels.insert("instance".to_string(), hostname);
    let mut prometheus_builder = PrometheusMetricsBuilder::new("exopticon");
    if let Ok(val) = env::var("EXOPTICON_METRICS_ENABLED") {
        if &val == "true" {
            prometheus_builder = prometheus_builder.endpoint("/metrics");
        }
    }

    let prometheus = prometheus_builder
        .const_labels(labels)
        .build()
        .expect("failed to build prometheus");
    prom_registry::set_metrics(prometheus.clone());

    let mut mode = ExopticonMode::Run;
    let mut add_user_flag = false;
    let mut add_storage_group_flag = false;
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
            "--add-storage-group" => {
                add_storage_group_flag = true;
            }
            _ => (),
        }
    }

    if add_user_flag || add_storage_group_flag {
        mode = ExopticonMode::Standby;
    }

    let arbiter = actix::Arbiter::new();
    let root_supervisor = RootSupervisor::new(mode, db_address);

    RootSupervisor::start_in_arbiter(
        &arbiter.handle(),
        move |_ctx: &mut Context<RootSupervisor>| root_supervisor,
    );

    if add_user_flag || add_storage_group_flag {
        if add_user_flag && add_user(&setup_address).await.is_err() {
            error!("Error creating user!");
            return;
        }

        if add_storage_group_flag && add_storage_group(&setup_address).await.is_err() {
            error!("Error creating  group!");
            return;
        }
        return;
    }

    HttpServer::new(move || {
        App::new()
            .wrap(prometheus.clone())
            .app_data(Data::new(RouteState {
                db: route_db_address.clone(),
            }))
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&secret)
                    .name("id")
                    .path("/")
                    .max_age_secs(Duration::days(7).whole_seconds())
                    .secure(true)
                    .same_site(SameSite::Strict),
            ))
            // setup builtin logger to get nice logging for each request
            .wrap(Logger::new("%{r}a %r %s %b %{Referer}i %{User-Agent}i %T"))
            .configure(app::generate_config)
    })
    .bind("0.0.0.0:3000")
    .expect("Can not bind to '0.0.0.0:3000'")
    .run()
    .await
    .expect("Http server exited");
}
