//! This library is needed because the deadpool-lapin crate
//! manages Connections instead of Channels, and this is [an anti-pattern].
//! More details are available in [this related issue]
//!
//! [an anti-pattern]: https://www.cloudamqp.com/blog/part4-rabbitmq-13-common-errors.html
//! [this related issue]: https://github.com/bikeshedder/deadpool/issues/47

use anyhow::{Context, Error};
use async_trait::async_trait;
use lapin::{Channel, Connection};

// Re-export error type
pub use deadpool::managed::PoolError;

/// Provides a channel pool around a single AMQP connection.
/// This struct can be cloned and transferred across thread boundaries
/// and uses reference counting for its internal state.
pub type Pool = deadpool::managed::Pool<Manager>;

pub struct Manager {
    connection: Connection,
}

impl Manager {
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(connection: Connection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl deadpool::managed::Manager for Manager {
    type Type = Channel;
    type Error = Error;
    async fn create(&self) -> Result<Channel, Error> {
        self.connection
            .create_channel()
            .await
            .context("could not create new channel in connection's channel pool")
    }
    async fn recycle(&self, _channel: &mut Channel) -> deadpool::managed::RecycleResult<Error> {
        Ok(())
    }
}
