use axum::{
    extract::Request,
    http::request::Parts,
    response::{IntoResponse as _, Response},
};
use color_eyre::eyre;
use futures::{SinkExt as _, StreamExt as _};
use tokio_tungstenite::connect_async;
use tungstenite::client::IntoClientRequest;

use super::mapping::Target;

pub async fn handle(request: &mut Request, parts: Parts, target: Target) -> Option<Response> {
    if !hyper_tungstenite::is_upgrade_request(request) {
        return None;
    }

    tracing::debug!("Upgrading to WebSocket");
    let (response, websocket) = hyper_tungstenite::upgrade(request, None).unwrap();

    // Spawn a task to handle the websocket connection.
    tokio::spawn(async move {
        if let Err(e) = serve_websocket(websocket, parts, target).await {
            eprintln!("Error in websocket connection: {e}");
        }
    });

    Some(response.into_response())
}

/// Handle a websocket connection.
async fn serve_websocket(
    websocket: hyper_tungstenite::HyperWebsocket,
    request_parts: Parts,
    target: Target,
) -> eyre::Result<()> {
    let mut websocket_client = websocket.await?;

    let url = match target {
        Target::Socket(addr) => format!(
            "ws://{addr}/{path}",
            path = request_parts
                .uri
                .path_and_query()
                .map(|v| v.as_str().trim_start_matches('/'))
                .unwrap_or("")
        ),
        _ => return Err(eyre::eyre!("Invalid target for websocket")),
    };

    // Add the headers from the original request to the target request.
    let mut target_request = url.into_client_request()?;
    *target_request.headers_mut() = request_parts.headers.clone();

    let (mut websocket_target, _) = connect_async(target_request).await?;

    loop {
        tokio::select! {
            Some(client_message) = websocket_client.next() => {
                websocket_target.send(client_message?).await?;
            },
            Some(target_message) = websocket_target.next() => {
                websocket_client.send(target_message?).await?;
            },
            else => {
                break
            },
        }
    }

    Ok(())
}
