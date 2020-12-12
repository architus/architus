use crate::config::Configuration;
use crate::feature_gate::GuildFeature;
use crate::UptimeEvent;
use anyhow::Result;
use backoff::future::FutureOperation;
use futures::Stream;
use static_assertions::assert_impl_all;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::broadcast;

pub type FeatureGateClient =
    crate::feature_gate::feature_gate_client::FeatureGateClient<tonic::transport::Channel>;

/// Represents a shared handler that continuously polls the feature service
/// and sits between the connection tracker and the uptime service
/// to maintain a pool of the actively listened guilds
/// and take the intersection of those guilds with the ones that have indexing enabled
#[derive(Clone, Debug)]
pub struct ActiveGuilds {
    config: Arc<Configuration>,
    guilds: Arc<RwLock<HashMap<u64, GuildStatus>>>,
    feature_gate_client: Arc<FeatureGateClient>,
}

/// Represents the cached online/offline + indexing enabled/disabled status of a single guild
#[derive(Clone, Debug)]
enum GuildStatus {
    Loaded {
        is_active: bool,
        // The moment that the guild went offline is used for eviction purposes
        // upon periodic polling
        eviction_timer_start: Option<Instant>,
    },
    /// If a guild is loading its active status,
    /// then there will be a broadcast channel in this map
    /// that can be subscribed to once the entry is populated.
    Loading(broadcast::Sender<bool>),
}

assert_impl_all!(ActiveGuilds: Sync, Send);

impl ActiveGuilds {
    /// Creates a new shared handler and wraps the connection to the feature gate service
    pub fn new(feature_gate_client: FeatureGateClient, config: Arc<Configuration>) -> Self {
        Self {
            config,
            guilds: Arc::new(RwLock::new(HashMap::new())),
            feature_gate_client: Arc::new(feature_gate_client),
        }
    }

    /// Runs a task that continuously polls the feature gate to maintain an active list of guilds
    /// that have log indexing enabled
    pub async fn go_poll(&self) -> Result<()> {
        // TODO implement
        Ok(())
    }

    /// Filters uptime events to ensure that they only contain active guilds
    /// that have events that are actually forwarded
    pub fn pipe_uptime_events(
        &self,
        in_stream: impl Stream<Item = UptimeEvent>,
    ) -> impl Stream<Item = UptimeEvent> {
        // TODO implement
        in_stream
    }

    /// Determines whether the given `guild_id` should have events forwarded to the queue
    /// If not tracked, then asynchronously loads this guild
    /// Note: clippy lint ignore is due to bug I discovered;
    /// remove once `https://github.com/rust-lang/rust-clippy/issues/6446` is addressed
    #[allow(clippy::await_holding_lock)]
    pub async fn is_active(&self, guild_id: u64) -> bool {
        // Provide the opportunity to try again
        // if a non-connection error or data race is encountered
        loop {
            let guilds_read = self.guilds.read().expect("active guilds lock poisoned");
            return match guilds_read.get(&guild_id) {
                Some(GuildStatus::Loaded { is_active, .. }) => *is_active,
                Some(GuildStatus::Loading(loading_tx)) => {
                    // Start waiting on the channel
                    // Note: since we require that signalers acquire the write lock
                    // before signaling/removing, this is data-race-free
                    let mut loaded_rx = loading_tx.subscribe();
                    drop(guilds_read);
                    match loaded_rx.recv().await {
                        Ok(is_active) => is_active,
                        Err(err) => {
                            log::warn!(
                                "An error occurred while waiting on guild to be loaded: {:?}",
                                err
                            );
                            // Try again
                            continue;
                        }
                    }
                }
                None => {
                    // Start loading the guild manually
                    // Note: since we can't upgrade our read lock in-place,
                    // we have to drop and re-acquire it.
                    // Because this is not atomic, there is a possible data race
                    // where the guild has been loaded once we have the write lock.
                    // To mitigate this, we check to see if the map has a value once we have the write lock,
                    // and if so, drop the lock and try to read the status again
                    drop(guilds_read);
                    let mut guilds_write =
                        self.guilds.write().expect("active guilds lock poisoned");
                    if guilds_write.get(&guild_id).is_some() {
                        continue;
                    }

                    // Insert a new loading channel into the map and drop the lock
                    // (we use an unused variable binding on the receiver
                    // to ensure that signaling doesn't produce an error
                    // since the RAII receiver in the variable binding is dropped
                    // at the end of the scope)
                    let (loading_tx, _receiver) = broadcast::channel(1);
                    guilds_write.insert(guild_id, GuildStatus::Loading(loading_tx.clone()));
                    drop(guilds_write);

                    // Fetch the guild from the feature server using a backoff
                    let send = || async {
                        let mut feature_gate_client = (*self.feature_gate_client).clone();
                        let result = feature_gate_client
                            .check_guild_feature(GuildFeature {
                                feature_name: self.config.indexing_feature.clone(),
                                guild_id,
                            })
                            .await;
                        consume_rpc_result(result)
                    };
                    let result = send.retry(self.config.rpc_backoff.build()).await;
                    let is_active = result.map_or_else(|err| {
                        log::warn!("Could not contact the feature-gate service for information about indexing on guild {}: {:?}", guild_id, err);
                        // Default to true if the feature gate cannot be contacted
                        // for graceful degradation
                        true
                    }, |r| r.has_feature);
                    let default_status = GuildStatus::Loaded {
                        is_active,
                        // Note: we start the eviction timer for eagerly loaded guilds
                        // If we receive the Online event for the guild, the timer will be removed
                        eviction_timer_start: Some(Instant::now()),
                    };

                    // Broadcast the value on the channel after acquiring the write lock again
                    let mut guilds_write =
                        self.guilds.write().expect("active guilds lock poisoned");
                    if let Err(err) = loading_tx.send(is_active) {
                        // This should be impossible since we retain a reference to a receiver,
                        // but in case it isn't:
                        log::warn!(
                            "An error occurred while sending loading information: {:?}",
                            err
                        );
                    }

                    // There is a data race where another load could have been performed,
                    // so mutate the entry in-place
                    guilds_write
                        .entry(guild_id)
                        .and_modify(|status| {
                            match status {
                                GuildStatus::Loaded {
                                    is_active: active, ..
                                } => *active = is_active,
                                _ => *status = default_status.clone(),
                            };
                        })
                        .or_insert(default_status);

                    is_active
                }
            };
        }
    }
}

/// Transforms an RPC result into a more useful one,
/// and prepares a backoff error for potentially recoverable tonic Status's
fn consume_rpc_result<T>(
    result: Result<tonic::Response<T>, tonic::Status>,
) -> Result<T, backoff::Error<tonic::Status>> {
    match result {
        Ok(response) => Ok(response.into_inner()),
        Err(status) => match status.code() {
            tonic::Code::Internal | tonic::Code::Unknown | tonic::Code::Unavailable => {
                Err(backoff::Error::Permanent(status))
            }
            _ => Err(backoff::Error::Transient(status)),
        },
    }
}
