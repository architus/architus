use crate::config::Configuration;
use crate::uptime::debounced_pool::{DebouncedPool, Update as DebouncedPoolUpdate};
use crate::uptime::{Event as UptimeEvent, UpdateMessage};
use futures::{stream, Stream, StreamExt as _1};
use static_assertions::assert_impl_all;
use std::sync::{Arc, Mutex};
use tokio::stream::StreamExt as _2;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

/// Represents a guild-level connection tracking handler for the ingress service,
/// deriving a stateful status of the connection to each external service
/// and using that to inform heartbeat/online/offline events eventually sent
/// to an uptime tracking service.
/// This information is then used to schedule batch indexing jobs.
///
/// When creating, gives a multi-producer channel that can be used to send updates
/// to the uptime tracking handler
pub struct Tracker {
    updates: UnboundedReceiver<UpdateMessage>,
    debounced_guild_updates: UnboundedReceiver<DebouncedPoolUpdate<u64>>,
    state: TrackerState,
}

impl Tracker {
    /// Creates a new tracker from the configuration,
    /// also giving a multi-producer clone-able channel that can be used to send updates
    pub fn new(config: Arc<Configuration>) -> (Self, UnboundedSender<UpdateMessage>) {
        let (update_sender, update_receiver) = mpsc::unbounded_channel::<UpdateMessage>();
        let (active_guilds, debounced_guild_updates) =
            DebouncedPool::new(config.guild_uptime_debounce_delay);
        let new_tracker = Self {
            updates: update_receiver,
            debounced_guild_updates,
            state: TrackerState {
                config,
                active_guilds,
                connection_status: Arc::new(Mutex::new(ConnectionStatus::new())),
            },
        };
        (new_tracker, update_sender)
    }

    /// Listen for incoming updates and use them to update the internal state.
    /// Emits outgoing uptime events to be eventually forwarded to the uptime service
    pub fn stream_events(self) -> impl Stream<Item = UptimeEvent> {
        let uptime_events = self.state.pipe_updates(self.updates);
        let debounced_uptime_events = self
            .state
            .pipe_debounced_guild_updates(self.debounced_guild_updates);

        // Emit the result of merging both streams
        uptime_events.merge(debounced_uptime_events)
    }
}

/// Shared tracker state that is used to coordinate state while a tracker runs
#[derive(Clone)]
struct TrackerState {
    config: Arc<Configuration>,
    active_guilds: DebouncedPool<u64>,
    connection_status: Arc<Mutex<ConnectionStatus>>,
}

assert_impl_all!(TrackerState: Sync, Send);

impl TrackerState {
    /// Stream processor that uses the stateful tracking information
    /// to generate the uptime events from the individual updates
    fn pipe_updates(
        &self,
        in_stream: impl Stream<Item = UpdateMessage>,
    ) -> impl Stream<Item = UptimeEvent> {
        let pool_copy = self.active_guilds.clone();
        let connection_status_mutex = Arc::clone(&self.connection_status);
        in_stream.flat_map(move |update| {
            // Note the timestamp that this was received,
            // (ignores the propagation delay from source to here,
            // but since this processor is synchronous,
            // this should be negligible and provides a more ergonomic upstream API)
            let timestamp = architus_id::time::millisecond_ts();
            match update {
                // For guild online/offline,
                // instead of emitting an event right now,
                // use the debounced pool and emit nothing
                UpdateMessage::GuildOnline(guild_id) => {
                    pool_copy.add(guild_id);
                    stream::iter(Vec::with_capacity(0))
                }
                UpdateMessage::GuildOffline(guild_id) => {
                    pool_copy.remove(guild_id);
                    stream::iter(Vec::with_capacity(0))
                }
                UpdateMessage::QueueOnline | UpdateMessage::GatewayOnline => {
                    let mut connection_status = connection_status_mutex
                        .lock()
                        .expect("connection status poisoned");
                    // Only emit an uptime event if the entire service just became online
                    let events = if connection_status.online_update(&update) {
                        pool_copy.release();
                        let items = pool_copy.items::<Vec<_>>();
                        let events = vec![UptimeEvent::Online {
                            guilds: items,
                            timestamp,
                        }];
                        events
                    } else {
                        Vec::with_capacity(0)
                    };
                    stream::iter(events)
                }
                UpdateMessage::QueueOffline | UpdateMessage::GatewayOffline => {
                    let mut connection_status = connection_status_mutex
                        .lock()
                        .expect("connection status poisoned");
                    // Only emit an uptime event if the entire service just became offline
                    let events = if connection_status.offline_update(&update) {
                        let items = pool_copy.items::<Vec<_>>();
                        let events = vec![UptimeEvent::Offline {
                            guilds: items,
                            timestamp,
                        }];
                        pool_copy.release();
                        events
                    } else {
                        Vec::with_capacity(0)
                    };
                    stream::iter(events)
                }
                UpdateMessage::GatewayHeartbeat => {
                    let connection_status = connection_status_mutex
                        .lock()
                        .expect("connection status poisoned");
                    let events = if connection_status.online() {
                        let mut events = pool_copy
                            .release()
                            .map_or_else(Vec::new, pool_update_to_uptime);
                        let items = pool_copy.items();
                        events.push(UptimeEvent::Heartbeat {
                            guilds: items,
                            timestamp,
                        });
                        events
                    } else {
                        Vec::with_capacity(0)
                    };
                    stream::iter(events)
                }
            }
        })
    }

