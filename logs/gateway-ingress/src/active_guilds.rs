use crate::config::Configuration;
use crate::rpc;
use crate::rpc::feature_gate::{BatchCheck, Client as FeatureGateClient, GuildFeature};
use slog::Logger;
use static_assertions::assert_impl_all;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Represents the cached indexing enabled/disabled status of a single guild,
/// or a guild with their indexing enabled/disabled status being loaded
#[derive(Clone, Debug)]
enum GuildStatus {
    Loaded(LoadedStatus),
    Loading,
}

/// Contains the state of a loaded guild,
/// including whether it has indexing enabled or disabled
#[derive(Clone, Debug)]
struct LoadedStatus {
    is_active: bool,
}

/// Represents a shared handler that continuously polls the feature service
/// to determine whether guilds have indexing enabled or not.
pub struct ActiveGuilds {
    config: Arc<Configuration>,
    guilds: RwLock<HashMap<u64, GuildStatus>>,
    feature_gate_client: FeatureGateClient,
    logger: Logger,
}

assert_impl_all!(Arc<ActiveGuilds>: Send, Sync);

impl ActiveGuilds {
    /// Creates a new shared handler and wraps the connection to the feature gate service.
    pub fn new(
        feature_gate_client: FeatureGateClient,
        config: Arc<Configuration>,
        logger: Logger,
    ) -> Self {
        Self {
            config,
            guilds: RwLock::new(HashMap::new()),
            feature_gate_client: feature_gate_client,
            logger,
        }
    }

    /// Runs a task that continuously polls the feature gate to maintain an active list of guilds
    /// that have log indexing enabled
    pub async fn go_poll(&self) {
        loop {
            tokio::time::sleep(self.config.active_guilds_poll_interval).await;
            slog::info!(
                self.logger,
                "polling for the set of guilds with indexing enabled"
            );

            // Get a list of all active guilds by their guild ids (only include loaded ones;
            // loading ones will be eagerly loaded faster than we can bulk-fetch them anyways)
            let loaded_guild_ids = self.get_loaded_guilds();
            slog::debug!(self.logger, "currently loaded guilds"; "loaded_guilds" => ?loaded_guild_ids);

            // Poll for the updated status from the feature gate service
            let poll_results = match self.try_poll(&loaded_guild_ids).await {
                Ok(results) => results,
                Err(err) => {
                    slog::warn!(
                        self.logger,
                        "failed to poll for the updated set of guilds with indexing enabled";
                        "error" => ?err,
                        "loaded_guilds" => ?loaded_guild_ids,
                    );
                    continue;
                }
            };

            // Store all poll results (we never evict guilds;
            // but this is probably okay since a u64 & boolean are very small to store anyways)
            self.update_guilds(&poll_results);

            slog::info!(
                self.logger,
                "loaded updated set of guilds with indexing enabled";
                "active_guilds_count" => poll_results.values().filter(|b| **b).count(),
                "polling_again_in" => ?self.config.active_guilds_poll_interval,
            );
        }
    }

