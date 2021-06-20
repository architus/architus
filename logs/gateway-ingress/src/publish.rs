//! Handles the core publishing logic for taking raw gateway events
//! and sending them to RabbitMQ

use crate::config::Configuration;
use crate::rpc::gateway_queue_lib::GatewayEvent;
use crate::uptime::active_guilds::ActiveGuilds;
use crate::uptime::UpdateMessage;
use anyhow::{anyhow, Context};
use architus_amqp_pool::{Manager, Pool, PoolError};
use architus_id::{time, HoarFrost, IdProvisioner};
use backoff::backoff::Backoff;
use deadpool::Runtime;
use futures::{join, try_join, FutureExt, Stream, StreamExt};
use lapin::options::{BasicPublishOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection};
use lazy_static::lazy_static;
use prost::Message;
use slog::Logger;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use twilight_gateway::{Event, EventType, Intents};
use twilight_model::gateway::event::gateway::GatewayEventDeserializer;
use twilight_model::gateway::event::shard::Payload;
use twilight_model::gateway::OpCode;

/// Shape of the message passed to the reconnection background coroutine
/// to request that the handle factory be re-initialized with a valid connection.
struct ReconnectRequest {
    ready_tx: broadcast::Sender<()>,
    connection: Option<Connection>,
    next_generation: usize,
}

/// Wraps the behavior of consuming events from the gateway
/// and publishing them to RabbitMQ.
/// Handles a backpressure-enabled queue in the middle that will report
/// if it reaches a certain threshold of items that still haven't been processed.
/// Ensures that temporary RabbitMQ availability errors
/// do not result in a loss of raw events.
pub struct Publisher {
    active_guilds: ActiveGuilds,
    config: Arc<Configuration>,
    logger: Logger,
    id_provisioner: IdProvisioner,
    handle_factory: Arc<HandleFactory>,
    reconnect_rx: Option<mpsc::UnboundedReceiver<ReconnectRequest>>,
}

impl Publisher {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(
        queue_connection: Connection,
        active_guilds: ActiveGuilds,
        config: Arc<Configuration>,
        logger: Logger,
        update_tx: mpsc::UnboundedSender<UpdateMessage>,
    ) -> Self {
        let logger = logger.new(slog::o!("max_queue_length" => config.raw_events.queue_length));
        let (reconnect_tx, reconnect_rx) = mpsc::unbounded_channel::<ReconnectRequest>();
        Self {
            active_guilds,
            config: Arc::clone(&config),
            logger: logger.clone(),
            id_provisioner: IdProvisioner::new(Some(logger.clone())),
            handle_factory: Arc::new(HandleFactory::new(
                queue_connection,
                update_tx,
                logger,
                config,
                reconnect_tx,
            )),
            reconnect_rx: Some(reconnect_rx),
        }
    }

    /// Runs the publisher forever,
    /// which handles reporting when the queue is reaching its capacity.
    /// The internal `handle_factory` handles reconnecting to the queue when needed
    /// and ensuring that all enqueued messages (up to the channel's capacity)
    /// eventually get published even if transient errors occur reaching RabbitMQ.
    pub async fn consume_events(
        mut self,
        raw_event_stream: impl Stream<Item = Event>,
    ) -> Result<(), anyhow::Error> {
        let reconnect_rx = self.reconnect_rx.take().unwrap();

        // Create a backpressure-capable stream factory
        // so that raw events piling up is caught and reported on
        let (event_tx, event_rx) = mpsc::channel(self.config.raw_events.queue_length);
        let queue_future = async {
            let pipe_future = self.pipe_events_to_bounded_queue(raw_event_stream, event_tx.clone());
            let watch_future = self
                .watch_queue_length(|| self.config.raw_events.queue_length - event_tx.capacity());
            join!(pipe_future, watch_future);
        };

        // Consume each event from the stream in parallel
        let consume_future = ReceiverStream::new(event_rx).for_each_concurrent(
            Some(self.config.raw_events.publish_concurrency),
            |event| async { self.publish_event(event).await },
        );

        // Reconnect to the RabbitMQ instance in the background if it ever fails
        let reconnection_future = self.handle_factory.reconnect_background_loop(reconnect_rx);

        try_join!(
            queue_future.map(|_| Ok::<(), anyhow::Error>(())),
            consume_future.map(|_| Ok::<(), anyhow::Error>(())),
            reconnection_future
        )?;
        Ok(())
    }

