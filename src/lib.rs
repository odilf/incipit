pub mod config;
pub mod drawbridge;
pub(crate) mod util;

pub use config::Config;

use axum::{middleware, Router};
use color_eyre::eyre::{self, Context as _};
use tokio::net::TcpListener;
use std::sync::Arc;

/// Starts incipit.
///
/// Returns when the server stops.
pub async fn run(config: Config) -> eyre::Result<()> {
    let (http_listener, router) = setup(config).await?;

    axum::serve(http_listener, router)
        .await
        .wrap_err("Axum server failed")?;

    Ok(())
}

/// Sets up the server.
///
/// Namely, it binds to the socket specified in the config, watches for file
/// changes and sets up the router with the drawbridge middleware.
pub(crate) async fn setup(config: Config) -> eyre::Result<(TcpListener, Router)> {
    let config = config::watch_config(config)?;

    let router = Router::new().layer(middleware::from_fn_with_state(
        Arc::clone(&config),
        drawbridge::middleware,
    ));

    let socket = config.read().unwrap().socket();
    let http_listener = TcpListener::bind(socket)
        .await
        .wrap_err_with(|| format!("Can't bind to {socket}"))?;

    tracing::info!("listening on {}", socket);

    Ok((http_listener, router))
}
