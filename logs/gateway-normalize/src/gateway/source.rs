use crate::gateway::path::Path;
use crate::gateway::{CombinedAuditLogEntry, Context, ProcessingError};
use jmespath::Variable;
use static_assertions::assert_impl_all;
use std::future::Future;
use std::pin::Pin;
use twilight_model::guild::audit_log::AuditLogEntry;

pub mod inner {
    use crate::gateway::path::Path;
    use crate::gateway::source::OnFailure;
    use crate::gateway::{Context, ProcessingError};
    use jmespath::Variable;
    use std::future::Future;
    use std::pin::Pin;

    /// Represents a value that is attempted to be extracted from some JSON
    /// using a `JMESPath` value and some parsing/failure-recovery logic
    pub struct JsonSource<T>
    where
        T: Clone + Sync,
    {
        pub path: Path,
        pub extractor:
            Box<dyn (Fn(&Variable, Context<'_>) -> Result<T, anyhow::Error>) + Send + Sync>,
        pub on_failure: OnFailure<T>,
    }

    impl<T> JsonSource<T>
    where
        T: Clone + Sync,
    {
        pub fn consume(
            &self,
            context: Context<'_>,
            json: &serde_json::Value,
        ) -> Result<T, ProcessingError> {
            match (
                self.path.extract(json, &self.extractor, context.clone()),
                &self.on_failure,
            ) {
                (Ok(t), _) => Ok(t),
                (Err(err), OnFailure::Abort) => Err(ProcessingError::FatalSourceError(err)),
                (Err(_), OnFailure::Drop) => Err(ProcessingError::Drop),
                (Err(ref err), OnFailure::Or(t)) => {
                    log::debug!("A failure occurred with running a JSON processor fragment for event '{:?}': {:?}, falling back to default value", context.event.event_type, err);
                    log::trace!("Context = {:?}", context);
                    Ok(t.clone())
                }
                (Err(ref err), OnFailure::OrElse(func_t)) => {
                    log::debug!("A failure occurred with running a JSON processor fragment for event '{:?}': {:?}, falling back to default closure", context.event.event_type, err);
                    log::trace!("Context = {:?}", context);
                    Ok(func_t(context))
                }
            }
        }
    }

    /// Runs an arbitrary synchronous, potentially fallible function that produces a value
    /// given an event normalization context
    /// (that provides access to the raw JSON, Discord API, timestamp, and other metadata)
    pub struct SyncFnSource<T>
    where
        T: Clone + Sync,
    {
        pub f: Box<dyn (Fn(Context<'_>) -> Result<T, anyhow::Error>) + Send + Sync>,
        pub on_failure: OnFailure<T>,
    }

    impl<T> SyncFnSource<T>
    where
        T: Clone + Sync,
    {
        pub fn consume(&self, context: Context<'_>) -> Result<T, ProcessingError> {
            let result = (self.f)(context.clone());
            match (result, &self.on_failure) {
                (Ok(t), _) => Ok(t),
                (Err(err), OnFailure::Abort) => Err(ProcessingError::FatalSourceError(err)),
                (Err(_), OnFailure::Drop) => Err(ProcessingError::Drop),
                (Err(ref err), OnFailure::Or(t)) => {
                    log::debug!("A failure occurred with running a sync processor fragment for event '{:?}': {:?}, falling back to default value", context.event.event_type, err);
                    log::trace!("Context = {:?}", context);
                    Ok(t.clone())
                }
                (Err(ref err), OnFailure::OrElse(func_t)) => {
                    log::debug!("A failure occurred with running a sync processor fragment for event '{:?}': {:?}, falling back to default closure", context.event.event_type, err);
                    log::trace!("Context = {:?}", context);
                    Ok(func_t(context))
                }
            }
        }
    }

    /// Runs an arbitrary asynchronous, potentially fallible function that produces a value
    /// given an event normalization context
    /// (that provides access to the raw JSON, Discord API, timestamp, and other metadata)
    pub struct AsyncFnSource<T>
    where
        T: Clone + Send + Sync,
    {
        pub f: Box<
            dyn (for<'a> Fn(
                    Context<'a>,
                )
                    -> Pin<Box<dyn Future<Output = Result<T, anyhow::Error>> + Send + 'a>>)
                + Send
                + Sync,
        >,
        pub on_failure: OnFailure<T>,
    }

    impl<T> AsyncFnSource<T>
    where
        T: Clone + Send + Sync,
    {
        pub async fn consume(&self, context: Context<'_>) -> Result<T, ProcessingError> {
            match ((*self.f)(context.clone()).await, &self.on_failure) {
                (Ok(t), _) => Ok(t),
                (Err(err), OnFailure::Abort) => Err(ProcessingError::FatalSourceError(err)),
                (Err(_), OnFailure::Drop) => Err(ProcessingError::Drop),
                (Err(ref err), OnFailure::Or(t)) => {
                    log::debug!("A failure occurred with running an async processor fragment for event '{:?}': {:?}, falling back to default value", context.event.event_type, err);
                    log::trace!("Context = {:?}", context);
                    Ok(t.clone())
                }
                (Err(ref err), OnFailure::OrElse(func_t)) => {
                    log::debug!("A failure occurred with running an async processor fragment for event '{:?}': {:?}, falling back to default closure", context.event.event_type, err);
                    log::trace!("Context = {:?}", context);
                    Ok(func_t(context))
                }
            }
        }
    }
}

/// Represents a source for a single field value
/// that is used in the process of event normalization.
/// Allows a single field value to be extracted from a variety of locations,
/// and supports specifying what to do in the case of failure for the fallible variants,
/// such as whether to fall back to some constant/closure result
/// or to abort the entire event normalization.
pub enum Source<T>
where
    T: Clone + Send + Sync,
{
    Constant(T),
    /// Runs an arbitrary synchronous, potentially fallible function that produces a value
    /// given an event normalization context
    /// (that provides access to the raw JSON, Discord API, timestamp, and other metadata)
    SyncFn(inner::SyncFnSource<T>),
    /// Runs an arbitrary asynchronous, potentially fallible function that produces a value
    /// given an event normalization context
    /// (that provides access to the raw JSON, Discord API, timestamp, and other metadata)
    AsyncFn(inner::AsyncFnSource<T>),
    /// Represents a value that is attempted to be extracted from the gateway event JSON
    /// using a JMESPath value and some parsing/failure-recovery logic
    Gateway(inner::JsonSource<T>),
    /// Represents a value that is attempted to be extracted from the audit log JSON
    /// using a JMESPath value and some parsing/failure-recovery logic.
    /// Note that any sources of this variant will always be run after audit log sourcing
    /// logic in the whole `Processor`, which does cause some staggering in the execution order
    AuditLog(inner::JsonSource<T>),
}

/// Specifies the desired behavior when an operation fails to source a value
pub enum OnFailure<T> {
    /// Unexpected scenario that causes the entire event normalization to fail
    /// (no events will be submitted, and the original event will be re-queued)
    Abort,
    /// Expected scenario that results in the original event being dropped and not re-queued
    Drop,
    /// Falls back to some default value
    Or(T),
    /// Falls back to the execution of some infallible synchronous function
    OrElse(fn(Context<'_>) -> T),
}

// Make sure basic source types are sync
assert_impl_all!(Source<u64>: Sync);
assert_impl_all!(Source<String>: Sync);
assert_impl_all!(Source<Option<u64>>: Sync);
assert_impl_all!(Source<crate::rpc::submission::EventType>: Sync);
assert_impl_all!(Source<architus_id::HoarFrost>: Sync);
assert_impl_all!(Source<Vec<u64>>: Sync);

impl<T> Source<T>
where
    T: Clone + Send + Sync,
{
    /// Utility constructor for a `Source::SyncFn` that takes in a static closure
    /// and a failure policy without requiring an explicit boxing on the closure
    pub fn sync_fn<F>(f: F, on_failure: OnFailure<T>) -> Self
    where
        F: (Fn(Context<'_>) -> Result<T, anyhow::Error>) + Send + Sync + 'static,
    {
        Self::SyncFn(inner::SyncFnSource {
            f: Box::new(f),
            on_failure,
        })
    }

    /// Utility constructor for a `Source::AsyncFn` that takes in a static closure
    /// and a failure policy without requiring an explicit boxing on the closure
    pub fn async_fn<F>(f: F, on_failure: OnFailure<T>) -> Self
    where
        for<'a> F: Fn(Context<'a>) -> Pin<Box<dyn Future<Output = Result<T, anyhow::Error>> + Send + 'a>>
            + Sync
            + Send
            + 'static,
    {
        Self::AsyncFn(inner::AsyncFnSource {
            f: Box::new(f),
            on_failure,
        })
    }

    /// Utility constructor for a `Source::Gateway` that takes in a static closure
    /// and a path/failure policy without requiring an explicit boxing on the closure
    pub fn gateway<F>(path: Path, extractor: F, on_failure: OnFailure<T>) -> Self
    where
        F: (Fn(&Variable, Context<'_>) -> Result<T, anyhow::Error>) + Send + Sync + 'static,
    {
        Self::Gateway(inner::JsonSource {
            path,
            extractor: Box::new(extractor),
            on_failure,
        })
    }

    /// Utility constructor for a `Source::AuditLog` that takes in a static closure
    /// and a path/failure policy without requiring an explicit boxing on the closure
    pub fn audit_log<F>(path: Path, extractor: F, on_failure: OnFailure<T>) -> Self
    where
        F: (Fn(&Variable, Context<'_>) -> Result<T, anyhow::Error>) + Send + Sync + 'static,
    {
        Self::AuditLog(inner::JsonSource {
            path,
            extractor: Box::new(extractor),
            on_failure,
        })
    }

    /// Consumes the source using the event normalization context
    /// to produce a result value or an error.
    /// If an error is produced, then that signals to abort the entire event normalization
    pub async fn consume(&self, context: Context<'_>) -> Result<T, ProcessingError> {
        let context_copy = context.clone();
        match self {
            Self::Constant(t) => Ok(t.clone()),
            Self::SyncFn(source) => source.consume(context),
            Self::AsyncFn(source) => source.consume(context).await,
            Self::Gateway(source) => source.consume(context.clone(), &context.event.inner),
            Self::AuditLog(source) => {
                let audit_log_entry = context.audit_log_entry.read().await;
                match (audit_log_entry.as_ref(), &source.on_failure) {
                    (Some(CombinedAuditLogEntry { json, .. }), _) => {
                        // Consume using the normal JSON source using the audit log json
                        source.consume(context.clone(), json)
                    }
                    (None, OnFailure::Abort) => Err(ProcessingError::NoAuditLogEntry(
                        String::from(context.event.event_type),
                    )),
                    (None, OnFailure::Drop) => Err(ProcessingError::Drop),
                    (None, OnFailure::Or(t)) => Ok(t.clone()),
                    (None, OnFailure::OrElse(func_t)) => Ok(func_t(context_copy)),
                }
            }
        }
    }
}

/// Defines the sourcing logic for an audit log entry
/// using an inner asynchronous fallible function
/// and a failure recovery policy
#[allow(clippy::module_name_repetitions)]
pub struct AuditLogSource(inner::AsyncFnSource<Option<AuditLogEntry>>);

// Make sure source is sync
assert_impl_all!(AuditLogSource: Sync);

impl AuditLogSource {
    /// Utility constructor for the audit log source
    /// that takes in an async closure that is run when sourcing,
    /// in addition to the failure recovery policy
    pub fn new<F>(f: F, on_failure: OnFailure<Option<AuditLogEntry>>) -> Self
    where
        for<'a> F: Fn(
                Context<'a>,
            ) -> Pin<
                Box<dyn Future<Output = Result<Option<AuditLogEntry>, anyhow::Error>> + Send + 'a>,
            > + Sync
            + Send
            + 'static,
    {
        Self(inner::AsyncFnSource {
            f: Box::new(f),
            on_failure,
        })
    }

    /// Actually consumes the audit log source given an event normalization context
    pub async fn consume(
        &self,
        context: Context<'_>,
    ) -> Result<Option<AuditLogEntry>, ProcessingError> {
        self.0.consume(context).await
    }
}
