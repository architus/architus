use crate::config::Configuration;
use crate::feature_gate::{BatchCheck, GuildFeature};
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
    Loaded(LoadedState),
    /// If a guild is loading its active status,
    /// then there will be a broadcast channel in this map
    /// that can be subscribed to once the entry is populated.
    Loading(broadcast::Sender<bool>),
}

#[derive(Clone, Debug)]
struct LoadedState {
    is_active: bool,
    connection: GuildConnection,
}

enum ActiveEdge {
    Rising,
    Falling,
}

impl LoadedState {
    fn update(&mut self, active: bool) -> Option<ActiveEdge> {
        let active_before = self.active();
        self.is_active = active;
        let active_after = self.active();
        match (active_before, active_after) {
            (true, false) => Some(ActiveEdge::Falling),
            (false, true) => Some(ActiveEdge::Rising),
            _ => None,
        }
    }

    const fn active(&self) -> bool {
        match self.connection {
            GuildConnection::Online => self.is_active,
            GuildConnection::Offline(_) => false,
        }
    }
}

#[derive(Clone, Debug)]
enum GuildConnection {
    Online,
    // Contains the moment that the guild went offline is used for eviction purposes
    // upon periodic polling
    Offline(Instant),
}

impl GuildConnection {
    fn offline() -> Self {
        Self::Offline(Instant::now())
    }
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
    /// Note: clippy lint ignore is due to bug I discovered;
    /// remove once `https://github.com/rust-lang/rust-clippy/issues/6446` is addressed
    #[allow(clippy::await_holding_lock)]
    pub async fn go_poll(&self) -> Result<()> {
        loop {
            tokio::time::delay_for(self.config.active_guilds_poll_interval).await;

            // Get a list of all active guilds by their guild ids (only include loaded ones;
            // loading ones will be eagerly loaded faster than we can bulk-fetch them anyways)
            let guilds_read = self.guilds.read().expect("active guilds lock poisoned");
            let active_guilds = guilds_read
                .iter()
                .filter_map(|(guild_id, status)| match status {
                    GuildStatus::Loading(_) => None,
                    GuildStatus::Loaded { .. } => Some(guild_id),
                })
                .cloned()
                .collect::<Vec<_>>();
            drop(guilds_read);

            // Chunk the active guild ids by the batch check's max size and send a request for each
            let mut results = HashMap::<u64, bool>::with_capacity(active_guilds.len());
            for guild_chunk in active_guilds.chunks(self.config.feature_gate_batch_check_size) {
                let send = || async {
                    let mut feature_gate_client = (*self.feature_gate_client).clone();
                    let result = feature_gate_client
                        .batch_check_guild_features(BatchCheck {
                            feature_name: self.config.indexing_feature.clone(),
                            guild_ids: Vec::from(guild_chunk),
                        })
                        .await;
                    consume_rpc_result(result)
                };
                let result = send.retry(self.config.rpc_backoff.build()).await;
                match result {
                    Ok(batch_result) => {
                        if batch_result.has_feature.len() != guild_chunk.len() {
                            log::warn!(
                                "feature-gate service returned batch result with different length than input; expected: {}, actual: {}",
                                guild_chunk.len(),
                                batch_result.has_feature.len()
                            );
                            // Ignore and move to the next chunk
                            continue;
                        }
                        for (guild_id, result) in guild_chunk.iter().zip(batch_result.has_feature) {
                            results.insert(*guild_id, result);
                        }
                    }
                    Err(err) => {
                        log::warn!("An error occurred while sending RPC message to feature-gate service: {:?}", err);
                        // Ignore and move to the next poll interval
                        continue;
                    }
                }
            }

            // Acquire a write lock and merge the results of the polling into the map
            let mut guilds_write = self.guilds.write().expect("active guilds lock poisoned");
            let mut rising_edge_guilds = Vec::<u64>::new();
            let mut falling_edge_guilds = Vec::<u64>::new();
            for (guild_id, is_active) in results {
                // Only update the status if it is still in the map
                guilds_write.entry(guild_id).and_modify(|status| {
                    if let GuildStatus::Loaded(state) = status {
                        // Note the result of the update,
                        // and prepare to send guild online/offline messages if the edge changes
                        match state.update(is_active) {
                            Some(ActiveEdge::Rising) => rising_edge_guilds.push(guild_id),
                            Some(ActiveEdge::Falling) => falling_edge_guilds.push(guild_id),
                            None => {}
                        }
                    }

                    // Do nothing if the guild is loading
                    // (this shouldn't be possible since we filtered, but there might be
                    // a rapid offline->online scenario that could cause this)
                });
            }

            // Check for evictions
            let mut to_evict = Vec::<u64>::new();
            for (guild_id, status) in guilds_write.iter_mut() {
                if let GuildStatus::Loaded(LoadedState {
                    connection: GuildConnection::Offline(eviction_timer_start),
                    ..
                }) = status
                {
                    let elapsed = Instant::now().duration_since(*eviction_timer_start);
                    if elapsed > self.config.active_guild_eviction_duration {
                        // Evict the guild
                        to_evict.push(*guild_id);
                    }
                }
            }
            for guild_id in to_evict {
                guilds_write.remove(&guild_id);
            }
            drop(guilds_write);

            // TODO Send polling edge results to shared channel
        }
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
                Some(GuildStatus::Loaded(LoadedState { is_active, .. })) => *is_active,
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
                        log::warn!(
                            "Could not contact the feature-gate service for information about indexing on guild {}: {:?}",
                            guild_id,
                            err
                        );
                        // Default to true if the feature gate cannot be contacted
                        // for graceful degradation
                        true
                    }, |r| r.has_feature);
                    let default_status = GuildStatus::Loaded(LoadedState {
                        is_active,
                        // Note: we set eagerly loaded guilds as offline
                        // If we receive the Online event for the guild, the status will be updated
                        connection: GuildConnection::offline(),
                    });

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
                                GuildStatus::Loaded(LoadedState {
                                    is_active: active, ..
                                }) => *active = is_active,
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