    /// Acts as a stream processor for the debounced bulk guild updates from the debounced pool,
    /// converting them into uptime events if the connection is online
    fn pipe_debounced_guild_updates(
        &self,
        in_stream: impl Stream<Item = DebouncedPoolUpdate<u64>>,
    ) -> impl Stream<Item = UptimeEvent> {
        let connection_status_mutex = Arc::clone(&self.connection_status);
        in_stream.flat_map(move |update| {
            let connection_status = connection_status_mutex
                .lock()
                .expect("connection status poisoned");
            let events = if connection_status.online() {
                pool_update_to_uptime(update)
            } else {
                Vec::with_capacity(0)
            };
            stream::iter(events)
        })
    }
}

fn pool_update_to_uptime(update: DebouncedPoolUpdate<u64>) -> Vec<UptimeEvent> {
    // Use the timestamp that the debounced pool update is processed
    // since it can contain any backing items that happened at any point
    // from t_now to t_now - debounced_delay
    let timestamp = architus_id::time::millisecond_ts();

    let mut updates = Vec::new();
    if let Some(added) = update.added {
        updates.push(UptimeEvent::Online {
            guilds: added,
            timestamp,
        });
    }
    if let Some(removed) = update.removed {
        updates.push(UptimeEvent::Offline {
            guilds: removed,
            timestamp,
        });
    }

    updates
}

/// Holds the connection state to the gateway and queue
/// and utility methods to capture the rising/falling edges of overall connection
struct ConnectionStatus {
    gateway_online: bool,
    queue_online: bool,
}

impl ConnectionStatus {
    const fn new() -> Self {
        Self {
            gateway_online: false,
            queue_online: false,
        }
    }

    const fn online(&self) -> bool {
        self.gateway_online && self.queue_online
    }

    fn online_update(&mut self, update: &UpdateMessage) -> bool {
        let offline_before = !self.online();
        match update {
            UpdateMessage::QueueOnline => self.queue_online = true,
            UpdateMessage::GatewayOnline => self.gateway_online = true,
            _ => {}
        }
        let online_after = self.online();
        offline_before && online_after
    }