    /// Performs the consumption on the raw event stream,
    /// re-sending them to the bounded queue
    /// or dropping them if the queue is full.
    async fn pipe_events_to_bounded_queue(
        &self,
        raw_event_stream: impl Stream<Item = Event>,
        event_tx: mpsc::Sender<Event>,
    ) {
        raw_event_stream
            .for_each(|event| async {
                if let Err(send_err) = event_tx.try_send(event) {
                    slog::warn!(
                        self.logger,
                        "sending raw gateway event into shared channel failed; dropping";
                        "error" => ?send_err
                    );
                }
            })
            .await;
    }

    /// Asynchronously watches the queue length to ensure
    /// that it doesn't exceed a warning threshold.
    /// This is useful for reporting before the queue starts dropping events.
    async fn watch_queue_length(&self, check_length: impl Fn() -> usize) {
        let mut interval = tokio::time::interval(self.config.raw_events.watch_period);
        loop {
            interval.tick().await;
            let current_length = check_length();
            if current_length > self.config.raw_events.warn_threshold {
                slog::warn!(
                    self.logger,
                    "current queue length exceeds warning threshold";
                    "current_queue_length" => current_length,
                    "warn_threshold" => self.config.raw_events.warn_threshold
                );
            }
        }
    }

    /// Publishes a single event to the RabbitMQ message queue.
    /// Blocks until the event can be successfully published,
    /// potentially indefinitely.
    /// This can cause events to start being dropped on the bounded queue.
    async fn publish_event(&self, event: Event) {
        // Provision an ID and note the timestamp immediately
        let timestamp = time::millisecond_ts();
        let id = self.id_provisioner.with_ts(timestamp);
        let logger = self.logger.new(slog::o!(
            "event_timestamp" => timestamp,
            "event_id" => id,
        ));

        // Create the `GatewayEvent` from the raw event
        let gateway_event =
            if let Some(gateway_event) = convert_raw_event(event, timestamp, id, &logger) {
                gateway_event
            } else {
                return;
            };

        // Serialize the event to raw bytes
        let mut raw_bytes = Vec::with_capacity(gateway_event.encoded_len());
        if let Err(encode_err) = gateway_event.encode(&mut raw_bytes) {
            slog::warn!(
                logger,
                "could not encode gateway event to bytes";
                "error" => ?encode_err,
                "event" => ?gateway_event
            );
            return;
        }

        // Make sure the guild is active before forwarding
        if !self.active_guilds.is_active(gateway_event.guild_id).await {
            return;
        }

        loop {
            // Continuously try to publish the event to RabbitMQ,
            // notifying the handle factory if there is an error.
            // This lets the queue get reconnected to if there are transient errors.
            // The initial call to self.handle_factory.acquire() may block
            // until the queue is ready to use
            let publisher_handle = Arc::clone(&self.handle_factory)
                .acquire(raw_bytes.clone(), logger.clone())
                .await;
            if let Err(publish_err) = publisher_handle.try_publish().await {
                Arc::clone(&self.handle_factory).notify_error(publish_err);
            } else {
                return;
            }
        }
    }
}

/// Used as a token to report an error back to the handle factory
/// so that it can reconnect to RabbitMQ once an error occurs.
#[derive(Debug)]
struct Error {
    logger: Logger,
    generation: usize,
    inner: anyhow::Error,
}

