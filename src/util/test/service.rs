use color_eyre::eyre;
use tokio::task::JoinSet;

use crate::config::ServiceConfig;

use super::Server;

pub enum Handler {
    Simple(fn(&str) -> Result<String, u16>),
}

impl Handler {
    pub fn inner(&self) -> fn(&str) -> Result<String, u16> {
        match self {
            Handler::Simple(handler) => *handler,
        }
    }
}

// A service mock.
pub struct Service<T = Server> {
    /// A function that returns `Ok(String)` as data and `Err(i32)` where the number is the HTTP
    /// status code.
    pub handler: Handler,

    /// The config the service would have
    pub config: ServiceConfig,

    pub server: T,
}

type StoppedService = Service<()>;

impl StoppedService {
    pub async fn start(self) -> eyre::Result<Service> {
        let server = Server::start(
            ([127, 0, 0, 1], self.config.port).into(),
            self.handler.inner(),
        )
        .await?;

        Ok(Service {
            handler: self.handler,
            config: self.config,
            server,
        })
    }
}

fn service1() -> StoppedService {
    Service {
        handler: Handler::Simple(|_| Ok("Hello world".into())),
        config: ServiceConfig {
            port: 1234,
            host: "service0.example.com".into(),
            name: "service0".into(),
            repo: None,
            command: None,
        },
        server: (),
    }
}

fn service2() -> StoppedService {
    Service {
        handler: Handler::Simple(|path| Ok(format!("Hello path: {path}"))),
        config: ServiceConfig {
            port: 9423,
            host: "service1.example.com".into(),
            name: "service1".into(),
            repo: None,
            command: None,
        },
        server: (),
    }
}

fn service3() -> StoppedService {
    Service {
        handler: Handler::Simple(|path| match path {
            "" => Ok("root".into()),
            "hello" => Ok("Hello".into()),
            _ => Err(404),
        }),
        config: ServiceConfig {
            port: 6969,
            host: "service2.example.com".into(),
            name: "service2".into(),
            repo: None,
            command: None,
        },
        server: (),
    }
}

fn service_websockets() -> StoppedService {
    Service {
        handler: Handler::Simple(|path| match path {
            "" => Ok("root".into()),
            "hello" => Ok("Hello".into()),
            _ => Err(404),
        }),
        config: ServiceConfig {
            port: 4455,
            host: "websockets.example.com".into(),
            name: "websocket_service".into(),
            repo: None,
            command: None,
        },
        server: (),
    }
}

pub fn services() -> Vec<StoppedService> {
    vec![service1(), service2(), service3(), service_websockets()]
}

pub async fn start_services() -> eyre::Result<Vec<Service>> {
    let services = services();
    let mut set = JoinSet::new();

    for service in services {
        set.spawn(service.start());
    }

    // Wait for all services to start
    set.join_all().await.into_iter().collect()
}
