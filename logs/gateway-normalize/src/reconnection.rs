//! Exposes a small utility wrapper for `backoff`
//! that resets the backoff after a certain period of time

use backoff::backoff::Backoff;
use backoff::ExponentialBackoff;
use std::time::{Duration, Instant};

/// Represents a reconnection backoff utility wrapper
/// for a long-running task that should use an exponential backoff
/// when multiple failures occur in short succession,
/// but reset the backoff if the task has been running for a long time
/// (greater than the threshold)
#[derive(Debug)]
pub struct State {
    current: ExponentialBackoff,
    last_start: Option<Instant>,
    threshold: Duration,
}

impl State {
    pub const fn new(source: ExponentialBackoff, threshold: Duration) -> Self {
        Self {
            current: source,
            last_start: None,
            threshold,
        }
    }

    pub async fn wait(&mut self) -> anyhow::Result<()> {
        if let Some(last_start) = self.last_start {
            let running_time = Instant::now().duration_since(last_start);
            if running_time > self.threshold {
                // The running time was longer than the threshold to use the old backoff,
                // so reset it with the source backoff (from the config)
                self.current.reset();
            }

            match self.current.next_backoff() {
                None => return Err(anyhow::anyhow!("reconnection backoff elapsed")),
                Some(backoff) => tokio::time::sleep(backoff).await,
            }
        }

        // Mark the start of the next iteration
        self.last_start = Some(Instant::now());
        Ok(())
    }
}
