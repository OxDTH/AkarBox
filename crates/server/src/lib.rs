//! HTTP layer for AkarBox, built on [axum](https://docs.rs/axum).
//!
//! | Method | Path                 | What it does                                      |
//! |--------|----------------------|---------------------------------------------------|
//! | GET    | `/api/info`          | Version, available converters, format catalogue  |
//! | GET    | `/api/targets/{ext}` | Formats a given input format can be converted to   |
//! | POST   | `/api/detect?name=`  | First bytes of a file -> its format + targets      |
//! | POST   | `/api/convert`       | Multipart (`file`, `target`, `options`) -> converted file (or zip) |
//! | POST   | `/api/pdf/merge`     | Multipart, several `file`s -> one merged PDF       |
//! | POST   | `/api/pdf/split`     | Multipart (`file`, `pages`) -> PDF with only those pages |
//! | GET    | everything else      | The embedded web UI                               |

mod api;
mod assets;
mod error;

use akarbox_core::Registry;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tower_http::trace::TraceLayer;

/// Runtime settings, read from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// `AKARBOX_ADDR`, default `0.0.0.0:8080`.
    pub addr: String,
    /// `AKARBOX_MAX_UPLOAD_MB`, default 1024.
    pub max_upload_bytes: usize,
    /// `AKARBOX_MAX_JOBS`: conversions allowed to run at once, default = CPU count.
    pub max_jobs: usize,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        // A small closure (anonymous function) that reads and parses a variable.
        let num = |name: &str, default: usize| -> anyhow::Result<usize> {
            match std::env::var(name) {
                Ok(v) => Ok(v.parse()?),
                Err(_) => Ok(default),
            }
        };
        let cpus = std::thread::available_parallelism().map_or(2, |n| n.get());
        Ok(Self {
            addr: std::env::var("AKARBOX_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            max_upload_bytes: num("AKARBOX_MAX_UPLOAD_MB", 1024)? * 1024 * 1024,
            max_jobs: num("AKARBOX_MAX_JOBS", cpus)?.max(1),
        })
    }
}

/// Shared state handed to every request handler.
///
/// `Arc` = "atomically reference-counted pointer": cheap to clone, and every clone
/// points at the same data. That's how all requests share one registry.
#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<Registry>,
    /// Limits how many conversions run at the same time so a burst of uploads
    /// can't overload the machine.
    pub jobs: Arc<Semaphore>,
    /// Is qpdf installed (needed for PDF merge/split)?
    pub pdf_tools: bool,
}

pub fn router(state: AppState, config: &Config) -> Router {
    Router::new()
        .route("/api/info", get(api::info))
        .route("/api/targets/{ext}", get(api::targets))
        // Detection only needs the first few KB of a file.
        .route(
            "/api/detect",
            post(api::detect).layer(DefaultBodyLimit::max(64 * 1024)),
        )
        .route("/api/convert", post(api::convert))
        .route("/api/pdf/merge", post(api::pdf_merge))
        .route("/api/pdf/split", post(api::pdf_split))
        .layer(DefaultBodyLimit::max(config.max_upload_bytes))
        .fallback(assets::static_handler)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Build the registry and serve until the process is stopped.
pub async fn serve(listener: tokio::net::TcpListener, config: Config) -> std::io::Result<()> {
    // Probing for tools touches the filesystem, so do it off the async threads.
    let (registry, pdf_tools) =
        tokio::task::spawn_blocking(|| (Registry::with_builtin(), akarbox_core::pdf::available()))
            .await
            .expect("registry setup panicked");
    if !pdf_tools {
        tracing::warn!("qpdf not installed: PDF merge/split disabled");
    }
    let state = AppState {
        registry: Arc::new(registry),
        jobs: Arc::new(Semaphore::new(config.max_jobs)),
        pdf_tools,
    };
    axum::serve(listener, router(state, &config)).await
}
