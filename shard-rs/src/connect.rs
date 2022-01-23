//! Contains utility functions that connect to external services,
//! used during service initialization

use crate::config::Configuration;
use crate::timeout::TimeoutOr;
use anyhow::Context;
use deadpool_postgres::{Pool, Runtime};
use slog::Logger;
use std::sync::Arc;
use tokio_postgres::NoTls;

/// Creates a new database client pool
/// and pings it to ensure that the connection is live.
#[allow(clippy::module_name_repetitions)]
pub async fn connect_to_db(config: Arc<Configuration>, logger: Logger) -> anyhow::Result<Pool> {
    let pool = config
        .database
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .context("could not create database client pool")?;

    let initialization_backoff = config.initialization.backoff.build();
    let timeout = config.initialization.attempt_timeout;
    let ping_database = || async {
        match crate::timeout::timeout(timeout, pool.get()).await {
            Ok(_) => Ok(()),
            Err(err) => {
                let err: TimeoutOr<anyhow::Error> = match &err {
                    TimeoutOr::Timeout(timeout) => {
                        slog::warn!(
                            logger,
                            "getting database client from pool timed out";
                            "timeout" => ?timeout,
                        );
                        TimeoutOr::Timeout(*timeout)
                    }
                    TimeoutOr::Other(inner_err) => {
                        slog::warn!(
                            logger,
                            "getting database client from pool failed";
                            "error" => ?inner_err,
                        );
                        TimeoutOr::Other(anyhow::anyhow!(
                            "getting database client from pool failed: {:?}",
                            inner_err
                        ))
                    }
                };
                Err(backoff::Error::Transient(err))
            }
        }
    };

    backoff::future::retry(initialization_backoff, ping_database)
        .await
        .map_err(|e| anyhow::anyhow!("{:?}", e))
        .context("could not ping database to verify reachability after retrying")?;

    slog::info!(
        logger,
        "connected to database";
    );

    Ok(pool)
}
