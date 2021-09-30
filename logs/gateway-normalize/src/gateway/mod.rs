pub mod path;
pub mod processors;

use crate::config::Configuration;
use crate::event::NormalizedEvent;
use crate::gateway::path::Path;
use crate::rpc::gateway_queue_lib::GatewayEvent;
use crate::util;
use jmespath::Variable;
use slog::Logger;
use static_assertions::assert_impl_all;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;
use twilight_http::Client;
use twilight_model::gateway::event::EventType as GatewayEventType;

/// Wrapper around `GatewayEvent` that includes the deserialized JSON
pub struct EventWithSource {
    inner: GatewayEvent,
    source: serde_json::Value,
}

impl TryFrom<GatewayEvent> for EventWithSource {
    type Error = rmp_serde::decode::Error;

    fn try_from(value: GatewayEvent) -> Result<Self, Self::Error> {
        let source = rmp_serde::from_slice::<serde_json::Value>(&value.inner)?;
        Ok(Self {
            inner: value,
            source,
        })
    }
}

/// Enumerates the various errors that can occur during event processing
/// that cause it to halt
#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum ProcessingError {
    #[error("no sub-processor found for event type {0}")]
    SubProcessorNotFound(String),
    #[error("fatal sourcing error encountered: {0}")]
    FatalSourceError(anyhow::Error),
    #[error("dropping original gateway event")]
    Drop,
}

impl ProcessingError {
    /// Whether the error occurs in a non-nominal case that should be logged
    pub const fn is_unexpected(&self) -> bool {
        matches!(
            self,
            Self::SubProcessorNotFound(_) | Self::FatalSourceError(_)
        )
    }

    /// Whether the error should result in a re-queue
    pub const fn should_requeue(&self) -> bool {
        !matches!(self, Self::FatalSourceError(_))
    }
}

/// Represents a collection of processors that each have
/// a corresponding gateway event type
/// and are capable of normalizing raw JSON of that type
/// into `NormalizedEvent`s
pub struct ProcessorFleet {
    processors: HashMap<String, Processor>,
    client: Client,
    config: Arc<Configuration>,
    emojis: Arc<crate::emoji::Db>,
    logger: Logger,
}

// ProcessorFleet needs to be safe to share
assert_impl_all!(ProcessorFleet: Sync);

impl ProcessorFleet {
    /// Creates a processor with an empty set of sub-processors
    #[must_use]
    pub fn new(
        client: Client,
        config: Arc<Configuration>,
        emojis: Arc<crate::emoji::Db>,
        logger: Logger,
    ) -> Self {
        Self {
            processors: HashMap::new(),
            client,
            config,
            emojis,
            logger,
        }
    }

    /// Adds a new sub-processor to this aggregate processor,
    /// working by serializing the event type into a string
    /// and adding it to the internal map
    fn register(&mut self, event_type: GatewayEventType, processor: Processor) {
        let event_key = util::value_to_string(&event_type).unwrap();
        self.processors.insert(event_key, processor);
    }

    /// Applies the main data-oriented workflow to the given JSON
    pub async fn normalize(
        &self,
        event: EventWithSource,
    ) -> Result<NormalizedEvent, ProcessingError> {
        if let Some(processor) = self.processors.get(&event.inner.event_type) {
            let logger = self.logger.new(slog::o!(
                "event_ingress_timestamp" => event.inner.ingress_timestamp,
                "event_type" => event.inner.event_type.clone(),
                "guild_id" => event.inner.guild_id
            ));
            processor
                .apply(event, &self.client, &self.config, &self.emojis, &logger)
                .await
        } else {
            Err(ProcessingError::SubProcessorNotFound(
                event.inner.event_type,
            ))
        }
    }
}

type ProcessorResult = Result<NormalizedEvent, anyhow::Error>;
type AsyncProcessorFuture<'a> = Box<dyn Future<Output = ProcessorResult> + Send + 'a>;

pub enum Processor {
    Sync(Box<dyn (Fn(ProcessorArgs<'_>) -> ProcessorResult) + Send + Sync>),

    #[allow(dead_code)]
    Async(Box<dyn (for<'a> Fn(ProcessorArgs<'a>) -> Pin<AsyncProcessorFuture<'a>>) + Send + Sync>),
}

// ProcessorFleet needs to be safe to share
assert_impl_all!(Processor: Sync);

impl Processor {
    pub fn sync<F>(f: F) -> Self
    where
        F: (Fn(ProcessorArgs<'_>) -> ProcessorResult) + Send + Sync + 'static,
    {
        Self::Sync(Box::new(f))
    }

    #[allow(dead_code)]
    pub fn r#async<F>(f: F) -> Self
    where
        for<'a> F: Fn(ProcessorArgs<'a>) -> Pin<AsyncProcessorFuture<'a>> + Sync + Send + 'static,
    {
        Self::Async(Box::new(f))
    }

    /// Runs a single processor, attempting to create a Normalized Event as a result
    pub async fn apply<'a>(
        &self,
        event: EventWithSource,
        client: &'a Client,
        config: &'a Configuration,
        emojis: &'a crate::emoji::Db,
        logger: &'a Logger,
    ) -> Result<NormalizedEvent, ProcessingError> {
        let EventWithSource {
            inner: event,
            source,
        } = event;

        let processor_args = ProcessorArgs {
            client,
            config,
            emojis,
            logger,
            event,
            inner: source,
        };

        let result = match self {
            Self::Sync(f) => f(processor_args),
            Self::Async(f) => f(processor_args).await,
        };

        result.map_err(ProcessingError::FatalSourceError)
    }
}

pub struct ProcessorArgs<'a> {
    event: GatewayEvent,
    inner: serde_json::Value,
    client: &'a Client,
    config: &'a Configuration,
    emojis: &'a crate::emoji::Db,
    logger: &'a Logger,
}

impl<'a> ProcessorArgs<'a> {
    fn context<'b: 'a>(&'b self) -> Context<'b> {
        Context {
            event: &self.event,
            source: &self.inner,
            client: self.client,
            config: self.config,
            emojis: self.emojis,
            logger: self.logger,
        }
    }
}

/// Struct of borrows/references to various values
/// that might be useful when normalizing an incoming event,
/// including the source data.
/// Can be cheaply cloned.
#[derive(Clone, Debug)]
pub struct Context<'a> {
    event: &'a GatewayEvent,
    source: &'a serde_json::Value,
    client: &'a Client,
    config: &'a Configuration,
    emojis: &'a crate::emoji::Db,
    logger: &'a Logger,
}

#[allow(dead_code)]
impl Context<'_> {
    /// Attempts to extract a gateway value
    pub fn gateway<T, F>(&self, path: &Path, extractor: F) -> Result<T, anyhow::Error>
    where
        F: (Fn(&Variable, Context<'_>) -> Result<T, anyhow::Error>) + Send + Sync + 'static,
    {
        path.extract(self.source, &extractor, self.clone())
    }
}
