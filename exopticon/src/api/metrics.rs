use axum::{routing::get, Router};
use futures::future::ready;
use metrics_exporter_prometheus::PrometheusBuilder;

use crate::AppState;

pub fn router() -> Router<AppState> {
    let builder = PrometheusBuilder::new();

    let handle = builder
        .install_recorder()
        .expect("failed to install recorder");

    Router::<AppState>::new().route("/", get(move || ready(handle.render())))
}
