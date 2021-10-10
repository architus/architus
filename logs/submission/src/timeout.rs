//! Small utility to handle timeouts in async operations

// #![allow(clippy::future_not_send)]

use std::error::Error;
use std::fmt::Debug;
use std::future::Future;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum TimeoutOr<T>
where
    T: Error,
{
    #[error("operation timed out after {0:?}")]
    Timeout(Duration),
    #[error(transparent)]
    Other(#[from] T),
}

/// Wraps a future in a timeout,
/// returning:
/// - `TimeoutOr::Timeout` error result variant in the case of a timeout
/// - `TimeoutOr::Other` error result variant in the case
///    of an error output of the inner future
pub async fn timeout<F, S, E>(duration: Duration, future: F) -> Result<S, TimeoutOr<E>>
where
    F: Future<Output = Result<S, E>>,
    E: Error,
{
    match tokio::time::timeout(duration, future).await {
        Ok(Ok(success)) => Ok(success),
        Ok(Err(err)) => Err(TimeoutOr::Other(err)),
        Err(_) => Err(TimeoutOr::Timeout(duration)),
    }
}
