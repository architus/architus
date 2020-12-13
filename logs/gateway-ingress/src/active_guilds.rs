use crate::config::Configuration;
use crate::feature_gate::{BatchCheck, GuildFeature};
use crate::UptimeEvent;
use anyhow::Result;
use backoff::future::FutureOperation;
use futures::{Stream, StreamExt};
use static_assertions::assert_impl_all;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tokio::sync::broadcast;
use tokio::sync::mpsc;

pub type FeatureGateClient =
    crate::feature_gate::feature_gate_client::FeatureGateClient<tonic::transport::Channel>;

/// Represents the cached online/offline + indexing enabled/disabled status of a single guild
#[derive(Clone, Debug)]
enum GuildStatus {
    Loaded(LoadedState),
    Loading(LoadingNotifier),
}

#[derive(Clone, Debug)]
struct LoadedState {
    is_active: bool,
    connection: GuildConnection,
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

    fn update_connection(&mut self, connection: GuildConnection) -> Option<ActiveEdge> {
        let active_before = self.active();
        self.connection = connection;
        let active_after = self.active();
        match (active_before, active_after) {
            (true, false) => Some(ActiveEdge::Falling),
            (false, true) => Some(ActiveEdge::Rising),
            _ => None,
        }
    }

    fn update_with_connection(
        &mut self,
        active: bool,
        connection: GuildConnection,
    ) -> Option<ActiveEdge> {
        let active_before = self.active();
        self.is_active = active;
        self.connection = connection;
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
enum ActiveEdge {
    Rising,
    Falling,
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

#[derive(Debug)]
struct LoadingNotifier {
    inner: broadcast::Sender<bool>,
    // we use an unused variable binding on the receiver
    // to ensure that signaling doesn't produce an error
    // since the RAII receiver in the variable binding is dropped
    // after loading has completed
    _receiver: broadcast::Receiver<bool>,
}

impl LoadingNotifier {
    fn new() -> Self {
        // Note: we need the limit to be 2 in case a processed upstream uptime event and eager load
        // both operate on the same guild
        let (loading_tx, loading_rx) = broadcast::channel(2);
        Self {
            inner: loading_tx,
            _receiver: loading_rx,
        }
    }

    fn subscribe(&self) -> broadcast::Receiver<bool> {
        self.inner.subscribe()
    }

    fn notify(&self, is_active: bool) {
        if let Err(err) = self.inner.send(is_active) {
            // This should be impossible since we retain a reference to a receiver,
            // but in case it isn't:
            log::warn!(
                "An error occurred while sending loading information: {:?}",
                err
            );
        }
    }
}

impl Clone for LoadingNotifier {
    /// Note: this function is only safe to call
    /// while the write lock on the map has been acquired
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _receiver: self.inner.subscribe(),
        }
    }
}

/// Represents a shared handler that continuously polls the feature service
/// and sits between the connection tracker and the uptime service
/// to maintain a pool of the actively listened guilds
/// and take the intersection of those guilds with the ones that have indexing enabled
#[derive(Clone, Debug)]
pub struct ActiveGuilds {
    config: Arc<Configuration>,
    guilds: Arc<RwLock<HashMap<u64, GuildStatus>>>,
    feature_gate_client: Arc<FeatureGateClient>,
    uptime_event_tx: Arc<mpsc::UnboundedSender<UptimeEvent>>,
}

assert_impl_all!(ActiveGuilds: Sync, Send);

impl ActiveGuilds {
    /// Creates a new shared handler and wraps the connection to the feature gate service.
    /// Returns a stream sink that uptime events get piped to
    pub fn new(
        feature_gate_client: FeatureGateClient,
        config: Arc<Configuration>,
    ) -> (Self, impl Stream<Item = UptimeEvent>) {
        let (uptime_event_tx, uptime_event_rx) = mpsc::unbounded_channel::<UptimeEvent>();
        let new_self = Self {
            config,
            guilds: Arc::new(RwLock::new(HashMap::new())),
            feature_gate_client: Arc::new(feature_gate_client),
            uptime_event_tx: Arc::new(uptime_event_tx),
        };
        (new_self, uptime_event_rx)
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
            let loaded_guilds = guilds_read
                .iter()
                .filter_map(|(guild_id, status)| match status {
                    GuildStatus::Loading(_) => None,
                    GuildStatus::Loaded { .. } => Some(guild_id),
                })
                .cloned()
                .collect::<Vec<_>>();
            drop(guilds_read);
            let poll_results = match self.poll(&loaded_guilds).await {
                Some(results) => results,
                // Ignore failures and go to the next poll round
                None => continue,
            };

            // Acquire a write lock and merge the results of the polling into the map
            let mut guilds_write = self.guilds.write().expect("active guilds lock poisoned");
            let timestamp = architus_id::time::millisecond_ts();
            let mut rising_edge_guilds = Vec::<u64>::new();
            let mut falling_edge_guilds = Vec::<u64>::new();
            for (guild_id, is_active) in poll_results {
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

            // Emit the edge events to the uptime channel
            self.source_uptime_events(timestamp, rising_edge_guilds, falling_edge_guilds);
        }
    }

