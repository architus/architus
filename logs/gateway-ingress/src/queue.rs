//! Handles creating a bounded queue that logs
//! when its number of queued items approaches its capacity.
//! This is useful to constantly consume events from the gateway,
//! ensuring they are processed and filtered as soon as possible.
//! Dropping events over capacity is preferable to directly exposing the back-pressure
//! from a bounded queue because we should loudly complain when dropping events
//! and continue to try to consume recent events
//! instead of slowly letting a delay build up during event ingress.
//! This is also needed in order for the `ingress_timestamp` field to be resolved accurately;
//! which is used in downstream processors to assign timestamps to events,
//! so introducing back-pressure would start adding artificial delay
//! to all events' ingress timestamps.

use futures::{Stream, StreamExt};
use slog::Logger;
use std::fmt::Debug;
use std::time::Duration;
use tokio::sync::mpsc::{self, Sender};
use tokio_stream::wrappers::ReceiverStream;

pub struct BoundedQueue<T> {
    logger: Logger,
    config: BoundedQueueConfig,
    item_tx: Sender<T>,
}

pub struct BoundedQueueConfig {
    pub identifier: String,
    pub max_size: usize,
    pub warning_threshold: usize,
    pub watch_size_interval: Duration,
}

impl<T> BoundedQueue<T>
where
    T: Debug,
{
    pub fn new(config: BoundedQueueConfig, logger: &Logger) -> (Self, impl Stream<Item = T>) {
        let (item_tx, item_rx) = mpsc::channel::<T>(config.max_size);
        let new_self = Self {
            logger: logger.new(slog::o!(
                "queue_identifier" => config.identifier.clone(),
                "warn_threshold" => config.warning_threshold,
                "max_size" => config.max_size,
            )),
            config,
            item_tx,
        };

        (new_self, ReceiverStream::new(item_rx))
    }

    /// Pipes events into this bounded queue,
    /// dropping them if the queue is already full.
    pub async fn pipe_in(&self, in_stream: impl Stream<Item = T>) {
        in_stream
            .for_each(|item| async {
                if let Err(send_err) = self.item_tx.try_send(item) {
                    slog::warn!(
                        self.logger,
                        "sending item into bounded queue failed; dropping";
                        "error" => ?send_err,
                    );
                }
            })
            .await;
    }

    /// Asynchronously watches the queue length to ensure
    /// that it doesn't exceed a warning threshold.
    /// This is useful for reporting before the queue starts dropping events.
    pub async fn watch_size(&self) {
        let mut interval = tokio::time::interval(self.config.watch_size_interval);
        loop {
            interval.tick().await;
            let current_length = self.config.max_size - self.item_tx.capacity();
            if current_length > self.config.warning_threshold {
                slog::warn!(
                    self.logger,
                    "current queue length exceeds warning threshold";
                    "current_queue_length" => current_length,
                );
            }
        }
    }
}
