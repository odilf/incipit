use axum::extract::Host;
use std::net::SocketAddr;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::config::RuntimeConfig;

/// A trait for mapping hosts to addresses.
///
/// This is used to determine where to forward requests based on the host header.
pub trait HostMapping {
    fn route(&self, host: &Host) -> Option<SocketAddr>;
}

impl HostMapping for RuntimeConfig {
    fn route(&self, Host(host): &Host) -> Option<SocketAddr> {
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

impl<K, V, S> HostMapping for HashMap<K, V, S>
where
    K: for<'k> From<&'k Host> + std::cmp::Eq + std::hash::Hash,
    for<'v> &'v V: Into<SocketAddr>,
    S: std::hash::BuildHasher,
{
    fn route(&self, host: &Host) -> Option<SocketAddr> {
        self.get(&K::from(host)).map(Into::into)
    }
}
