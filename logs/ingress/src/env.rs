use log::debug;
use std::env;

/// Configuration object loaded from the environment upon startup
pub struct Environment {
    pub token: String,
}

impl Environment {
    /// Loads the environment variables, called once at startup
    pub fn load() -> Environment {
        debug!("Loading configuration from the environment");
        Environment {
            token: load_string("DISCORD_TOKEN", "Discord bot token"),
        }
    }
}

/// Loads a string from the environment with the given key,
/// panicking if not found (and displaying formatted message using description)
fn load_string(key: &str, description: &str) -> String {
    match env::var(key) {
        Ok(value) => {
            debug!("Loaded {} from ${}", description, key);
            value
        }
        Err(_) => panic!(format!("requires {} in $${}", description, key)),
    }
}
