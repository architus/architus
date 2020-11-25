use anyhow::{Context, Result};
use log::debug;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;

/// Configuration object loaded upon startup
#[derive(Deserialize)]
pub struct Configuration {
    /// Collection of secret values used to connect to services
    pub secrets: Secrets,
    /// Maximum number of executing futures for gateway event normalization processing
    pub normalized_stream_concurrency: usize,
    /// Maximum number of executing futures for normalized event importing
    pub import_stream_concurrency: usize,
}

/// Collection of secret values used to connect to services
#[derive(Deserialize)]
pub struct Secrets {
    /// Discord bot token used to authenticate with the Gateway API
    pub discord_token: String,
}

impl Configuration {
    /// Attempts to load the config from the file, called once at startup
    pub fn try_load(path: impl AsRef<str>) -> Result<Configuration> {
        let path = path.as_ref();
        debug!("Loading configuration from {}", path);
        let mut contents = String::new();
        File::open(path)
            .context(format!("Could open config file from {}", path))?
            .read_to_string(&mut contents)
            .context(format!("Could not read in config file from {}", path))?;
        let config =
            toml::from_str::<Configuration>(&contents).context("Parsing config file failed")?;
        Ok(config)
    }
}