/// Internal state for `HandleFactory`
enum HandleFactoryState {
    Connecting {
        ready_tx: broadcast::Sender<()>,
        // Keep an active Receiver in the struct so that sending doesn't fail
        _ready_rx: broadcast::Receiver<()>,
    },
    Connected {
        pool: Arc<Pool>,
        generation: usize,
    },
}

/// Handles reconnection logic for the message queue,
/// and issuing of Handles for event publish attempts
struct HandleFactory {
    update_tx: mpsc::UnboundedSender<UpdateMessage>,
    logger: Logger,
    config: Arc<Configuration>,
    state: Mutex<HandleFactoryState>,
    reconnect_tx: mpsc::UnboundedSender<ReconnectRequest>,
}

impl HandleFactory {
    fn new(
        queue_connection: Connection,
        update_tx: mpsc::UnboundedSender<UpdateMessage>,
        logger: Logger,
        config: Arc<Configuration>,
        reconnect_tx: mpsc::UnboundedSender<ReconnectRequest>,
    ) -> Self {
        // Set up the state of the factory to be connecting
        // and send the request to (re)connect to the background coroutine.
        let (ready_tx, ready_rx) = broadcast::channel::<()>(1);
        let request = ReconnectRequest {
            ready_tx: ready_tx.clone(),
            connection: Some(queue_connection),
            next_generation: 0,
        };
        if reconnect_tx.send(request).is_err() {
            // This should never fail; fail fast
            panic!("could not send initial ReconnectRequest: Receiver dropped");
        }

        Self {
            update_tx,
            logger,
            config,
            state: Mutex::new(HandleFactoryState::Connecting {
                ready_tx,
                _ready_rx: ready_rx,
            }),
            reconnect_tx,
        }
    }

    /// Attempts to acquire an active handle to publish an event.
    /// Blocks until the internal state is marked as Connected
    /// and the RabbitMQ connection pool can be obtained.
    // Ignore the clippy warning for holding the lock through await.
    // We manually drop it so that this isn't a problem.
    #[allow(clippy::await_holding_lock)]
    async fn acquire(&self, serialized: Vec<u8>, logger: Logger) -> Handle {
        // Loop over this to handle the race condition that the ready channel is signaled
        // but then the state is marked as Connecting again
        // before this function observes the Connected state value
        loop {
            let state_handle = self
                .state
                .lock()
                .expect("HandleFactory.state Mutex poisoned");
            let mut ready_rx = match &*state_handle {
                HandleFactoryState::Connected { pool, generation } => {
                    return Handle::new(
                        serialized,
                        logger,
                        Arc::clone(&self.config),
                        Arc::clone(pool),
                        *generation,
                    );
                }
                HandleFactoryState::Connecting { ready_tx, .. } => ready_tx.subscribe(),
            };

            // Make sure to drop the lock before waiting on the ready channel
            std::mem::drop(state_handle);
            if let Err(err) = ready_rx.recv().await {
                slog::warn!(
                    logger,
                    "waiting for ready channel during HandleFactory::acquire failed";
                    "error" => ?err
                );
            };
        }
    }

