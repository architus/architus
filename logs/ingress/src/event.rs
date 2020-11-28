use crate::logging::{Event as LogEvent, EventOrigin, EventSource, EventType};
use logs_lib::id::HoarFrost;
use std::convert::Into;

/// Normalized log event to send to log ingestion
#[derive(Clone, PartialEq, Debug)]
pub struct NormalizedEvent {
    /// Id using snowflake format,
    /// using the *time that the event was received by wherever it's ingested*
    pub id: HoarFrost,
    /// Unix timestamp of the *time of the underlying event* (if available)
    pub timestamp: u64,
    /// The source data, including the original gateway/audit log entries
    pub source: Source,
    /// The origin of the event
    pub origin: EventOrigin,
    /// The type of action the event is
    pub event_type: EventType,
    /// An optional *human-readable* reason/message of the event
    pub reason: Option<String>,
    /// Related guild the event occurred in
    pub guild_id: Option<u64>,
    /// Id of the entity that caused the event to occur
    pub agent_id: Option<u64>,
    /// Id of the entity that the event is about/affects
    /// (can be any Id type)
    pub subject_id: Option<u64>,
    /// Id of the corresponding audit log entry this event corresponds to, if any
    /// (included for indexing purposes)
    pub audit_log_id: Option<u64>,
}

impl Into<LogEvent> for NormalizedEvent {
    fn into(self) -> LogEvent {
        // Convert the normalized event struct (specific to this service)
        // into the `LogEvent` struct, which is the gRPC-serializable struct
        LogEvent {
            id: self.id.0,
            timestamp: self.timestamp,
            source: Some(EventSource {
                gateway: self
                    .source
                    .gateway
                    .and_then(|json| serde_json::to_string(&json).ok())
                    .unwrap_or_else(|| String::from("")),
                audit_log: self
                    .source
                    .audit_log
                    .and_then(|json| serde_json::to_string(&json).ok())
                    .unwrap_or_else(|| String::from("")),
            }),
            origin: self.origin.into(),
            event_type: self.event_type.into(),
            reason: self.reason.unwrap_or_else(|| String::from("")),
            guild_id: self.guild_id.unwrap_or(0),
            agent_id: self.agent_id.unwrap_or(0),
            subject_id: self.subject_id.unwrap_or(0),
            audit_log_id: self.audit_log_id.unwrap_or(0),
        }
    }
}

/// Represents the in-memory version of the original JSON that this event represents,
/// included in the event struct for future data processing
#[derive(Clone, PartialEq, Debug)]
pub struct Source {
    pub gateway: Option<serde_json::Value>,
    pub audit_log: Option<serde_json::Value>,
}

impl Source {
    /// Calculates the `EventOrigin` variant for this source object,
    /// using the presence of the each sub-field to produce the result
    #[must_use]
    pub fn origin(&self) -> EventOrigin {
        match (self.gateway.as_ref(), self.audit_log.as_ref()) {
            (Some(_), Some(_)) => EventOrigin::Hybrid,
            (Some(_), None) => EventOrigin::Gateway,
            (None, Some(_)) => EventOrigin::AuditLog,
            (None, None) => EventOrigin::Internal,
        }
    }
}