    /// Loads the up-to-date status of all given guilds
    async fn try_poll(&self, guilds: &[u64]) -> Result<HashMap<u64, bool>, tonic::Status> {
        // Chunk the active guild ids by the batch check's max size and send a request for each
        let mut results = HashMap::<u64, bool>::with_capacity(guilds.len());
        for guild_chunk in guilds.chunks(self.config.feature_gate_batch_check_size) {
            let send = || async {
                // Cloning the tonic gRPC client is cheap; it is internally ref-counted
                let mut feature_gate_client = self.feature_gate_client.clone();

                let result = feature_gate_client
                    .batch_check_guild_features(BatchCheck {
                        feature_name: self.config.indexing_feature.clone(),
                        guild_ids: Vec::from(guild_chunk),
                    })
                    .await;
                rpc::into_backoff(result)
            };

            let result = backoff::future::retry(self.config.rpc_backoff.build(), send).await;
            match result {
                Ok(batch_result) => {
                    if batch_result.has_feature.len() != guild_chunk.len() {
                        slog::warn!(
                            self.logger,
                            "feature-gate service returned batch result with different length than input";
                            "expected_length" => guild_chunk.len(),
                            "actual_length" => batch_result.has_feature.len(),
                        );
                        // Ignore and move to the next chunk
                        continue;
                    }
                    for (guild_id, result) in guild_chunk.iter().zip(batch_result.has_feature) {
                        results.insert(*guild_id, result);
                    }
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }

        Ok(results)
    }

    pub fn get_loaded_guilds(&self) -> Vec<u64> {
        let guilds_read = self.guilds.read().expect("active guilds lock poisoned");
        guilds_read
            .iter()
            .filter_map(|(guild_id, status)| match status {
                GuildStatus::Loading => None,
                GuildStatus::Loaded { .. } => Some(guild_id),
            })
            .copied()
            .collect::<Vec<_>>()
    }

    pub fn update_guilds(&self, updates: &HashMap<u64, bool>) {
        let mut guilds_write = self.guilds.write().expect("active guilds lock poisoned");
        for (guild_id, is_active) in updates {
            guilds_write.insert(
                *guild_id,
                GuildStatus::Loaded(LoadedStatus {
                    is_active: *is_active,
                }),
            );
        }
    }

    /// Determines whether the given `guild_id` should have events forwarded to the queue
    /// If not tracked, then asynchronously loads this guild.
    /// This requires `Arc<Self>` as a method target since it needs to spawn a background task
    /// that is guaranteed to finish, whether or not the future returned by this function
    /// is polled to completion.
    pub async fn is_active(self: Arc<Self>, guild_id: u64) -> bool {
        // Provide the opportunity to try again
        // if a non-connection error or data race is encountered,
        // or if we are waiting for a loading status to get loaded by another task.
        loop {
            // Use an explicit lexical scope to ensure the read lock handle
            // has been dropped before we sleep.
            {
                let guilds_read = self.guilds.read().expect("active guilds lock poisoned");
                match guilds_read.get(&guild_id) {
                    Some(GuildStatus::Loaded(LoadedStatus { is_active })) => {
                        return *is_active;
                    }
                    Some(GuildStatus::Loading) => {
                        // Continue in the sleep-poll loop
                        // until the background eager load task has finished
                    }
                    None => {
                        // Note: since we can't upgrade our read lock in-place,
                        // we have to drop and re-acquire it.
                        // Because this is not atomic, there is a possible data race
                        // where the guild has a status once we re-acquire the write lock.
                        // This means we need to check again before spawning the eager load task.
                        drop(guilds_read);
                        let mut guilds_write =
                            self.guilds.write().expect("active guilds lock poisoned");
                        if guilds_write.get(&guild_id).is_some() {
                            continue;
                        }

                        // Mark the guild as loading
                        guilds_write.insert(guild_id, GuildStatus::Loading);
                        drop(guilds_write);

                        // Spawn the loading operation in the background
                        // to ensure that it runs o completion
                        let self_clone = Arc::clone(&self);
                        let join_handle =
                            tokio::spawn(async move { self_clone.eager_load(guild_id).await });

                        // The `eager_load` method returns the result,
                        // so we can directly wait for it here
                        return join_handle
                            .await
                            .expect("the eager load background operation panicked");
                    }
                };
            }

            // Another task is loading the status for this guild,
            // just sleep for some interval
            tokio::time::sleep(self.config.is_active_loading_poll_interval).await;
        }
    }

    /// Sends a single-guild request to the feature-gate service and loads the result.
    /// This function needs to be polled to completion.
    async fn eager_load(&self, guild_id: u64) -> bool {
        // Fetch the guild from the feature server using a backoff
        let send = || async {
            // Cloning the tonic gRPC client is cheap; it is internally ref-counted
            let mut feature_gate_client = self.feature_gate_client.clone();

            let result = feature_gate_client
                .check_guild_feature(GuildFeature {
                    feature_name: self.config.indexing_feature.clone(),
                    guild_id,
                })
                .await;
            rpc::into_backoff(result)
        };

        let result = backoff::future::retry(self.config.rpc_backoff.build(), send).await;
        let is_active = match result {
            Ok(response) => response.has_feature,
            Err(err) => {
                slog::warn!(
                    self.logger,
                    "could not contact the feature-gate service for information about indexing on guild";
                    "guild_id" => guild_id,
                    "error" => ?err,
                );
                // Default to true if the feature gate cannot be contacted
                // for graceful degradation
                true
            }
        };

        // Update the state by acquiring a write lock
        let mut guilds_write = self.guilds.write().expect("active guilds lock poisoned");
        guilds_write.insert(guild_id, GuildStatus::Loaded(LoadedStatus { is_active }));

        is_active
    }
}
