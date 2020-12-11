use crate::UptimeEvent;
use anyhow::Result;
use futures::Stream;
use static_assertions::assert_impl_all;

/// Represents a shared handler that continuously polls the feature service
/// and sits between the connection tracker and the uptime service
/// to maintain a pool of the actively listened guilds
/// and take the intersection of those guilds with the ones that have indexing enabled
#[derive(Clone, Debug)]
pub struct ActiveGuilds {}

assert_impl_all!(ActiveGuilds: Sync, Send);

impl ActiveGuilds {
    /// Creates a new shared handler and wraps the connection to the feature gate service
    pub fn new(_feature_gate_client: ()) -> Self {
        // TODO implement and change feature gate client name
        Self {}
    }

    /// Runs a task that continuously polls the feature gate to maintain an active list of guilds
    /// that have log indexing enabled
    pub async fn go_poll(&self) -> Result<()> {
        // TODO implement
        Ok(())
    }

    /// Filters uptime events to ensure that they only contain active guilds
    /// that have events that are actually forwarded
    pub fn pipe_uptime_events(
        &self,
        in_stream: impl Stream<Item = UptimeEvent>,
    ) -> impl Stream<Item = UptimeEvent> {
        // TODO implement
        in_stream
    }

    /// Determines whether the given `guild_id` should have events forwarded to the queue
    pub fn is_active(&self, _guild_id: u64) -> bool {
        false
    }
}
