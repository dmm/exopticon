use axum::{http::header, response::IntoResponse};
use rust_embed::RustEmbed;

use super::UserError;

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Asset;

/// Route handler for index.html
pub async fn index_file_handler() -> Result<impl IntoResponse, UserError> {
    Asset::get("index.html").map_or(Err(UserError::NotFound), |content| {
        Ok(([(header::CONTENT_TYPE, "text/html")], content))
    })
}

/// route handler for static files
pub async fn static_file_handler(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Result<impl IntoResponse, UserError> {
    let content_type = mime_guess::from_path(&path).first_or_octet_stream().clone();
    Asset::get(&path).map_or(Err(UserError::NotFound), |content| {
        Ok((
            [(header::CONTENT_TYPE, content_type.to_string())],
            content.into_owned(),
        ))
    })
}