    /// Runs a loop in the background that takes reconnect requests on the channel
    /// and handles the backoff loop.
    /// Returns an error if their is a fatal error.
    /// Note that not being able to connect is not considered a fatal error,
    /// because we don't want to lose queued events if at all possible.
    async fn reconnect_background_loop(
        &self,
        mut reconnect_rx: mpsc::UnboundedReceiver<ReconnectRequest>,
    ) -> Result<(), anyhow::Error> {
        loop {
            // Get the next request from the shared channel
            let request = if let Some(next_request) = reconnect_rx.recv().await {
                next_request
            } else {
                // This shouldn't be possible; fail fast
                slog::error!(self.logger, "reconnect request channel was closed");
                panic!();
            };

            let ReconnectRequest {
                ready_tx,
                connection: mut existing_connection,
                next_generation,
            } = request;

            // Reconnect to the Rabbit MQ instance if needed
            let connection = if let Some(connection) = existing_connection.take() {
                connection
            } else {
                // Continuously attempt to make the connection
                let connection = self.reconnect().await;

                // Notify the uptime tracker of the queue coming online
                self.update_tx
                    .send(UpdateMessage::QueueOnline)
                    .context("could not send queue online update to uptime tracker")?;
                connection
            };

            // Declare the RMQ queue to publish incoming events to
            declare_event_queue(&connection, Arc::clone(&self.config), self.logger.clone()).await?;

            // Create a pool for the RMQ channels
            let manager = Manager::new(connection);
            let mut pool_config = self.config.gateway_queue.connection_pool.clone();
            pool_config.runtime = Runtime::Tokio1;
            let channel_pool = Pool::from_config(manager, pool_config);

            // Store the connection pool in the handle factory
            // and notify all watchers that it is ready.
            let mut state_handle = self
                .state
                .lock()
                .expect("HandleFactory.state Mutex poisoned");
            match &mut *state_handle {
                HandleFactoryState::Connected { generation, .. } => {
                    // This shouldn't be possible; just log and ignore
                    let logger = self.logger.new(slog::o!(
                        "planned_next_generation" => next_generation,
                        "current_generation" => *generation,
                    ));
                    slog::warn!(
                        logger,
                        "invariant violated: HandleFactoryState marked as connected before background reconnect loop ready to signal reconnect";
                    );

                    // Sending is OK even with the lock is acquired,
                    // because listeners will re-acquire the lock before returning successfully
                    if let Err(send_err) = ready_tx.send(()) {
                        slog::warn!(
                            logger,
                            "could not send ready message to listeners; all receivers dropped";
                            "error" => ?send_err,
                        );
                    }
                }
                HandleFactoryState::Connecting { .. } => {
                    let logger = self
                        .logger
                        .new(slog::o!("next_generation" => next_generation));
                    slog::info!(
                        logger,
                        "reconnected to RabbitMQ; bumping generation and notifying listeners"
                    );

                    // Update the state
                    *state_handle = HandleFactoryState::Connected {
                        pool: Arc::new(channel_pool),
                        generation: next_generation,
                    };

                    // Sending is OK even with the lock is acquired,
                    // because listeners will re-acquire the lock before returning successfully
                    if let Err(send_err) = ready_tx.send(()) {
                        slog::warn!(
                            logger,
                            "could not send ready message to listeners; all receivers dropped";
                            "error" => ?send_err,
                        );
                    }
                }
            }
        }
    }

    /// Repeatedly attempts to reconnect to the message queue.
    /// There is no limit on the number of re-attempts
    /// because we want to avoid dropping gateway events if at all possible.
    async fn reconnect(&self) -> Connection {
        let mut backoff_config = self.config.reconnection_backoff.clone();
        // Set the duration for 10 years to effectively be forever
        backoff_config.duration = Duration::from_secs(60 * 60 * 24 * 365 * 10);
        let mut backoff = backoff_config.build();
        loop {
            // Sleep for the next backoff interval
            let backoff_time = backoff.next_backoff();
            if let Some(time) = backoff_time {
                tokio::time::sleep(time).await;
            } else {
                // This shouldn't be possible
                slog::warn!(
                    self.logger,
                    "reconnect backoff elapsed despite 10 year duration; resetting it"
                );
                backoff.reset();
                continue;
            }

            // Perform the connection attempt
            let connect_future = crate::connect::to_queue_attempt(Arc::clone(&self.config));
            match connect_future.await {
                Ok(connection) => return connection,
                Err(err) => {
                    slog::warn!(
                        self.logger,
                        "reconnecting to RabbitMQ failed; retrying after delay";
                        "error" => ?err,
                    );
                }
            }
        }
    }

