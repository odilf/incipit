pub mod client;
mod server;
mod service;

pub use client::fetch;
pub use server::{Server, WebSocketServer};
pub use service::{services, start_services, Service};

use crate::Config;

use color_eyre::eyre;
use tokio::task::JoinHandle;

pub fn example_config() -> Config {
    Config {
        root_directory: None,
        incipit_host: Some("incipit.example.com".into()),
        addr: None,
        port: None,
        db_path: None,
        services: services().into_iter().map(|s| s.config).collect(),
    }
}

/// Start the services and incipit in the background.
///
/// Shorthand for [`start_services`] and [`start_incipit_background`].
pub async fn scaffold() -> eyre::Result<(Vec<Service>, JoinHandle<eyre::Result<()>>)> {
    let services = start_services().await?;
    let handle = start_incipit_background().await?;
    Ok((services, handle))
}

/// Starts incipit in the background.
pub async fn start_incipit_background() -> eyre::Result<JoinHandle<eyre::Result<()>>> {
    let config = example_config();
    let (http_listener, router) = crate::setup(config).await?;

    let handle = tokio::spawn(async {
        axum::serve(http_listener, router).await?;

        Ok(())
    });

    Ok(handle)
}
