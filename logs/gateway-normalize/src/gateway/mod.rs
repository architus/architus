pub mod sub_processors;

use crate::event::{NormalizedEvent, Source};
use crate::rpc::submission::EventType;
use architus_id::IdProvisioner;
use gateway_queue_lib::GatewayEvent;
use jmespath::Expression;
use static_assertions::assert_impl_all;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use twilight_model::gateway::event::EventType as GatewayEventType;

#[derive(Error, Clone, Debug)]
pub enum ProcessingError {
    #[error("no sub-processor found for event type {0}")]
    SubProcessorNotFound(String),
    #[error("no guild id was parsed for event type {0}")]
    NoGuildId(String),
}

type Result<T> = std::result::Result<T, ProcessingError>;

/// Represents a collection of processors that each have
/// a corresponding gateway event type
/// and are capable of normalizing raw JSON of that type
/// into `NormalizedEvent`s
#[derive(Debug)]
pub struct Processor {
    sub_processors: HashMap<String, EventProcessor>,
    id_provisioner: IdProvisioner,
}

// Processor needs to be safe to share
assert_impl_all!(Processor: Sync);

impl Default for Processor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor {
    /// Creates a processor with an empty set of sub-processors
    #[must_use]
    pub fn new() -> Self {
        Self {
            sub_processors: HashMap::new(),
            id_provisioner: IdProvisioner::new(),
        }
    }

    /// Adds a new sub-processor to this aggregate processor and returns itself,
    /// acting as a builder. Works by serializing the event type into a string
    /// and adding it to the internal map
    #[must_use]
    fn register(mut self, event_type: GatewayEventType, sub_processor: EventProcessor) -> Self {
        let pattern: &[_] = &['\'', '"'];
        let event_type_str =
            serde_json::to_string(&event_type).expect("GatewayEventType was not serializable");
        let trimmed = String::from(event_type_str.trim_matches(pattern));
        self.sub_processors.insert(trimmed, sub_processor);
        self
    }

    /// Returns whether the processor can consume the given event type
    #[must_use]
    pub fn can_process(&self, event_type: &str) -> bool {
        self.sub_processors.contains_key(event_type)
    }

    /// Applies the main data-oriented workflow to the given JSON
    pub async fn normalize(&self, event: GatewayEvent<'_>) -> Result<NormalizedEvent> {
        if let Some(sub_processor) = self.sub_processors.get(event.event_type) {
            return Ok(sub_processor.apply(event, &self.id_provisioner)?);
        }

        Err(ProcessingError::SubProcessorNotFound(String::from(
            event.event_type,
        )))
    }
}

/// Represents a shareable static configuration that describes
/// how to translate a single gateway event into a normalized event
#[derive(Clone, Debug)]
pub enum EventProcessor {
    Static {
        event_type: EventType,
        timestamp_src: TimestampSource,
        subject_id_src: Option<Path>,
        agent_id_src: Option<Path>,
        audit_log_src: Option<AuditLogSource>,
        guild_id_src: Option<Path>,
        channel_id_src: Option<Path>,
        reason_src: Option<Path>,
    },
}

impl EventProcessor {
    /// Applies the event sub-processor to create a normalized event
    pub fn apply(
        &self,
        event: GatewayEvent,
        id_provisioner: &IdProvisioner,
    ) -> Result<NormalizedEvent> {
        match self {
            Self::Static {
                event_type,
                timestamp_src,
                subject_id_src,
                agent_id_src,
                // TODO use audit log source
                audit_log_src: _audit_log_src,
                guild_id_src,
                channel_id_src,
                reason_src,
            } => {
                // TODO add audit log entry support
                let audit_log_entry: Option<serde_json::Value> = None;
                let id = id_provisioner.with_ts(event.ingress_timestamp);

                // Extract all fields based on the static sub-processor definition
                let timestamp = match timestamp_src {
                    TimestampSource::TimeOfIngress => event.ingress_timestamp,
                    TimestampSource::Snowflake(path) => path
                        .apply_id(Some(&event.inner), audit_log_entry.as_ref())
                        .map_or(event.ingress_timestamp, architus_id::extract_timestamp),
                };
                let reason = reason_src
                    .as_ref()
                    .and_then(|path| path.apply(Some(&event.inner), audit_log_entry.as_ref()))
                    .and_then(|value| value.as_string().cloned());
                let subject_id = subject_id_src
                    .as_ref()
                    .and_then(|path| path.apply_id(Some(&event.inner), audit_log_entry.as_ref()));
                let guild_id = guild_id_src
                    .as_ref()
                    .and_then(|path| path.apply_id(Some(&event.inner), audit_log_entry.as_ref()))
                    .ok_or_else(|| ProcessingError::NoGuildId(String::from(event.event_type)))?;
                let agent_id = agent_id_src
                    .as_ref()
                    .and_then(|path| path.apply_id(Some(&event.inner), audit_log_entry.as_ref()));
                let channel_id = channel_id_src
                    .as_ref()
                    .and_then(|path| path.apply_id(Some(&event.inner), audit_log_entry.as_ref()));

                // Construct the source from the original JSON values
                let source = Source {
                    gateway: Some(event.inner),
                    audit_log: audit_log_entry,
                    internal: None,
                };
                let origin = source.origin();

                Ok(NormalizedEvent {
                    id,
                    timestamp,
                    source,
                    origin,
                    event_type: *event_type,
                    reason,
                    guild_id,
                    agent_id,
                    subject_id,
                    channel_id,
                    audit_log_id: None,
                })
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum TimestampSource {
    /// Naively sources the timestamp from the time of ingress
    TimeOfIngress,
    /// Uses a JSON query path to extract a timestamp from a Snowflake-formatted ID
    Snowflake(Path),
}

// TODO implement
#[derive(Clone, Debug)]
pub struct AuditLogSource {}

/// Represents a wrapper around `JMESPath` values to scope a path
/// to the gateway and/or audit log JSON objects
#[derive(Clone, Debug)]
pub enum Path {
    Gateway(Expression<'static>),
    AuditLog(Expression<'static>),
}

impl Path {
    /// Creates a gateway path from the given string,
    /// panicking if there was a parsing error.
    /// Only use in initialization pathways that will fail-fast
    #[must_use]
    pub fn gateway(query: &str) -> Self {
        Self::Gateway(jmespath::compile(query).unwrap())
    }

    /// Creates an audit log path from the given string,
    /// panicking if there was a parsing error.
    /// Only use in initialization pathways that will fail-fast
    #[must_use]
    pub fn audit_log(query: &str) -> Self {
        Self::AuditLog(jmespath::compile(query).unwrap())
    }

    /// Attempts to resolve the path to a JSON value,
    /// using both the gateway and audit log JSON as potential sources
    pub fn apply(
        &self,
        gateway: Option<&serde_json::Value>,
        audit_log: Option<&serde_json::Value>,
    ) -> Option<Arc<jmespath::Variable>> {
        match self {
            Self::Gateway(expr) => match gateway {
                Some(gateway) => expr.search(gateway).ok(),
                None => None,
            },
            Self::AuditLog(expr) => match audit_log {
                Some(audit_log) => expr.search(audit_log).ok(),
                None => None,
            },
        }
    }

    /// Attempts to resolve the path to a u64 value from a string,
    /// using both the gateway and audit log JSON as potential sources
    pub fn apply_id(
        &self,
        gateway: Option<&serde_json::Value>,
        audit_log: Option<&serde_json::Value>,
    ) -> Option<u64> {
        self.apply(gateway, audit_log)
            .and_then(|value| value.as_string().and_then(|s| s.parse::<u64>().ok()))
    }
}
