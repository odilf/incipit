use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{Arc, RwLock},
};

use color_eyre::eyre;

use crate::service;

const CONFIG_PATH_ENV: &str = "INCIPIT_CONFIG_PATH";

/// Global configuration of `incipit`
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FileConfig {
    pub domain: String,

    /// Path to the root directory for other relative paths. If `None`, it will default to the
    /// directory where `uoh.toml` is located.
    pub root_directory: Option<PathBuf>,

    pub services: Vec<service::config::FileConfig>,

    pub addr: Option<String>,
    pub port: Option<u16>,
}

impl FileConfig {
    fn into_runtime_config(self, root_directory: PathBuf) -> RuntimeConfig {
        let services = self
            .services
            .into_iter()
            .map(|s| s.into_runtime_config(&self.domain))
            .collect();

        RuntimeConfig {
            domain: self.domain,
            root_directory: self.root_directory.unwrap_or(root_directory),
            services,

            addr: self
                .addr
                .and_then(|addr| addr.parse().ok())
                .unwrap_or(IpAddr::from([0, 0, 0, 0])),
            port: self.port.unwrap_or(80),
        }
    }
}

/// Runtime configuration of `incipit`. This is a modified version of `GlobalUserConfig` that
/// is more populated with runtime and default values.
// TODO: Figure out a nicer way to do this with less repetition.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub domain: String,
    pub addr: IpAddr,
    pub port: u16,
    pub root_directory: PathBuf,
    pub services: Vec<service::config::RuntimeConfig>,
}

impl RuntimeConfig {
    pub fn socket(&self) -> SocketAddr {
        SocketAddr::new(self.addr, self.port)
    }
}

pub fn watch_config(config: RuntimeConfig) -> eyre::Result<Arc<RwLock<RuntimeConfig>>> {
    let config = Arc::new(RwLock::new(config));

    // TODO: Watch files
    // let mut watcher = notify::recommended_watcher(|res| match res {
    //     Ok(event) => tracing::info!("event: {:?}", event),
    //     Err(e) => tracing::warn!("watch error: {:?}", e),
    // })?;
    //
    // let config_path = config.read().unwrap().root_directory.join("uoh.toml");
    // tracing::info!("Watching for changes in {config_path:?}",);
    //
    // watcher.watch(&config_path, RecursiveMode::Recursive)?;

    Ok(config)
}

/// Finds and reads a config
///
/// It searches from an env variable or if some parent directory contains a file named `uoh.toml`.
pub fn read_config() -> Result<RuntimeConfig, GetConfigError> {
    let Some(directory) = find_config_dir() else {
        return Err(GetConfigError::ConfigNotFound);
    };

    let file = std::fs::read_to_string(directory.join("uoh.toml"))?;

    let user_config: FileConfig = toml::from_str(&file)?;

    Ok(user_config.into_runtime_config(directory))
}

#[derive(thiserror::Error, Debug)]
pub enum GetConfigError {
    #[error("Config not found")]
    ConfigNotFound,

    #[error("Failed to read config")]
    ReadConfigError(#[from] std::io::Error),

    #[error("Failed to parse config")]
    ConfigParseError(#[from] toml::de::Error),
}

fn find_config_dir() -> Option<PathBuf> {
    tracing::trace!("Searching for config");
    if let Some(path) = std::env::var_os(CONFIG_PATH_ENV) {
        return Some(PathBuf::from(path).to_path_buf());
    }

    let mut directory = std::env::current_dir().ok()?;

    while !directory.join("uoh.toml").exists() {
        directory = directory.parent()?.to_path_buf();
    }

    Some(directory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_find_config() {
        let path = find_config_dir();
        assert!(path.is_some());
    }
}
