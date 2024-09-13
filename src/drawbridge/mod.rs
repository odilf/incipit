//! Utilities to forward requests from one host to another.

mod mapping;

#[cfg(test)]
mod test;

use axum::{
    extract::{Host, Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use color_eyre::eyre;
use hyper::StatusCode;
use hyper_util::rt::TokioIo;
use mapping::Target;
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tokio::net::TcpStream;

pub use mapping::HostMapping;

use crate::Config;

async fn forward_to_addr(request: Request, addr: SocketAddr) -> eyre::Result<Response> {
    tracing::trace!("Forwarding request {request:?} to {addr}");

    let stream = TcpStream::connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    tokio::task::spawn(async move {
        if let Err(error) = conn.await {
            tracing::error!("Connection failed: {error}");
        }
    });

    Ok(sender.send_request(request).await?.into_response())
}

async fn forward(request: Request, target: Target, next: Next) -> eyre::Result<Response> {
    let response = match target {
        Target::Socket(addr) => forward_to_addr(request, addr).await?,
        Target::Incipit => next.run(request).await,
        Target::Unknown => {
            (StatusCode::NOT_FOUND, "404 - Host not known by incipit").into_response()
        }
    };

    Ok(response)
}

pub async fn middleware(
    State(config): State<Arc<RwLock<Config>>>,
    Host(host): Host,
    request: Request,
    next: Next,
) -> Response {
    let target = config.read().unwrap().route(&host);

    match forward(request, target, next).await {
        Ok(response) => response,
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, format!("500 - {err}")).into_response(),
    }
}
