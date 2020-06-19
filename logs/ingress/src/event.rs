use logs_lib::id::HoarFrost;
use logs_lib::{ActionOrigin, ActionType, to_json};
use serde::ser;
use serde::Serialize;
use serenity::model::id::{AuditLogEntryId, GuildId, UserId};

/// Normalized log event to send to log ingestion
#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct NormalizedEvent {
    /// Id using snowflake format,
    /// using the *time that the event was received by wherever it's ingested*
    pub id: HoarFrost,
    /// Unix timestamp of the *time of the underlying event* (if available)
    pub timestamp: u64,
    /// The source data, including the original gateway/audit log entries
    pub source: Source,
    /// The origin of the event
    pub origin: ActionOrigin,
    /// The type of action the event is
    pub action_type: ActionType,
    /// An optional *human-readable* reason/message of the event
    pub reason: Option<String>,
    /// Related guild the event ocurred in
    pub guild_id: Option<GuildId>,
    /// Id of the entity that caused the event to occur
    pub agent_id: Option<UserId>,
    /// Id of the entity that the event is about/affects
    /// (can be any Id type)
    pub subject_id: Option<u64>,
    /// Id of the corresponding audit log entry this event corresponds to, if any
    /// (included for indexing purposes)
    pub audit_log_id: Option<AuditLogEntryId>,
}

#[derive(Clone, PartialEq, Debug, Serialize)]
pub struct Source {
    pub gateway: Option<serde_json::Value>,
    pub audit_log: Option<serde_json::Value>,
}

impl Source {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            gateway: None,
            audit_log: None,
        }
    }

    #[must_use]
    pub fn gateway<T: ser::Serialize>(gateway_event: &T) -> Self {
        Self {
            gateway: to_json(gateway_event),
            audit_log: None,
        }
    }
}
