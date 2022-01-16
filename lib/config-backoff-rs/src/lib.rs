//! Exposes a struct that can be used in a config
//! to define backoff::ExponentialBackoff's behavior.

use backoff::backoff::Backoff as _;
use backoff::ExponentialBackoff;
use serde::Deserialize;
use std::time::Duration;

/// Controls an exponential backoff that can be loaded from a config file
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Backoff {
    #[serde(with = "humantime_serde")]
    pub initial_interval: Duration,
    #[serde(with = "humantime_serde")]
    pub max_interval: Duration,
    #[serde(with = "humantime_serde")]
    pub duration: Duration,
    pub multiplier: f64,
}

impl Backoff {
    pub fn build(&self) -> ExponentialBackoff {
        self.into()
    }
}

impl<'a> From<&'a Backoff> for ExponentialBackoff {
    fn from(other: &'a Backoff) -> ExponentialBackoff {
        let mut eb = ExponentialBackoff {
            current_interval: other.initial_interval,
            initial_interval: other.initial_interval,
            multiplier: other.multiplier,
            max_interval: other.max_interval,
            max_elapsed_time: Some(other.duration),
            ..ExponentialBackoff::default()
        };
        eb.reset();
        eb
    }
}
