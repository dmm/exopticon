use axum::{
    Router,
    extract::{OriginalUri, Path},
    http::header,
    response::{IntoResponse, Response},
    routing::get,
};
use rust_embed::RustEmbed;

use super::UserError;

#[derive(RustEmbed)]
#[folder = "web/dist/browser"]
struct Asset;

pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route("/index.html", get(index_file_handler))
        .route("/manifest.webmanifest", get(manifest_file_handler))
        .route("/ngsw-worker.js", get(root_static_file_handler))
        .route("/ngsw.json", get(root_static_file_handler))
        .route("/safety-worker.js", get(root_static_file_handler))
        .route("/worker-basic.min.js", get(root_static_file_handler))
        .route("/favicon.ico", get(root_static_file_handler))
        .route("/assets/{*path}", get(asset_file_handler))
        .route("/", get(index_file_handler))
        .route("/{*path}", get(frontend_file_handler))
}

#[allow(clippy::unused_async)]
async fn root_static_file_handler(
    OriginalUri(uri): OriginalUri,
) -> Result<impl IntoResponse, UserError> {
    let response = static_file(uri.path().trim_start_matches('/'))?;
    Ok(([(header::CACHE_CONTROL, "no-cache")], response))
}

#[allow(clippy::unused_async)]
async fn asset_file_handler(Path(path): Path<String>) -> Result<Response, UserError> {
    static_file(&format!("assets/{path}"))
}

async fn frontend_file_handler(Path(path): Path<String>) -> Result<Response, UserError> {
    match static_file(&path) {
        // Only missing files at the build root, such as JavaScript
        // bundles, should return 404 here.
        Err(UserError::NotFound)
            if path.contains('/') || std::path::Path::new(&path).extension().is_none() =>
        {
            index_file_handler().await.map(IntoResponse::into_response)
        }
        response => response,
    }
}

/// Route handler for index.html
#[allow(clippy::unused_async)]
pub async fn index_file_handler() -> Result<impl IntoResponse, UserError> {
    Asset::get("index.html").map_or(Err(UserError::NotFound), |content| {
        Ok(([(header::CONTENT_TYPE, "text/html")], content.data))
    })
}

#[allow(clippy::unused_async)]
pub async fn manifest_file_handler() -> Result<impl IntoResponse, UserError> {
    Asset::get("manifest.webmanifest").map_or(Err(UserError::NotFound), |content| {
        Ok((
            [(header::CONTENT_TYPE, "application/manifest+json")],
            content.data,
        ))
    })
}

fn static_file(path: &str) -> Result<Response, UserError> {
    let content_type = mime_guess::from_path(path).first_or_octet_stream();
    Asset::get(path).map_or(Err(UserError::NotFound), |content| {
        Ok((
            [(header::CONTENT_TYPE, content_type.to_string())],
            content.data,
        )
            .into_response())
    })
}
