//! The frontend, compiled into the binary.
//!
//! Everything under `static/` is embedded at build time by `rust-embed`
//! (in debug builds it is read from disk instead, so edits to
//! `static/index.html` show up without recompiling). Changing the
//! frontend for a release build therefore requires a rebuild.

use axum::{
    http::{header, StatusCode, Uri},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

/// Files embedded from `static/`.
#[derive(RustEmbed)]
#[folder = "static/"]
struct Assets;

/// Serves the embedded frontend. Any path that isn't a real asset (e.g.
/// /d/<token>, which only exists client-side) falls back to index.html so
/// the single-page app's own router can pick it up.
pub async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if !path.is_empty() {
        if let Some(content) = Assets::get(path) {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            return (
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                content.data.into_owned(),
            )
                .into_response();
        }
    }

    match Assets::get("index.html") {
        Some(content) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8".to_string())],
            content.data.into_owned(),
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}
