use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use crate::config::Config;

/// The target to a mapping, which can be either a socket address, incipit itself or unknown
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Target {
    Socket(SocketAddr),
    Incipit,
    #[default]
    Unknown,
}

impl Target {
    /// Creates a new [`Target::Addr`] on 0.0.0.0 with the specified port
    pub fn port(port: u16) -> Self {
        let addr = ([0, 0, 0, 0], port).into();
        Target::Socket(addr)
    }
}

/// A trait for mapping hosts to addresses.
///
/// This is used to determine where to forward requests based on the host header.
///
/// Returns `None` when the host is not known.
pub trait HostMapping {
    fn route(&self, host: &str) -> Target;
}

impl HostMapping for Config {
    fn route(&self, host: &str) -> Target {
        if self.incipit_host.as_ref().map(|ih| ih == host).unwrap_or(false) {
            Target::Incipit
        } else if let Some(service) = self.services.iter().find(|&service| service.host == *host) {
            Target::Socket((self.addr(), service.port).into())
        } else {
            Target::Unknown
        }
    }
}

impl HostMapping for Arc<RwLock<Config>> {
    fn route(&self, host: &str) -> Target {
        // TODO: Handle this with eyre
        let config = self.read().expect("Lock should not be poisoned");
        config.route(host)
    }
}

impl<T> HostMapping for T
    where T: Fn(&str) -> Target {
    fn route(&self, host: &str) -> Target {
        self(host)
    }
}

// impl<K, V, S> HostMapping for std::collections::HashMap<K, V, S>
// where
//     K: for<'k> From<&'k String> + std::cmp::Eq + std::hash::Hash,
//     for<'v> &'v V: Into<SocketAddr>,
//     S: std::hash::BuildHasher,
// {
//     fn route(&self, Host(host): &Host) -> SocketAddr {
//         self.get(&K::from(host)).map(Into::into)
//     }
// }
