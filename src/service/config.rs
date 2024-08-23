use std::net::SocketAddr;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FileConfig {
    /// Name of the service.
    pub name: String,

    /// Port that the service listens on.
    pub port: u16,

    /// Host of the service. If `None`, it will default to <name>.<domain> (where the domain is
    /// obtained from the global config).
    pub host: Option<String>,

    pub repo: Option<RepoConfig>,

    pub run_command: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
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

impl FileConfig {
    pub fn into_runtime_config(self, domain: &str) -> RuntimeConfig {
        let host = self
            .host
            .unwrap_or_else(|| format!("{}.{}", self.name, domain));

        RuntimeConfig {
            name: self.name,
            port: self.port,
            host,
            // TODO: We should auto set the `main` branch if it's not given
            repo: self.repo,
            run_command: self.run_command,
        }
    }
}

// TODO: Maybe "runtime config" as a concept doesn't make sense? Maybe it should just be "state".
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub name: String,
    pub port: u16,
    pub host: String,
    pub repo: Option<RepoConfig>,
    pub run_command: String,
}

impl RuntimeConfig {
    pub fn addr(&self) -> SocketAddr {
        SocketAddr::from(([0, 0, 0, 0], self.port))
    }
}
