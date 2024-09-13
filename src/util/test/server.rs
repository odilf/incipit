use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use color_eyre::eyre;
use futures::never::Never;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

type History = Arc<Mutex<Vec<(Request<hyper::body::Incoming>, Result<String, u16>)>>>;

#[derive(Debug)]
pub struct Server {
    /// A history of all requests that have been made to the server
    pub history: History,

    /// The handle to the tokio task that is running the server
    handle: JoinHandle<eyre::Result<Infallible>>,
}

impl Server {
    pub async fn start(
        addr: SocketAddr,
        handler: fn(&str) -> Result<String, u16>,
    ) -> eyre::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let request_history = Arc::new(Mutex::new(Vec::new()));

        let handle = tokio::spawn(Server::serve(
            listener,
            handler,
            Arc::clone(&request_history),
        ));

        Ok(Self {
            history: request_history,
            handle,
        })
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

                                let output = match response {
                                    Ok(message) => {
                                        Ok(Response::new(Full::new(Bytes::from(message))))
                                    }
                                    Err(status) => Response::builder()
                                        .status(status)
                                        .body(Full::new(Bytes::new())),
                                };

                                output
                            }
                        }),
                    )
                    .await?;

                Ok::<_, eyre::Report>(())
            });
        }
    }
}