    fn offline_update(&mut self, update: &UpdateMessage) -> bool {
        let online_before = self.online();
        match update {
            UpdateMessage::QueueOffline => self.queue_online = false,
            UpdateMessage::GatewayOffline => self.gateway_online = false,
            _ => {}
        }
        let offline_after = !self.online();
        online_before && offline_after
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Configuration;
    use crate::uptime::connection::{Tracker, UpdateMessage};
    use crate::uptime::UptimeEvent;
    use anyhow::Result;
    use futures::StreamExt;
    use std::collections::HashSet;
    use std::hash::Hash;
    use std::iter::FromIterator;
    use std::sync::Arc;
    use std::time::Duration;

    /// Defines set-equality for uptime events
    #[derive(Debug, Clone)]
    struct TestWrapper(UptimeEvent);
    impl PartialEq for TestWrapper {
        fn eq(&self, other: &Self) -> bool {
            match (&self.0, &other.0) {
                (UptimeEvent::Online { guilds: a, .. }, UptimeEvent::Online { guilds: b, .. })
                | (
                    UptimeEvent::Offline { guilds: a, .. },
                    UptimeEvent::Offline { guilds: b, .. },
                )
                | (
                    UptimeEvent::Heartbeat { guilds: a, .. },
                    UptimeEvent::Heartbeat { guilds: b, .. },
                ) => set(a) == set(b),
                _ => false,
            }
        }
    }

    fn set<T: Hash + Eq + Clone>(v: &Vec<T>) -> HashSet<T> {
        HashSet::<T>::from_iter(v.iter().cloned())
    }

    #[tokio::test]
    async fn test_basic_debounced() -> Result<()> {
        let mut config = Configuration::default();
        config.guild_uptime_debounce_delay = Duration::from_millis(25);
        let (tracker, update_tx) = Tracker::new(Arc::new(config));
        // Apply full back-pressure to process events immediately
        let (events_tx, mut events_rx) = tokio::sync::mpsc::unbounded_channel::<UptimeEvent>();
        tokio::spawn(async move {
            // Apply full back-pressure to process events immediately
            let mut stream = tracker.stream_events();
            while let Some(event) = stream.next().await {
                events_tx.send(event).unwrap();
            }
        });

        // Note: timestamp is ignored when asserting equality
        update_tx.send(UpdateMessage::GuildOnline(0))?;
        update_tx.send(UpdateMessage::GuildOnline(1))?;
        update_tx.send(UpdateMessage::GuildOnline(2))?;
        tokio::time::delay_for(Duration::from_millis(50)).await;

        assert_eq!(
            events_rx.next().await.map(TestWrapper),
            Some(TestWrapper(UptimeEvent::Online {
                guilds: vec![0, 1, 2],
                timestamp: 0
            }))
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_heartbeat_flush() -> Result<()> {
        let mut config = Configuration::default();
        config.guild_uptime_debounce_delay = Duration::from_millis(25);
        let (tracker, update_tx) = Tracker::new(Arc::new(config));
        // Apply full back-pressure to process events immediately
        let (events_tx, mut events_rx) = tokio::sync::mpsc::unbounded_channel::<UptimeEvent>();
        tokio::spawn(async move {
            // Apply full back-pressure to process events immediately
            let mut stream = tracker.stream_events();
            while let Some(event) = stream.next().await {
                events_tx.send(event).unwrap();
            }
        });

        // Note: timestamp is ignored when asserting equality
        update_tx.send(UpdateMessage::GuildOnline(0))?;
        update_tx.send(UpdateMessage::GuildOnline(1))?;
        tokio::time::delay_for(Duration::from_millis(50)).await;
        update_tx.send(UpdateMessage::GuildOnline(2))?;
        update_tx.send(UpdateMessage::GuildOffline(0))?;
        update_tx.send(UpdateMessage::GatewayHeartbeat)?;

        assert_eq!(
            events_rx.next().await.map(TestWrapper),
            Some(TestWrapper(UptimeEvent::Online {
                guilds: vec![0, 1],
                timestamp: 0
            }))
        );
        assert_eq!(
            events_rx.next().await.map(TestWrapper),
            Some(TestWrapper(UptimeEvent::Online {
                guilds: vec![2],
                timestamp: 0
            }))
        );
        assert_eq!(
            events_rx.next().await.map(TestWrapper),
            Some(TestWrapper(UptimeEvent::Offline {
                guilds: vec![0],
                timestamp: 0
            }))
        );
        assert_eq!(
            events_rx.next().await.map(TestWrapper),
            Some(TestWrapper(UptimeEvent::Heartbeat {
                guilds: vec![1, 2],
                timestamp: 0
            }))
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_offline_online() -> Result<()> {
        let mut config = Configuration::default();
        config.guild_uptime_debounce_delay = Duration::from_millis(25);
        let (tracker, update_tx) = Tracker::new(Arc::new(config));
        let (events_tx, mut events_rx) = tokio::sync::mpsc::unbounded_channel::<UptimeEvent>();
        tokio::spawn(async move {
            // Apply full back-pressure to process events immediately
            let mut stream = tracker.stream_events();
            while let Some(event) = stream.next().await {
                events_tx.send(event).unwrap();
            }
        });

        // Note: timestamp is ignored when asserting equality
        update_tx.send(UpdateMessage::GuildOnline(0))?;
        update_tx.send(UpdateMessage::GuildOnline(1))?;
        tokio::time::delay_for(Duration::from_millis(50)).await;
        update_tx.send(UpdateMessage::GatewayOffline)?;
        update_tx.send(UpdateMessage::QueueOffline)?;
        update_tx.send(UpdateMessage::QueueOnline)?;
        update_tx.send(UpdateMessage::GatewayOnline)?;

        assert_eq!(
            events_rx.next().await.map(TestWrapper),
            Some(TestWrapper(UptimeEvent::Online {
                guilds: vec![0, 1],
                timestamp: 0
            }))
        );
        assert_eq!(
            events_rx.next().await.map(TestWrapper),
            Some(TestWrapper(UptimeEvent::Offline {
                guilds: vec![0, 1],
                timestamp: 0
            }))
        );
        assert_eq!(
            events_rx.next().await.map(TestWrapper),
            Some(TestWrapper(UptimeEvent::Online {
                guilds: vec![0, 1],
                timestamp: 0
            }))
        );

        Ok(())
    }
}
