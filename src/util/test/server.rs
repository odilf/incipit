use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use color_eyre::eyre;
use futures::never::Never;
use futures::{SinkExt as _, StreamExt as _};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_tungstenite::tungstenite::Message;
use hyper_tungstenite::HyperWebsocket;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::sync::broadcast::{self, Receiver, Sender};
use tokio::task::JoinHandle;

type History<T = (Request<Incoming>, Result<String, u16>)> = Arc<Mutex<Vec<T>>>;

/// Server that handles HTTP connections.
///
/// See also [`WebSocketServer`]
#[derive(Debug)]
pub struct Server {
    /// A history of all requests that have been made to the server
    pub history: History,
    // /// The handle to the tokio task that is running the server
    // handle: JoinHandle<eyre::Result<Infallible>>,
}

impl Server {
    pub async fn start(
        addr: SocketAddr,
        handler: fn(&str) -> Result<String, u16>,
    ) -> eyre::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let history = Arc::new(Mutex::new(Vec::new()));

        let _handle = tokio::spawn(Server::serve(listener, handler, Arc::clone(&history)));

        Ok(Self { history })
    }

    pub async fn serve(
        listener: TcpListener,
        handler: fn(&str) -> Result<String, u16>,
        request_history: History,
    ) -> eyre::Result<Never> {
        // We start a loop to continuously accept incoming connections
        loop {
            let request_history = Arc::clone(&request_history); // Hella ugly

            let (stream, _) = listener.accept().await?;

            // Use an adapter to access something implementing `tokio::io` traits as if they implement
            // `hyper::rt` IO traits.
            let io = TokioIo::new(stream);

            // Spawn a tokio task to serve multiple connections concurrently
            tokio::task::spawn(async move {
                // let handler = adapt_handler(handler);
                // Finally, we bind the incoming connection to our `hello` service
                http1::Builder::new()
                    // `service_fn` converts our function in a `Service`
                    .serve_connection(
                        io,
                        service_fn(|request| {
                            let value = Arc::clone(&request_history); // Hella ugly too.
                            async move {
                                let path = request.uri().path();
                                let response = handler(path);

                                {
                                    let mut request_history = value.lock().unwrap();
                                    request_history.push((request, response.clone()));
                                }

                                match response {
                                    Ok(message) => {
                                        Ok(Response::new(Full::new(Bytes::from(message))))
                                    }
                                    Err(status) => Response::builder()
                                        .status(status)
                                        .body(Full::new(Bytes::new())),
                                }
                            }
                        }),
                    )
                    .await?;

                Ok::<_, eyre::Report>(())
            });
        }
    }
}

/// Server that handles WebSocket connections.
pub struct WebSocketServer {
    pub history: History<String>,
    sender: Sender<String>,
    /// The handle to the tokio task that is running the server
    _handle: JoinHandle<eyre::Result<Infallible>>,
}

impl WebSocketServer {
    pub async fn start(addr: SocketAddr) -> eyre::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let history = Arc::new(Mutex::new(Vec::new()));
        let (sender, _receiver) = broadcast::channel(10);

        let _handle = tokio::spawn(Self::serve(listener, Arc::clone(&history), sender.clone()));

        Ok(Self {
            history,
            sender,
            _handle,
        })
    }

    pub fn send(&mut self, message: String) -> eyre::Result<()> {
        self.sender.send(message)?;

        Ok(())
    }

    pub async fn serve(
        listener: TcpListener,
        history: History<String>,
        sender: Sender<String>,
    ) -> eyre::Result<Infallible> {
        // We start a loop to continuously accept incoming connections
        loop {
            let history = Arc::clone(&history); // Hella ugly
            let sender = sender.clone();

            let (stream, _) = listener.accept().await?;

            // Use an adapter to access something implementing `tokio::io` traits as if they implement
            // `hyper::rt` IO traits.
            let io = TokioIo::new(stream);

            // Spawn a tokio task to serve multiple connections concurrently
            tokio::task::spawn(async move {
                // let handler = adapt_handler(handler);
                // Finally, we bind the incoming connection to our `hello` service
                http1::Builder::new()
                    // `service_fn` converts our function in a `Service`
                    .serve_connection(
                        io,
                        service_fn(|mut request| {
                            let history = Arc::clone(&history);
                            let sender = sender.clone();
                            async move {
                                // Check if the request is a websocket upgrade request.
                                if !hyper_tungstenite::is_upgrade_request(&request) {
                                    return Ok(Response::new(Full::<Bytes>::from("Hello HTTP!")));
                                }

                                let (response, websocket) =
                                    hyper_tungstenite::upgrade(&mut request, None)?;

                                // Spawn a task to handle the websocket connection.
                                tokio::spawn(async move {
                                    if let Err(e) =
                                        serve_websocket(websocket, history, sender.subscribe())
                                            .await
                                    {
                                        eprintln!("Error in websocket connection: {e}");
                                    }
                                });

                                // Return the response so the spawned future can continue.
                                Ok::<_, eyre::Report>(response)
                            }
                        }),
                    )
                    .with_upgrades()
                    .await?;

                Ok::<_, eyre::Report>(())
            });
        }
    }
}

/// Handle a websocket connection.
async fn serve_websocket(
    websocket: HyperWebsocket,
    history: History<String>,
    mut receiver: Receiver<String>,
) -> eyre::Result<()> {
    let mut websocket = websocket.await?;

    loop {
        tokio::select! {
            Some(message) = websocket.next() => {
                match message? {
                    Message::Text(msg) => {
                        history.lock().unwrap().push(msg);
                    }

                    // TODO: Maybe change
                    Message::Close(msg) => {
                        if let Some(msg) = &msg {
                            println!(
                                "Received close message with code {} and message: {}",
                                msg.code, msg.reason
                            );
                        } else {
                            println!("Received close message");
                        }
                    }

                    // TODO: Handle other messages
                    _ => panic!("Unsupported message type"),
                }
            },

            Ok(message) = receiver.recv() => {
                websocket.send(Message::Text(message)).await?;
            },

            else => {
                break
            },
        }
    }

    Ok(())
}
