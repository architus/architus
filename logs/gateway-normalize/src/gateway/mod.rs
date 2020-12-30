pub mod path;
pub mod processors;
pub mod source;

use crate::event::{Agent, Channel, Content, Entity, NormalizedEvent, Source as EventSource};
use crate::gateway::source::{AuditLogSource, Source};
use crate::rpc::submission::EventType;
use anyhow::Context as _;
use architus_id::IdProvisioner;
use futures::try_join;
use gateway_queue_lib::GatewayEvent;
use static_assertions::assert_impl_all;
use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use twilight_model::gateway::event::EventType as GatewayEventType;
use twilight_model::guild::audit_log::AuditLogEntry;

/// Enumerates the various errors that can occur during event processing
/// that cause it to halt
#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("no sub-processor found for event type {0}")]
    SubProcessorNotFound(String),
    #[error("fatal sourcing error encountered: {0}")]
    FatalSourceError(anyhow::Error),
    #[error("no audit log entry for event type {0} was sourced, but it is used to source a required field")]
    NoAuditLogEntry(String),
}

/// Represents a collection of processors that each have
/// a corresponding gateway event type
/// and are capable of normalizing raw JSON of that type
/// into `NormalizedEvent`s
pub struct ProcessorFleet {
    processors: HashMap<String, Processor>,
    id_provisioner: IdProvisioner,
}

// Processor needs to be safe to share
assert_impl_all!(ProcessorFleet: Sync);

impl Default for ProcessorFleet {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessorFleet {
    /// Creates a processor with an empty set of sub-processors
    #[must_use]
    pub fn new() -> Self {
        Self {
            processors: HashMap::new(),
            id_provisioner: IdProvisioner::new(),
        }
    }

    /// Adds a new sub-processor to this aggregate processor and returns itself,
    /// acting as a builder. Works by serializing the event type into a string
    /// and adding it to the internal map
    #[must_use]
    fn register(mut self, event_type: GatewayEventType, processor: Processor) -> Self {
        let pattern: &[_] = &['\'', '"'];
        let event_type_str =
            serde_json::to_string(&event_type).expect("GatewayEventType was not serializable");
        let trimmed = String::from(event_type_str.trim_matches(pattern));
        self.processors.insert(trimmed, processor);
        self
    }

    /// Applies the main data-oriented workflow to the given JSON
    pub async fn normalize(
        &self,
        event: GatewayEvent<'_>,
    ) -> Result<NormalizedEvent, ProcessingError> {
        if let Some(processor) = self.processors.get(event.event_type) {
            processor.apply(event, &self.id_provisioner).await
        } else {
            Err(ProcessingError::SubProcessorNotFound(String::from(
                event.event_type,
            )))
        }
    }
}

pub struct Processor {
    audit_log: Option<AuditLogSource>,
    event_type: Source<EventType>,
    timestamp: Source<u64>,
    reason: Source<Option<String>>,
    channel: Source<Option<Channel>>,
    agent: Source<Option<Agent>>,
    subject: Source<Option<Entity>>,
    auxiliary: Source<Option<Entity>>,
    content: Source<Content>,
}

impl Processor {
    /// Runs a single processor, attempting to create a Normalized Event as a result
    pub async fn apply<'a>(
        &self,
        event: GatewayEvent<'a>,
        id_provisioner: &'a IdProvisioner,
    ) -> Result<NormalizedEvent, ProcessingError> {
        // Create a RwLock that source objects can wait on if needed
        let audit_log_lock: RwLock<Option<CombinedAuditLogEntry>> = RwLock::new(None);
        let ctx = Context {
            event: &event,
            id_provisioner,
            audit_log_entry: LockReader::new(&audit_log_lock),
        };

        let write_lock = if self.audit_log.is_some() {
            // Acquire a write lock and then move it into a future,
            // so that any readers will always block when entering their futures
            Some(audit_log_lock.write().await)
        } else {
            None
        };

        // Run each source in parallel
        let (_, event_type, timestamp, reason, channel, agent, subject, auxiliary, content) = try_join!(
            self.load_audit_log(write_lock, ctx.clone()),
            self.event_type.consume(ctx.clone()),
            self.timestamp.consume(ctx.clone()),
            self.reason.consume(ctx.clone()),
            self.channel.consume(ctx.clone()),
            self.agent.consume(ctx.clone()),
            self.subject.consume(ctx.clone()),
            self.auxiliary.consume(ctx.clone()),
            self.content.consume(ctx.clone()),
        )?;

        drop(ctx);
        let id = event.id;
        let guild_id = event.guild_id;
        let audit_log_entry = audit_log_lock.into_inner();
        let audit_log_id = audit_log_entry.as_ref().map(|combined| combined.entry.id.0);
        let audit_log_json = audit_log_entry.map(|combined| combined.json);
        let source = EventSource {
            gateway: Some(event.inner),
            audit_log: audit_log_json,
            ..EventSource::default()
        };
        let origin = source.origin();

        Ok(NormalizedEvent {
            id,
            timestamp,
            source,
            origin,
            event_type,
            guild_id,
            reason,
            audit_log_id,
            channel,
            agent,
            subject,
            auxiliary,
            content,
        })
    }

    /// Asynchronously loads an audit log entry,
    /// taking in a write lock at the beginning to ensure that any readers
    /// are blocked until loading is complete (if performed)
    async fn load_audit_log(
        &self,
        write_lock: Option<RwLockWriteGuard<'_, Option<CombinedAuditLogEntry>>>,
        context: Context<'_>,
    ) -> Result<(), ProcessingError> {
        if let Some(audit_log_source) = &self.audit_log {
            // Invariant: if audit_log_source is non-none, then write_lock is too
            let mut write_lock = write_lock.unwrap();
            match audit_log_source.consume(context).await {
                // Lock is released upon returning
                Err(err) => Err(err),
                Ok(None) => Ok(()),
                Ok(Some(audit_log_entry)) => {
                    let audit_log_json = serde_json::to_value(&audit_log_entry)
                        .with_context(|| {
                            format!(
                                "could not serialize audit log entry to JSON: {:?}",
                                audit_log_entry
                            )
                        })
                        .map_err(ProcessingError::FatalSourceError)?;
                    *write_lock = Some(CombinedAuditLogEntry {
                        entry: audit_log_entry,
                        json: audit_log_json,
                    });
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }
}

/// Represents a sourced audit log entry
/// that has been pre-serialized to JSON
/// to use with path-based field sources
#[derive(Clone, Debug)]
pub struct CombinedAuditLogEntry {
    entry: AuditLogEntry,
    json: serde_json::Value,
}

/// Struct of borrows/references to various values
/// that might be useful when normalizing an incoming event,
/// including the source data.
/// Can be cheaply cloned.
#[derive(Clone, Debug)]
pub struct Context<'a> {
    event: &'a GatewayEvent<'a>,
    id_provisioner: &'a IdProvisioner,
    audit_log_entry: LockReader<'a, Option<CombinedAuditLogEntry>>,
}

#[derive(Clone, Debug)]
pub struct LockReader<'a, T> {
    inner: &'a RwLock<T>,
}

impl<'a, T> LockReader<'a, T> {
    /// Asynchronously obtains a read-only handle to the inner lock
    #[allow(clippy::future_not_send)]
    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.inner.read().await
    }

    const fn new(lock: &'a RwLock<T>) -> Self {
        Self { inner: lock }
    }
}
