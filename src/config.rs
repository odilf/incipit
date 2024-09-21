use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::{Arc, RwLock},
};

use color_eyre::eyre;
use figment::Figment;

/// Global configuration of incipit. See [`service::Config`] for configuring services.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(from = "FileConfig")]
pub struct Config {
    /// Path to the root directory for other relative paths. If `None`, it will default to the
    /// directory where `uoh.toml` is located.
    ///
    /// It is where the git repos are cloned to, and the root from which relative paths are
    /// evaluated (such as `../path/to/file`).
    ///
    /// If not set explicitly in the config file, it will be the directory where the config is
    /// located.
    pub root_directory: Option<PathBuf>,

    /// The services that incipit runs. See [`service::Config`].
    pub services: Vec<ServiceConfig>,

    /// Host on which to access the incipit dashboard. If not set, incipit's dashboard won't be
    /// accessible, but it will still start the services and reverse-proxy requests.
    pub incipit_host: Option<String>,

    /// Address to run incipit on.
    ///
    /// Defaults to `0.0.0.0` (to expose to network)
    pub addr: Option<[u8; 4]>,

    /// Port to run incipit on.
    ///
    /// Default to 80 for HTTP, consider setting it to 443 if you're using HTTPS.
    pub port: Option<u16>,

    /// Path where the database is stored.
    ///
    /// Defaults to `$root_path/incipit.db`
    pub db_path: Option<PathBuf>,
}

impl Config {
    pub fn new() -> eyre::Result<Self> {
        use figment::providers::{Env, Format as _, Json, Toml};

        let config = Figment::new()
            .merge(Toml::file("incipit.toml"))
            .merge(Env::prefixed("INCIPIT_"))
            .join(Json::file("incipit.json"))
            .extract()?;

        Ok(config)
    }
}

/// Layout of the config that gets deserialized from. This is a separate struct to make
/// the file more convinient to write and the actual condig value more sensible at the time of
/// using it.
#[derive(serde::Deserialize)]
struct FileConfig {
    root_directory: Option<PathBuf>,
    service: HashMap<String, ServiceConfig<Option<()>>>,
    incipit_host: Option<String>,
    addr: Option<[u8; 4]>,
    port: Option<u16>,
    db_path: Option<PathBuf>,
}

impl From<FileConfig> for Config {
    fn from(file: FileConfig) -> Self {
        Self {
            root_directory: file.root_directory,
            services: file
                .service
                .into_iter()
                .map(|(name, service)| ServiceConfig {
                    name,
                    port: service.port,
                    host: service.host,
                    repo: service.repo,
                    command: service.command,
                })
                .collect(),
            incipit_host: file.incipit_host,
            addr: file.addr,
            port: file.port,
            db_path: file.db_path,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ServiceConfig<T = String> {
    /// Name of the service.
    pub name: T,

    /// Port that the service listens on.
    pub port: u16,

    /// Host of the service. If `None`, it will default to <name>.<domain> (where the domain is
    /// obtained from the global config).
    pub host: String,

    /// Options related to the Git repository.
    pub repo: Option<RepoConfig>,

    /// Options related to commands for updating and running the service
    pub command: Option<CommandConfig>,
}

#[derive(Debug, Clone, serde::Deserialize, clap::Parser)]
pub struct RepoConfig {
    /// url to the git repository.
    ///
    /// It needs to be accessible by the user running `incipit`. That is,
    /// either public or with the appropriate permissions.
    pub url: String,

    /// Branch to pull from. If `None`, it will default to `main`.
    pub branch: Option<String>,
    // TODO:
    // pub auto_pull: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct CommandConfig {
    /// Command to run the service
    pub run: String,
}

impl Config {
    pub fn addr(&self) -> IpAddr {
        const DEFAULT: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0));
        self.addr.map(Into::into).unwrap_or(DEFAULT)
    }

    pub fn socket(&self) -> SocketAddr {
        SocketAddr::new(self.addr(), self.port.unwrap_or(80))
    }
}

pub fn watch_config(config: Config) -> eyre::Result<Arc<RwLock<Config>>> {
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

#[derive(thiserror::Error, Debug)]
pub enum GetConfigError {
    #[error("Config not found")]
    ConfigNotFound,

    #[error("Failed to read config")]
    ReadConfigError(#[from] std::io::Error),

    #[error("Failed to parse config")]
    ConfigParseError(#[from] toml::de::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_find_config() {
        let path = Config::new();
        assert!(path.is_ok());
    }
}
