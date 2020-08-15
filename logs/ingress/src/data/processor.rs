use logs_lib::ActionType;
use jmespath::Expression;

/// Represents a shareable static configuration that describes
/// how to translate gateway actions into normalized events
pub struct GatewayProcessor {
    pub action_type: ActionType,
    pub timestamp_src: TimestampSource,
    pub subject_id_src: Option<Path>,
    pub agent_id_src: Option<Path>,
    pub audit_log_src: Option<AuditLogSource>,
}

pub enum TimestampSource {
    /// Naively sources the timestamp from the time of ingress
    TimeOfIngress,
    /// Uses a JSON query path to extract a timestamp from a Snowflake-formatted ID
    Snowflake(Path),
}

pub struct AuditLogSource {
    
}

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

