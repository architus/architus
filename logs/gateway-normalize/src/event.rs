use crate::rpc::import::{
    Event as LogEvent, EventOrigin, EventSource, EventType, SubmitIdempotentRequest,
};
use architus_id::HoarFrost;
use std::convert::Into;
use tonic::{IntoRequest, Request};

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
    /// Related guild the event occurred in
    pub guild_id: u64,
    /// Id of the entity that caused the event to occur
    pub agent_id: Option<u64>,
    /// Id of the entity that the event is about/affects
    /// (can be any Id type)
    pub subject_id: Option<u64>,
    /// Id of the corresponding audit log entry this event corresponds to, if any
    /// (included for indexing purposes)
    pub audit_log_id: Option<u64>,
    /// Channel that the event occurred in
    pub channel_id: Option<u64>,
    /// An optional *human-readable* reason/message of the event
    pub reason: Option<String>,
}

impl IntoRequest<SubmitIdempotentRequest> for NormalizedEvent {
    fn into_request(self) -> Request<SubmitIdempotentRequest> {
        Request::new(SubmitIdempotentRequest {
            event: Some(self.into()),
        })
    }
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
                internal: self
                    .source
                    .internal
                    .and_then(|json| serde_json::to_string(&json).ok())
                    .unwrap_or_else(|| String::from("")),
            }),
            origin: self.origin.into(),
            r#type: self.event_type.into(),
            guild_id: self.guild_id,
            agent_id: self.agent_id.unwrap_or(0),
            // TODO implement
            agent_type: 0,
            agent_metadata: None,
            subject_id: self.subject_id.unwrap_or(0),
            subject_type: 0,
            subject_metadata: 0,
            auxiliary_id: 0,
            auxiliary_type: 0,
            auxiliary_metadata: 0,
            content: String::from(""),
            audit_log_id: self.audit_log_id.unwrap_or(0),
            channel_id: self.channel_id.unwrap_or(0),
            reason: self.reason.unwrap_or_else(|| String::from("")),
            channel_name: String::from(""),
            content_metadata: None,
        }
    }
}

/// Represents the in-memory version of the original JSON that this event represents,
/// included in the event struct for future data processing
#[derive(Clone, PartialEq, Debug)]
pub struct Source {
    pub gateway: Option<serde_json::Value>,
    pub audit_log: Option<serde_json::Value>,
    pub internal: Option<serde_json::Value>,
}

impl Source {
    /// Calculates the `EventOrigin` variant for this source object,
    /// using the presence of the each sub-field to produce the result
    #[must_use]
    pub const fn origin(&self) -> EventOrigin {
        match (&self.gateway, &self.audit_log) {
            (Some(_), Some(_)) => EventOrigin::Hybrid,
            (Some(_), None) => EventOrigin::Gateway,
            (None, Some(_)) => EventOrigin::AuditLog,
            (None, None) => EventOrigin::Internal,
        }
    }
}
