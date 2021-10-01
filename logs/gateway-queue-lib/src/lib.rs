use architus_id::HoarFrost;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a half-serialized gateway event that exists on the durable RMQ queue
/// before it gets normalized into a standard log event
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GatewayEvent {
    /// Unique snowflake-formatted Id
    /// that will be eventually used to idempotently import a gateway-originating log event
    pub id: HoarFrost,
    /// Timestamp that the event was received,
    /// used to build the unique hoar frost ID
    pub ingress_timestamp: u64,
    /// Inner gateway payload (from the "d" key)
    /// See `https://discord.com/developers/docs/topics/gateway#payloads-gateway-payload-structure`
    pub inner: Value,
    /// Event type from the Discord gateway
    pub event_type: String,
    /// The guild id of the underlying event, if it exists
    pub guild_id: Option<u64>,
}
