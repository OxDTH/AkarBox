//! `akarbox-server`: the binary you run on a home server or in Docker.

use akarbox_server::Config;
use tracing_subscriber::EnvFilter;

// `#[tokio::main]` starts the async runtime and runs our async `main` on it.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env()?;
    let listener = tokio::net::TcpListener::bind(&config.addr).await?;
    tracing::info!("AkarBox listening on http://{}", listener.local_addr()?);

    akarbox_server::serve(listener, config).await?;
    Ok(())
}
