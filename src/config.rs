use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    thread,
};

use color_eyre::eyre::{self, Context as _};
use figment::Figment;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

/// Global configuration of incipit. See [`service::Config`] for configuring services.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(try_from = "FileConfig")]
pub struct Config {
    /// Path to the root directory for other relative paths. If `None`, it will default to the
    /// directory where `uoh.toml` is located.
    ///
    /// It is where the git repos are cloned to, and the root from which relative paths are
    /// evaluated (such as `../path/to/file`).
    ///
    /// If not set explicitly in the config file, it will be the directory where the config is
    /// located.
    pub file_path: Option<PathBuf>,

    /// The services that incipit runs. See [`service::Config`].
    pub services: Vec<ServiceConfig>,

    /// Host on which to access the incipit dashboard. If not set, incipit's dashboard won't be
    /// accessible, but it will still start the services and reverse-proxy requests.
    pub incipit_host: Option<String>,

    /// Address to run incipit on.
    ///
    /// Defaults to `0.0.0.0` (to expose to network)
    pub addr: Option<IpAddr>,

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

        let figment = Figment::new()
            .merge(Toml::file("incipit.toml"))
            .merge(Env::prefixed("INCIPIT_"))
            .join(Json::file("incipit.json"));

        let mut config: Config = figment.extract()?;

        // Dance to get the source file path.
        let get_source = || {
            figment
                .metadata()
                .find_map(|meta| meta.source.clone())
                .and_then(|source| source.file_path().map(|path| path.to_path_buf()))
        };

        config.file_path = config.file_path.or_else(get_source);

        Ok(config)
    }

    pub fn from_file(path: &Path) -> eyre::Result<Self> {
        let content = std::fs::read_to_string(path).wrap_err("Failed to read config")?;
        let config: Config = toml::from_str(&content).wrap_err("Failed to parse config")?;
        Ok(config)
    }
}

/// Layout of the config that gets deserialized from. This is a separate struct to make
/// the file more convinient to write and the actual condig value more sensible at the time of
/// using it.
#[derive(serde::Deserialize)]
struct FileConfig {
    service: HashMap<String, ServiceConfig<Option<()>>>,
    incipit_host: Option<String>,
    addr: Option<IpAddr>,
    port: Option<u16>,
    db_path: Option<PathBuf>,
}

impl TryFrom<FileConfig> for Config {
    type Error = eyre::Error;
    fn try_from(file: FileConfig) -> eyre::Result<Self> {
        let config = Self {
            file_path: None,
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
        };

        Ok(config)
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

pub fn watch(config: Arc<RwLock<Config>>) -> eyre::Result<Option<RecommendedWatcher>> {
    let Some(config_path) = config.read().unwrap().file_path.clone() else {
        tracing::warn!("Not watching config");
        return Ok(None);
    };

    let (sender, receiver) = std::sync::mpsc::channel();

    let mut watcher = RecommendedWatcher::new(sender.clone(), Default::default())?;

    watcher.watch(
        config_path
            .parent()
            .expect("`config_path` is a file so it will always have a parent."),
        RecursiveMode::NonRecursive,
    )?;

    tracing::info!(?config_path, "Watching for changes");

    let _handle = thread::spawn(move || {
        for event in receiver.into_iter() {
            if !event?.paths.contains(&config_path) {
                continue;
            }

            let mut config = config.write().expect("Lock shouldn't be poisoned");
            *config = Config::new().wrap_err("Failed to reload config")?;

            tracing::info!("Reloaded config: {config:#?}");
        }

        eyre::Ok(())
    });

    Ok(Some(watcher))
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

    #[test]
    fn test_try_from_file_config() -> eyre::Result<()> {
        let file_config = FileConfig {
            service: HashMap::new(),
            incipit_host: Some("incipit.example.com".into()),
            addr: Some([127, 0, 0, 1].into()),
            port: Some(8080),
            db_path: Some(PathBuf::from("db")),
        };

        let config = Config::try_from(file_config)?;

        assert_eq!(config.file_path, None);
        assert_eq!(config.incipit_host, Some("incipit.example.com".into()));
        assert_eq!(config.addr, Some([127, 0, 0, 1].into()));
        assert_eq!(config.port, Some(8080));
        assert_eq!(config.db_path, Some(PathBuf::from("db")));

        Ok(())
    }
}
