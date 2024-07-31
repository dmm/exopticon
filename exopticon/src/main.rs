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
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::integer_division)]
#![allow(clippy::missing_inline_in_public_items)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::implicit_return)]
#![allow(clippy::print_stdout)]
#![allow(clippy::expect_used)]
#![allow(clippy::future_not_send)]
#![allow(clippy::wildcard_imports)] // TODO: Fix DB handlers

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

/// Api Application implementation
mod api;

/// implements business logic
mod business;

/// Implements database infrastructure
mod db;

/// Error type
mod errors;

/// Database schema, generated by diesel
mod schema;

/// Utility functions
mod utils;

mod capture_actor;
mod capture_supervisor;
mod file_deletion_actor;
mod file_deletion_supervisor;
mod webrtc_client;

use crate::api::static_files::{index_file_handler, manifest_file_handler, static_file_handler};
use crate::api::{auth, camera_groups, cameras, storage_groups, video_units};
use crate::file_deletion_supervisor::FileDeletionSupervisor;

use axum::routing::{get, post};
use axum::{middleware, Router};
use axum_prometheus::PrometheusMetricLayer;
use capture_actor::VideoPacket;
use capture_supervisor::Command;
use dotenv::dotenv;
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, mpsc};
use tower_http::trace::{self, TraceLayer};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing_subscriber::{EnvFilter, Layer};

use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// size of webrtc udp send/recv buffers if not set with env variable
static DEFAULT_BUFFER_SIZE: usize = 2_097_152;

embed_migrations!("migrations/");

#[derive(Clone)]
pub struct AppState {
    pub candidate_ips: Vec<String>,
    pub udp_socket: Arc<UdpSocket>,
    pub udp_channel: broadcast::Sender<(usize, SocketAddr, Vec<u8>)>,
    pub db_service: crate::db::Service,
    pub capture_channel: mpsc::Sender<Command>,
    pub video_sender: broadcast::Sender<VideoPacket>,
}

fn parse_candidate_ips() -> Vec<String> {
    let mut candidate_ips = Vec::new();

    let Ok(candidate_string) = env::var("EXOPTICON_WEBRTC_IPS") else {
        return candidate_ips;
    };
    debug!("CANDIDATES: {candidate_string}");
    for c in candidate_string.split(',') {
        candidate_ips.push(c.to_string());
    }

    candidate_ips
}

async fn udp_listener(
    udp_socket: Arc<UdpSocket>,
    udp_channel: broadcast::Sender<(usize, SocketAddr, Vec<u8>)>,
) {
    let mut buf = vec![0; 2000];
    loop {
        match udp_socket.recv_from(&mut buf).await {
            Ok((len, addr)) => {
                udp_channel.send((len, addr, buf.clone())).unwrap();
            }
            Err(err) => {
                error!("UDP error! {}", err);
            }
        }
    }
}

#[tokio::main]
async fn main() {
    //    console_subscriber::init();

    //    let console_layer = console_subscriber::spawn();
    let filter = EnvFilter::from_default_env();
    tracing_subscriber::registry()
        //        .with(console_layer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_filter(filter),
        )
        .init();
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // create db connection pool
    let db_service = crate::db::Service::new(&database_url);
    let pool = match db_service.clone().pool {
        crate::db::ServiceKind::Real(p) => p,
        crate::db::ServiceKind::Null(_) => {
            panic!("Tried to start Exopticon with a null db pool!")
        }
    };

    // Run migrations
    info!("Running migrations...");
    embedded_migrations::run_with_output(
        &pool.get().expect("migration connection failed"),
        &mut std::io::stdout(),
    )
    .expect("migrations failed!");

    let buffer_size: usize = env::var("EXOPTICON_WEBRTC_BUFFER_SIZE")
        .unwrap_or_default()
        .parse()
        .map_or_else(
            |_| {
                info!(
                    "UDP buffer size {}. parsing env failed, using default size",
                    DEFAULT_BUFFER_SIZE
                );
                DEFAULT_BUFFER_SIZE
            },
            |buffer_size| {
                info!("UDP buffer size {}, parsed from env", buffer_size);
                buffer_size
            },
        );
    error!(
        "Creating webrtc udp socket with send/recv buffer size: {}",
        buffer_size
    );
    let tokio_socket = UdpSocket::bind(("0.0.0.0", 4000))
        .await
        .expect("Unable to open udp socket");
    let ext_socket: socket2::Socket = tokio_socket.into_std().expect("socket into_std").into();
    ext_socket
        .set_recv_buffer_size(buffer_size)
        .expect("setting socket recv buffer size");
    ext_socket
        .set_send_buffer_size(buffer_size)
        .expect("setting socket send buffer size");
    let tokio_socket2 = UdpSocket::from_std(ext_socket.into()).expect("converting to tokio socket");
    let udp_socket = Arc::new(tokio_socket2);

    // Start udp listener
    let (udp_channel, _rx) = broadcast::channel(10);
    tokio::spawn(udp_listener(udp_socket.clone(), udp_channel.clone()));

    // Start capture supervisor
    let capture_supervisor = capture_supervisor::CaptureSupervisor::new(db_service.clone());
    let capture_channel = capture_supervisor.get_command_channel();

    let deletion_supervisor = FileDeletionSupervisor::new(db_service.clone());

    let state = AppState {
        candidate_ips: parse_candidate_ips(),
        udp_socket,
        udp_channel,
        db_service,
        capture_channel,
        video_sender: capture_supervisor.get_packet_sender(),
    };

    // TODO: watch this future for exit...
    info!("Launching capture supervisor...");
    tokio::spawn(capture_supervisor.supervise());
    tokio::spawn(deletion_supervisor.supervise());

    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();

    let app = Router::new()
        .nest(
            "/v1/personal_access_tokens",
            auth::personal_access_token_router(),
        )
        .nest("/v1/storage_groups", storage_groups::router())
        .nest("/v1/camera_groups", camera_groups::router())
        .nest("/v1/cameras", cameras::router())
        .nest("/v1/video_units", video_units::router())
        .nest("/v1/webrtc", crate::api::webrtc::router())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::middleware,
        ))
        // metrics
        .route(
            "/metrics",
            get(|| async move { metric_handle.render() }).layer(middleware::from_fn(
                crate::api::basic_auth_middleware::metrics_auth_middleware,
            )),
        )
        // public routes
        .route("/auth", post(auth::login))
        .route("/index.html", get(index_file_handler))
        .route("/manifest.webmanifest", get(manifest_file_handler))
        .route("/assets/*path", get(static_file_handler))
        .route("/icons/:path", get(static_file_handler))
        .route("/", get(index_file_handler))
        .route("/*path", get(index_file_handler))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(prometheus_layer);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("to listen on 0.0.0.0:3000");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("to start server");
}
