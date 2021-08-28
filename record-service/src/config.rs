use anyhow::Context;

/// Configuration struct to load on startup
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Configuration {
    pub services: Services,
    pub secrets: Secrets,
}

/// Where to find services that record service uses
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Services {
    pub database: String,
}

/// Secret parameters for the record service
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Secrets {
    pub discord_token: String,
    pub db_username: String,
    pub db_password: String,
}

impl Configuration {
    pub fn try_load(path: impl AsRef<str>) -> anyhow::Result<Self> {
        let path = path.as_ref();

        let mut settings = config::Config::default();
        settings
            .merge(config::File::with_name(path))
            .context(format!("Could not read in config file from {}", path))?
            .merge(config::Environment::with_prefix("RECORD_SERVICE_CONFIG").separator("__"))
            .context("could not merge in values from the environment")?;
        let config = settings
            .try_into()
            .context("loading the Configuration struct from the merged config failed")?;
        Ok(config)
    }
}
