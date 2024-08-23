//! Utilities to forward requests from one host to another.

use axum::{
    extract::{Host, Request},
    response::{IntoResponse, Response},
};
use color_eyre::eyre;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpStream;

mod mapping;

pub use mapping::HostMapping;

/// Route HTTP traffic to the appropriate port based on the host header and a mapping.
pub async fn route_traffic(
    host: Host,
    request: Request,
    mapping: impl HostMapping,
) -> eyre::Result<Response> {
    let Some(addr) = mapping.route(&host) else {
        tracing::warn!("Unknown host {}", host.0);
        eyre::bail!("Unknown host {}", host.0);
    };

    forward(request, addr).await
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