    /// Filters uptime events to ensure that they only contain active guilds
    /// that have events that are actually forwarded
    pub async fn pipe_uptime_events(
        &self,
        in_stream: impl Stream<Item = UptimeEvent>,
    ) -> Result<()> {
        // Process each item in order and do not shut down the service if it fails
        in_stream
            .for_each(|event| async {
                match event {
                    UptimeEvent::Online { guilds, timestamp } => {
                        // Grab a write lock for when we insert loading indicators (which is likely)
                        let mut guilds_write =
                            self.guilds.write().expect("active guilds lock poisoned");

                        // Load the active values for all currently-loaded guilds
                        let mut active_values = guilds
                            .iter()
                            .filter_map(|id| match guilds_write.get(id) {
                                Some(GuildStatus::Loaded(LoadedState { is_active, .. })) => {
                                    Some((*id, *is_active))
                                }
                                _ => None,
                            })
                            .collect::<HashMap<_, _>>();
                        // Load guilds that either are unloaded or loading (likely from an eager load)
                        // this is because including them in the batch is cheap
                        // and it allows us to not have to wait on the other channels
                        // Additionally, we return/shadow the write lock guard to only temporarily drop the lock
                        // if sending a request for loading guilds from the feature-gate service
                        let to_load = guilds
                            .iter()
                            .filter(|id| !active_values.contains_key(id))
                            .cloned()
                            .collect::<Vec<_>>();
                        let mut guilds_write = if to_load.is_empty() {
                            guilds_write
                        } else {
                            // Create loading status for all guilds
                            for guild_id in &to_load {
                                // Insert the loading status if it already isn't there
                                guilds_write.entry(*guild_id).or_insert_with(|| {
                                    GuildStatus::Loading(LoadingNotifier::new())
                                });
                            }

                            // Send a batched request for all guilds
                            drop(guilds_write);
                            active_values.extend(
                                self.poll(&to_load)
                                    .await
                                    .unwrap_or_else(|| {
                                        // Default to true for each guild
                                        let mut map = HashMap::<u64, bool>::new();
                                        for id in guilds {
                                            map.insert(id, true);
                                        }
                                        map
                                    })
                                    .iter(),
                            );

                            self.guilds.write().expect("active guilds lock poisoned")
                        };

                        // Signal all waiting fields
                        let mut rising_edge_guilds = Vec::<u64>::new();
                        let mut falling_edge_guilds = Vec::<u64>::new();
                        for (guild_id, is_active) in active_values {
                            guilds_write
                                .entry(guild_id)
                                .and_modify(|status| match status {
                                    GuildStatus::Loaded(state) => {
                                        // Note the result of the update,
                                        // and prepare to send guild online/offline messages if the edge changes
                                        match state.update_with_connection(
                                            is_active,
                                            GuildConnection::Online,
                                        ) {
                                            Some(ActiveEdge::Rising) => {
                                                rising_edge_guilds.push(guild_id)
                                            }
                                            Some(ActiveEdge::Falling) => {
                                                falling_edge_guilds.push(guild_id)
                                            }
                                            None => {}
                                        }
                                    }
                                    GuildStatus::Loading(notifier) => {
                                        notifier.notify(is_active);
                                        let state = LoadedState {
                                            is_active,
                                            connection: GuildConnection::Online,
                                        };
                                        if state.active() {
                                            rising_edge_guilds.push(guild_id)
                                        }
                                        *status = GuildStatus::Loaded(state);
                                    }
                                })
                                .or_insert_with(|| {
                                    rising_edge_guilds.push(guild_id);
                                    GuildStatus::Loaded(LoadedState {
                                        is_active,
                                        connection: GuildConnection::Online,
                                    })
                                });
                        }
                        drop(guilds_write);

                        // Emit the edge events to the uptime channel
                        self.source_uptime_events(
                            timestamp,
                            rising_edge_guilds,
                            falling_edge_guilds,
                        );
                    }
                    UptimeEvent::Offline { guilds, timestamp } => {
                        let mut guilds_write =
                            self.guilds.write().expect("active guilds lock poisoned");
                        let mut rising_edge_guilds = Vec::<u64>::new();
                        let mut falling_edge_guilds = Vec::<u64>::new();

                        // Update each guild status
                        for guild_id in guilds {
                            guilds_write
                                .entry(guild_id)
                                .and_modify(|status| match status {
                                    GuildStatus::Loaded(state) => {
                                        // Note the result of the update,
                                        // and prepare to send guild online/offline messages if the edge changes
                                        match state.update_connection(
                                            GuildConnection::offline(),
                                        ) {
                                            Some(ActiveEdge::Rising) => {
                                                rising_edge_guilds.push(guild_id)
                                            }
                                            Some(ActiveEdge::Falling) => {
                                                falling_edge_guilds.push(guild_id)
                                            }
                                            None => {}
                                        }
                                    }
                                    GuildStatus::Loading(_) => {
                                        log::warn!("UptimeEvent::Offline event processed for guild that was marked as loading: {}. Ignoring", guild_id);
                                    }
                                })
                                .or_insert_with(|| {
                                    log::warn!("UptimeEvent::Offline event processed for guild that was not loaded: {}", guild_id);
                                    GuildStatus::Loaded(LoadedState {
                                        is_active: false,
                                        connection: GuildConnection::offline(),
                                    })
                                });
                        }

                        // Emit the edge events to the uptime channel
                        self.source_uptime_events(
                            timestamp,
                            rising_edge_guilds,
                            falling_edge_guilds,
                        );
                    }
                    UptimeEvent::Heartbeat { guilds, timestamp } => {
                        let guilds_read =
                            self.guilds.read().expect("active guilds lock poisoned");
                        let uptime_guilds = guilds.iter().filter(|guild_id| {
                            if guilds_read.contains_key(guild_id) {
                                true
                            } else {
                                log::warn!("UptimeEvent::Heartbeat event processed for guild that was not loaded: {}", guild_id);
                                false
                            }
                        }).cloned().collect::<Vec<_>>();
                        drop(guilds_read);

                        self.emit_uptime(UptimeEvent::Heartbeat { guilds: uptime_guilds, timestamp });
                    }
                }
            })
            .await;

        Ok(())
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
                Some(GuildStatus::Loading(notifier)) => {
                    // Start waiting on the channel
                    // Note: since we require that signalers acquire the write lock
                    // before signaling/removing, this is data-race-free
                    let mut loaded_rx = notifier.subscribe();
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
                    let notifier = LoadingNotifier::new();
                    guilds_write.insert(guild_id, GuildStatus::Loading(notifier.clone()));
                    drop(guilds_write);

                    // Eagerly load the guild id and return the result
                    self.eager_load(guild_id, notifier).await
                }
            };
        }
    }

    /// Sends uptime events in the shared channel for guilds that come online/offline
    /// as a result of changes to the status of loaded guilds
    fn source_uptime_events(&self, timestamp: u64, rising_edge: Vec<u64>, falling_edge: Vec<u64>) {
        if !rising_edge.is_empty() {
            self.emit_uptime(UptimeEvent::Online {
                guilds: rising_edge,
                timestamp,
            });
        }
        if !falling_edge.is_empty() {
            self.emit_uptime(UptimeEvent::Offline {
                guilds: falling_edge,
                timestamp,
            });
        }
    }

    /// Loads the up-to-date status of all given guilds
    #[allow(clippy::await_holding_lock)]
    async fn poll(&self, guilds: &[u64]) -> Option<HashMap<u64, bool>> {
        // Chunk the active guild ids by the batch check's max size and send a request for each
        let mut results = HashMap::<u64, bool>::with_capacity(guilds.len());
        for guild_chunk in guilds.chunks(self.config.feature_gate_batch_check_size) {
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
                    log::warn!(
                        "An error occurred while sending RPC message to feature-gate service: {:?}",
                        err
                    );
                    return None;
                }
            }
        }

        Some(results)
    }

    /// Sends an uptime event on the shared channel which is forwarded to the uptime service
    fn emit_uptime(&self, event: UptimeEvent) {
        if let Err(err) = self.uptime_event_tx.send(event) {
            log::warn!(
                "An error occurred while sending uptime event to shared channel: {:?}",
                err
            );
        }
    }

    /// Sends a single-guild request to the feature-gate service and loads the result
    async fn eager_load(&self, guild_id: u64, notifier: LoadingNotifier) -> bool {
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
        let timestamp = architus_id::time::millisecond_ts();
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
        let mut guilds_write = self.guilds.write().expect("active guilds lock poisoned");
        notifier.notify(is_active);

        // There is a data race where another load could have been performed,
        // so mutate the entry in-place
        let mut rising_edge_guilds = Vec::<u64>::new();
        let mut falling_edge_guilds = Vec::<u64>::new();
        guilds_write
            .entry(guild_id)
            .and_modify(|status| {
                match status {
                    GuildStatus::Loaded(state) => {
                        // Note the result of the update,
                        // and prepare to send guild online/offline messages if the edge changes
                        match state.update(is_active) {
                            Some(ActiveEdge::Rising) => rising_edge_guilds.push(guild_id),
                            Some(ActiveEdge::Falling) => falling_edge_guilds.push(guild_id),
                            None => {}
                        }
                    }
                    _ => *status = default_status.clone(),
                };
            })
            .or_insert(default_status);

        // Emit the edge events to the uptime channel
        self.source_uptime_events(timestamp, rising_edge_guilds, falling_edge_guilds);

        is_active
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