    /// Notifies the handle factory that an error occurred.
    /// If the generation of the error is current,
    /// then the state of the factory is marked as Connecting
    /// and the reconnect background coroutine is signalled
    /// to attempt to reconnect to the message queue.
    #[allow(clippy::needless_pass_by_value)]
    fn notify_error(&self, error: Error) {
        slog::warn!(
            error.logger,
            "publishing event failed; planning to retry";
            "error" => ?error.inner,
            "generation" => error.generation,
        );

        // If the state is already Connecting or the generation of the error is old,
        // then ignore.
        let mut state_handle = self
            .state
            .lock()
            .expect("HandleFactory.state Mutex poisoned");
        match &mut *state_handle {
            HandleFactoryState::Connecting { .. } => {}
            HandleFactoryState::Connected { generation, .. } => {
                if *generation > error.generation {
                    return;
                }

                // Send the request to reconnect to the background coroutine
                // This is OK to do with the lock held,
                // since the background coroutine will acquire the lock
                // once signalled before proceeding.
                let (ready_tx, ready_rx) = broadcast::channel::<()>(1);
                let request = ReconnectRequest {
                    ready_tx: ready_tx.clone(),
                    connection: None,
                    next_generation: generation.saturating_add(1),
                };
                if self.reconnect_tx.send(request).is_err() {
                    // This should never fail; fail fast
                    panic!("could not send ReconnectRequest: Receiver dropped");
                }

                // Mark the state of the factory itself as Connecting
                *state_handle = HandleFactoryState::Connecting {
                    ready_tx,
                    _ready_rx: ready_rx,
                };

                std::mem::drop(state_handle);

                // Notify the uptime tracker that the queue is offline
                if let Err(send_err) = self.update_tx.send(UpdateMessage::QueueOffline) {
                    slog::error!(
                        error.logger,
                        "could not send QueueOffline message to uptime tracker";
                        "error" => ?send_err,
                        "generation" => error.generation,
                    );
                }
            }
        };
    }
}

/// Represents a single attempt to publish an event to RabbitMQ.
/// If it fails, then it creates an instance of `Error` that includes the generation
/// so that the queue can be reconnected to and the publishes retried.
struct Handle {
    serialized: Vec<u8>,
    logger: Logger,
    config: Arc<Configuration>,
    channel_pool: Arc<Pool>,
    generation: usize,
}

impl Handle {
    fn new(
        serialized: Vec<u8>,
        logger: Logger,
        config: Arc<Configuration>,
        channel_pool: Arc<Pool>,
        generation: usize,
    ) -> Self {
        Self {
            serialized,
            logger,
            config,
            channel_pool,
            generation,
        }
    }

    /// Consumes the handle and attempts the publish to RabbitMQ
    /// This can only fail if the pool cannot retrieve a connection
    /// or the `basic_publish` call on the RabbitMQ connection fails.
    async fn try_publish(self) -> Result<(), Error> {
        // Asynchronously obtain a channel from the pool
        let channel = self
            .channel_pool
            .get()
            .await
            .map_err(|err| match err {
                PoolError::Backend(err) => err,
                PoolError::Closed => anyhow!("the pool has been closed"),
                PoolError::NoRuntimeSpecified => panic!("No async runtime was specified"),
                PoolError::Timeout(timeout) => {
                    anyhow!("timeout error from pool: {:?}", timeout)
                }
            })
            .map_err(|err| Error {
                logger: self.logger.clone(),
                generation: self.generation,
                inner: err,
            })?;

        // Finally, publish the event to the durable queue
        let generation = self.generation;
        let logger = self.logger.clone();
        channel
            .basic_publish(
                &self.config.gateway_queue.exchange,
                &self.config.gateway_queue.routing_key,
                BasicPublishOptions::default(),
                self.serialized,
                // 2 = persistent/durable
                // `https://www.rabbitmq.com/publishers.html#message-properties`
                BasicProperties::default().with_delivery_mode(2),
            )
            .await
            .context("could not publish gateway event to queue")
            .map_err(|err| Error {
                logger,
                generation,
                inner: err,
            })?;

        Ok(())
    }
}

