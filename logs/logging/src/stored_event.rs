use crate::logging::Event;
use serde::Serialize;
use std::convert::TryFrom;

/// Represents the JSON-serializable version of the stored Elasticsearch log event
/// See lib/ipc/proto/logging.proto for the original definition of this struct
#[derive(Debug, Serialize)]
pub struct StoredEvent {
    pub id: u64,
    pub timestamp: u64,
    pub source: StoredSource,
    pub origin: i32,
    pub event_type: i32,
    pub guild_id: u64,
    pub agent_id: u64,
    pub subject_id: u64,
    pub audit_log_id: u64,
    pub reason: std::string::String,
}

#[derive(Debug, Serialize)]
pub struct StoredSource {
    pub gateway: Option<serde_json::Value>,
    pub audit_log: Option<serde_json::Value>,
}

// Provide an implementation to convert the Event to a StoredEvent.
// Involves deserializing the inner JSON from the string so it can be embedded.
// Note that the protobuf definitions do not use the generic Struct message
// (from the Google well-known types) because it only supports f64 numbers,
// which may cause problems in the future.
// Instead, the internal JSON is sent as a string
impl TryFrom<Event> for StoredEvent {
    type Error = serde_json::Error;

    fn try_from(value: Event) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            timestamp: value.id,
            source: StoredSource {
                gateway: value
                    .source
                    .as_ref()
                    .map(|source| &source.gateway)
                    .map(|gateway| serde_json::from_str(gateway))
                    .transpose()?,
                audit_log: value
                    .source
                    .as_ref()
                    .map(|source| &source.audit_log)
                    .map(|audit_log| serde_json::from_str(audit_log))
                    .transpose()?,
            },
            origin: value.origin,
            event_type: value.event_type,
            guild_id: value.guild_id,
            agent_id: value.agent_id,
            subject_id: value.subject_id,
            audit_log_id: value.audit_log_id,
            reason: value.reason,
        })
    }
}
