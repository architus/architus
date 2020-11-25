use jmespath::Expression;
use logs_lib::ActionType;
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Builder for creating a processing chain for gateway actions
pub struct GatewayProcessor {
    events: HashMap<TypeId, EventProcessor>,
}

impl GatewayProcessor {
    pub fn add<T: ?Sized + Any>(mut self, p: EventProcessor) -> Self {
        let event_type = TypeId::of::<T>();
        self.events.insert(event_type, p);
        self
    }
}

/// Represents a shareable static configuration that describes
/// how to translate a single gateway event into a normalized event
pub enum EventProcessor {
    Static {
        action_type: ActionType,
        timestamp_src: TimestampSource,
        subject_id_src: Option<Path>,
        agent_id_src: Option<Path>,
        audit_log_src: Option<AuditLogSource>,
    },
}

pub enum TimestampSource {
    /// Naively sources the timestamp from the time of ingress
    TimeOfIngress,
    /// Uses a JSON query path to extract a timestamp from a Snowflake-formatted ID
    Snowflake(Path),
}

pub struct AuditLogSource {}

pub enum Path {
    Gateway(Expression<'static>),
    AuditLog(Expression<'static>),
}

impl Path {
    /// Creates a gateway path from the given string,
    /// panicking if there was a parsing error.
    /// Only use in initialization pathways that will fail-fast
    pub fn gateway(query: &str) -> Self {
        Self::Gateway(jmespath::compile(query).unwrap())
    }

    /// Creates an audit log path from the given string,
    /// panicking if there was a parsing error.
    /// Only use in initialization pathways that will fail-fast
    pub fn audit_log(query: &str) -> Self {
        Self::AuditLog(jmespath::compile(query).unwrap())
    }
}
