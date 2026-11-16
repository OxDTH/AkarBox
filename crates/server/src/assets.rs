//! Serves the web UI. The files in `frontend/dist` are compiled *into* the binary,
//! so the server is a single self-contained file.

use axum::http::{header, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../frontend/dist"]
struct Assets;

pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(file) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            // Vite puts a content hash in asset file names, so they can be cached forever.
            let cache = if path.starts_with("assets/") {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            (
                [
                    (header::CONTENT_TYPE, mime.as_ref()),
                    (header::CACHE_CONTROL, cache),
                ],
                file.data,
            )
                .into_response()
        }
        None if path.starts_with("api/") => StatusCode::NOT_FOUND.into_response(),
        // Unknown path: serve the app shell (or a hint if the UI wasn't built).
        None => match Assets::get("index.html") {
            Some(index) => Html(index.data).into_response(),
            None => Html(
                "<h1>AkarBox</h1><p>The web UI has not been built. Run \
                 <code>npm --prefix frontend run build</code> and rebuild the server.</p>",
            )
            .into_response(),
        },
    }
}