/// Declares the Rabbit MQ queue, which is done during initialization of the Rabbit MQ connection
async fn declare_event_queue(
    rmq_connection: &Connection,
    config: Arc<Configuration>,
    logger: Logger,
) -> Result<Channel, anyhow::Error> {
    // Create a temporary channel
    let rmq_channel = rmq_connection
        .create_channel()
        .await
        .context("could not create a new RabbitMQ channel")?;

    // Declare the queue
    let queue_name = &config.gateway_queue.queue_name;
    let queue_options = QueueDeclareOptions {
        durable: config.gateway_queue.durable,
        ..QueueDeclareOptions::default()
    };
    let arguments = config
        .gateway_queue
        .queue_parameters
        .as_ref()
        .cloned()
        .map_or_else(FieldTable::default, |map| {
            FieldTable::from(
                map.into_iter()
                    .map(|(key, value)| (lapin::types::ShortString::from(key), value))
                    .collect::<BTreeMap<_, _>>(),
            )
        });
    rmq_channel
        .queue_declare(queue_name, queue_options, arguments)
        .await
        .context("could not declare the RabbitMQ queue")?;

    slog::info!(
        logger,
        "declared RabbitMQ queue";
        "queue_name" => queue_name,
        "rabbit_url" => &config.services.gateway_queue,
    );
    Ok(rmq_channel)
}

