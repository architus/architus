use backoff::backoff::Backoff as _;
use backoff::ExponentialBackoff;
use serde::Deserialize;
use std::time::Duration;

/// Controls an exponential backoff that can be loaded from a config file
#[derive(Default, Debug, Deserialize, Clone)]
pub struct Backoff {
    #[serde(with = "serde_humantime")]
    pub initial_interval: Duration,
    #[serde(with = "serde_humantime")]
    pub max_interval: Duration,
    #[serde(with = "serde_humantime")]
    pub duration: Duration,
    pub multiplier: f64,
}

impl Backoff {
    pub fn build(&self) -> ExponentialBackoff {
        self.into()
    }
}

impl<'a> Into<ExponentialBackoff> for &'a Backoff {
    fn into(self) -> ExponentialBackoff {
        let mut eb = ExponentialBackoff {
            current_interval: self.initial_interval,
            initial_interval: self.initial_interval,
            multiplier: self.multiplier,
            max_interval: self.max_interval,
            max_elapsed_time: Some(self.duration),
            ..ExponentialBackoff::default()
        };
        eb.reset();
        eb
    }
}
