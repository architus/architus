//! Includes a two-stage pipeline where Gateway lifecycle events
//! are sent through a shared mpsc channel to a `connection::Tracker`,
//! which stores a per-guild connection status as well as an overall connection
//! to the Rabbit MQ queue and Discord Gateway.
//! This `connection::Tracker` re-emits `Event` structs that represent rising and falling edges
//! as guilds come online/offline, in addition to heartbeat messages to maintain an online guild.
//! These events are routed through the `active_guilds` module, which contains logic
//! for periodically polling the feature-gate service and noting the guilds with indexing enabled.
//! It filters the raw `Event`s from `connection::Tracker` based on the guilds with indexing enabled,
//! only allowing events for those that have it to be forwarded
//! (in addition to generating rising/falling edge events
//! when it detects an online guild has enabled/disabled indexing).

pub mod active_guilds;
pub mod connection;
mod debounced_pool;

use crate::rpc::uptime::{GatewaySubmitRequest, GatewaySubmitType};

/// Raw update messages that can come from the rest of the service,
/// and are used to update the current connections state,
/// sending uptime tracking events as needed.
#[derive(Clone, Debug, PartialEq)]
pub enum UpdateMessage {
    GuildOnline(u64),
    GuildOffline(u64),
    QueueOnline,
    QueueOffline,
    GatewayOnline,
    GatewayOffline,
    GatewayHeartbeat,
}

/// Represents a bulk uptime event that is eventually dispatched to the uptime service
/// in addition to the timestamp that the event happened at.
/// A stream of these events is the **final product** of this submodule,
/// which generates them based on service/guild connection via updates
/// before filtering them based on which guilds actually have indexing enabled.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Online { guilds: Vec<u64>, timestamp: u64 },
    Offline { guilds: Vec<u64>, timestamp: u64 },
    Heartbeat { guilds: Vec<u64>, timestamp: u64 },
}

// Provide the ability to convert an uptime event into a gateway submit request
impl Event {
    #[allow(clippy::missing_const_for_fn)]
    pub fn into_request(self, session: u64) -> GatewaySubmitRequest {
        match self {
            Self::Online { guilds, timestamp } => GatewaySubmitRequest {
                r#type: GatewaySubmitType::Online as i32,
                guilds,
                timestamp,
                session,
            },
            Self::Offline { guilds, timestamp } => GatewaySubmitRequest {
                r#type: GatewaySubmitType::Offline as i32,
                guilds,
                timestamp,
                session,
            },
            Self::Heartbeat { guilds, timestamp } => GatewaySubmitRequest {
                r#type: GatewaySubmitType::Heartbeat as i32,
                guilds,
                timestamp,
                session,
            },
        }
    }
}
