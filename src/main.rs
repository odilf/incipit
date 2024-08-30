use axum::{
    extract::{Host, Request, State},
    middleware::Next,
    response::IntoResponse as _,
    Router,
};
use color_eyre::eyre::{self, Context as _};
use hyper::StatusCode;
use incipit::{config::RuntimeConfig, drawbridge};

use std::sync::{Arc, RwLock};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    setup_tracing_and_eyre()?;

    let config = incipit::read_config()?;
    tracing::info!("Loaded config {config:#?}");

    let config = incipit::config::watch_config(config)?;

    let uoh_drawbridge = Router::new().layer(axum::middleware::from_fn_with_state(
        Arc::clone(&config),
        move |State(config): State<Arc<RwLock<RuntimeConfig>>>,
              host: Host,
              request: Request,
              _: Next| async move {
            match drawbridge::route_traffic(host, request, config).await {
                Ok(response) => response,
                Err(error) => {
                    tracing::warn!(?error);
                    (StatusCode::INTERNAL_SERVER_ERROR, "500 - Incipit error").into_response()
                }
            }
        },
    ));

    let socket = config.read().unwrap().socket();
    let http_listener = tokio::net::TcpListener::bind(socket)
        .await
        .wrap_err_with(|| format!("Can't bind to {socket}"))?;

    tracing::info!("listening on {}", socket);

    axum::serve(http_listener, uoh_drawbridge.clone())
        .await
        .wrap_err("Axum server failed")?;

    Ok(())
}

fn setup_tracing_and_eyre() -> eyre::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "incipit=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    color_eyre::install()?;

    Ok(())
}