/// Attempts to synchronously convert a raw gateway event into our struct
/// that will eventually be published to the gateway queue
fn convert_raw_event(
    event: Event,
    timestamp: u64,
    id: HoarFrost,
    logger: &Logger,
) -> Option<GatewayEvent> {
    if let Event::ShardPayload(Payload { bytes }) = event {
        let json = match std::str::from_utf8(&bytes) {
            Ok(json) => json,
            Err(err) => {
                slog::warn!(
                    logger,
                    "an error occurred while deserializing gateway JSON";
                    "error" => ?err
                );
                return None;
            }
        };

        // Use twilight's fast pre-deserializer to determine the op type,
        // and only deserialize it if it:
        // - is a proper Gateway dispatch event
        // - has a matching processor
        let deserializer = GatewayEventDeserializer::from_json(json)?;
        let (op, _, event_type) = deserializer.into_parts();
        if op != OpCode::Event as u8 {
            return None;
        }

        // Make sure we should forward the event
        let event_type_str = event_type.as_deref()?;
        let event_type =
            serde_json::from_str::<EventType>(&format!(r#""{}""#, event_type_str)).ok();
        if !should_forward(event_type) {
            return None;
        }

        let value = serde_json::from_str::<serde_json::Value>(json).ok()?;
        if let serde_json::Value::Object(map) = value {
            // Attempt to find the ".d" value (contains the Gateway message payload)
            // https://discord.com/developers/docs/topics/gateway#payloads-gateway-payload-structure
            let mut map = map;
            let inner_json = map.remove("d")?;

            // Make sure the guild id can be extracted before forwarding
            let guild_id_option = try_extract_guild_id(&inner_json, event_type);
            let guild_id = if let Some(guild_id) = guild_id_option {
                guild_id
            } else {
                slog::warn!(
                    logger,
                    "no guild id was extracted for event";
                    "event_type" => event_type_str,
                    "inner_json" => ?inner_json,
                );
                return None;
            };

            // Serialize the inner event to MessagePack
            let inner_json_bytes = match rmp_serde::to_vec(&inner_json) {
                Ok(buf) => buf,
                Err(err) => {
                    slog::warn!(
                        logger,
                        "could not serialize inner event JSON to MessagePack";
                        "error" => ?err,
                        "event_inner" => ?inner_json,
                    );
                    return None;
                }
            };

            return Some(GatewayEvent {
                id: id.0,
                ingress_timestamp: timestamp,
                inner: inner_json_bytes,
                event_type: event_type_str.to_owned(),
                guild_id,
            });
        }
    }

    None
}

lazy_static! {
    /// Includes all guild-related events that are processed.
    /// Signals to Discord that we intend to receive and process them
    pub static ref INTENTS: Intents = Intents::GUILD_MEMBERS
        | Intents::GUILD_MESSAGES
        | Intents::GUILD_MESSAGE_REACTIONS;
}

/// Determines whether the ingress shard should forward events to the queue
/// (certain events, such as raw gateway lifecycle events, should not be forwarded)
const fn should_forward(event_type_option: Option<EventType>) -> bool {
    // Don't forward lifecycle events (or typing/presence updates):
    // `https://discord.com/developers/docs/topics/gateway#commands-and-events-gateway-events`
    // Default to forwarding an event if it is not identified
    match event_type_option {
        Some(event_type) => {
            match event_type {
                // Based on the available processors in:
                // logs/gateway-normalize/src/gateway/processors.rs
                EventType::MemberAdd
                    | EventType::MemberRemove
                    | EventType::MessageCreate
                    | EventType::MessageUpdate
                    | EventType::MessageDelete
                    | EventType::MessageDeleteBulk
                    | EventType::InteractionCreate
                    | EventType::ReactionAdd
                    | EventType::ReactionRemove
                    | EventType::ReactionRemoveEmoji
                    | EventType::ReactionRemoveAll => true,
                EventType::GatewayHeartbeat
                    | EventType::GatewayHeartbeatAck
                    | EventType::GatewayHello
                    | EventType::GatewayInvalidateSession
                    | EventType::GatewayReconnect
                    | EventType::GiftCodeUpdate
                    | EventType::MemberChunk
                    | EventType::PresenceUpdate
                    | EventType::PresencesReplace
                    | EventType::Ready
                    | EventType::Resumed
                    | EventType::ShardConnected
                    | EventType::ShardConnecting
                    | EventType::ShardDisconnected
                    | EventType::ShardIdentifying
                    | EventType::ShardReconnecting
                    | EventType::ShardPayload
                    | EventType::ShardResuming
                    | EventType::TypingStart
                    | EventType::UnavailableGuild
                    | EventType::VoiceServerUpdate
                    // Disable forwarding events for guilds coming off/online
                    // Note: that means we'll have to have some other mechanism for logging bot joins/leaves
                    | EventType::GuildCreate
                    | EventType::GuildDelete
                    // Disable forwarding events for those that are handled via the audit log
                    | EventType::BanAdd
                    | EventType::BanRemove
                    | EventType::ChannelCreate
                    | EventType::ChannelDelete
                    | EventType::ChannelPinsUpdate
                    | EventType::ChannelUpdate
                    | EventType::GuildEmojisUpdate
                    | EventType::GuildIntegrationsUpdate
                    | EventType::GuildUpdate
                    | EventType::IntegrationCreate
                    | EventType::IntegrationDelete
                    | EventType::IntegrationUpdate
                    | EventType::StageInstanceCreate
                    | EventType::StageInstanceDelete
                    | EventType::StageInstanceUpdate
                    | EventType::InviteCreate
                    | EventType::InviteDelete
                    | EventType::MemberUpdate
                    | EventType::RoleCreate
                    | EventType::RoleDelete
                    | EventType::RoleUpdate
                    | EventType::UserUpdate
                    | EventType::VoiceStateUpdate
                    | EventType::WebhooksUpdate => false,
            }
        }
        // Forward events that couldn't be matched
        None => true,
    }
}

/// Attempts to extract a guild id from a partially-serialized gateway event
fn try_extract_guild_id(
    json_value: &serde_json::Value,
    _event_type: Option<EventType>,
) -> Option<u64> {
    if let serde_json::Value::Object(map) = json_value {
        if let Some(serde_json::Value::String(guild_id_string)) = map.get("guild_id") {
            // Attempt to parse the guild id string to a u64
            return guild_id_string.parse::<u64>().ok();
        }
    }

    None
}
