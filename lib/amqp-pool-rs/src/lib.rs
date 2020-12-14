use anyhow::{Context, Error};
use async_trait::async_trait;
use lapin::{Channel, Connection};

// Re-export error type
pub use deadpool::managed::PoolError;

/// Provides a channel pool around a single AMQP connection.
/// This struct can be cloned and transferred across thread boundaries
/// and uses reference counting for its internal state.
pub type Pool = deadpool::managed::Pool<Channel, Error>;

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
impl deadpool::managed::Manager<Channel, Error> for Manager {
    async fn create(&self) -> Result<Channel, Error> {
        self.connection
            .create_channel()
            .await
            .context("Could not create new channel in connection's channel pool")
    }
    async fn recycle(&self, _channel: &mut Channel) -> deadpool::managed::RecycleResult<Error> {
        Ok(())
    }
}
