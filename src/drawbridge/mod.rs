use axum::{
    extract::{Host, Request},
    response::{IntoResponse, Response},
};
use color_eyre::eyre;

use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use tokio::net::TcpStream;

use hyper_util::rt::TokioIo;

use crate::config::RuntimeConfig;

pub trait HostMapping {
    fn route(&self, host: &Host) -> Option<SocketAddr>;
}

pub struct HardCodedMapping {}

impl HostMapping for HardCodedMapping {
    fn route(&self, Host(host): &Host) -> Option<SocketAddr> {
        let port = match host.as_str() {
            "files.odilf.com" => 6942,
            "git.odilf.com" => 8264,
            "churri.odilf.com" => 2001,
            _ => return None,
        };

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        Some(addr)
    }
}

/// Route HTTP traffic to the appropriate port based on the host header and a mapping.
pub async fn route_traffic(
    host: Host,
    request: Request,
    mapping: impl HostMapping,
) -> eyre::Result<Response> {
    match mapping.route(&host) {
        Some(addr) => forward(request, addr).await,
        None => {
            tracing::warn!("Unknown host {}", host.0);
            eyre::bail!("Unknown host {}", host.0);
        }
    }
}

async fn forward(request: Request, addr: SocketAddr) -> eyre::Result<Response> {
    tracing::trace!("Forwarding to {addr}");
    let stream = TcpStream::connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    tokio::task::spawn(async move {
        if let Err(error) = conn.await {
            tracing::error!("Connection failed: {error:?}");
        }
    });

    Ok(sender.send_request(request).await?.into_response())
}

impl HostMapping for RuntimeConfig {
    fn route(&self, Host(host): &Host) -> Option<std::net::SocketAddr> {
        let service = self
            .services
            .iter()
            .find(|&service| service.host == *host)?;

        Some(service.addr())
    }
}

impl HostMapping for Arc<RwLock<RuntimeConfig>> {
    fn route(&self, host: &Host) -> Option<SocketAddr> {
        // TODO: Handle this with color_eyre
        let config = self.read().expect("Lock should not be poisoned");
        config.route(host)
    }
}
