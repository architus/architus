//! Handles the core publishing logic for taking raw gateway events
//! and sending them to RabbitMQ.
//! The majority of the complexity of this module
//! comes from the logic to handle having a fleet of background asynchronous tasks
//! be able to acquire a valid lapin::Channel instance before publishing an event.
//! If it is unable to do so, it will wait until a background task
//! is able to re-establish a connection to RabbitMQ and wake up all waiters.

use crate::config::Configuration;
use crate::rpc::gateway_queue_lib::GatewayEvent;
use anyhow::{anyhow, Context};
use architus_amqp_pool::{Manager, Pool, PoolError};
use backoff::backoff::Backoff;
use deadpool::Runtime;
use futures::{Stream, StreamExt};
use lapin::options::{BasicPublishOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection};
use prost::Message;
use slog::Logger;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};

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
    config: Arc<Configuration>,
    logger: Logger,
    handle_factory: Arc<HandleFactory>,
    reconnect_rx: Option<mpsc::UnboundedReceiver<ReconnectRequest>>,
}

impl Publisher {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(queue_connection: Connection, config: Arc<Configuration>, logger: Logger) -> Self {
        let logger = logger.new(slog::o!("max_queue_length" => config.raw_events.queue_length));
        let (reconnect_tx, reconnect_rx) = mpsc::unbounded_channel::<ReconnectRequest>();
        Self {
            config: Arc::clone(&config),
            logger: logger.clone(),
            handle_factory: Arc::new(HandleFactory::new(
                queue_connection,
                logger,
                config,
                reconnect_tx,
            )),
            reconnect_rx: Some(reconnect_rx),
        }
    }

    /// Runs the publisher indefinitely,
    /// which handles reporting when the queue is reaching its capacity.
    /// The internal `handle_factory` handles reconnecting to the queue when needed
    /// and ensuring that all enqueued messages (up to the channel's capacity)
    /// eventually get published even if transient errors occur reaching RabbitMQ.
    pub async fn consume_events(mut self, event_stream: impl Stream<Item = GatewayEvent>) {
        let reconnect_rx = self.reconnect_rx.take().unwrap();

        // Consume each event from the stream in parallel
        let consume_future = event_stream.for_each_concurrent(
            Some(self.config.raw_events.publish_concurrency),
            |event| async { self.publish_event(event).await },
        );

        // Reconnect to the RabbitMQ instance in the background if it ever fails
        let reconnection_future = self.handle_factory.reconnect_background_loop(reconnect_rx);

        futures::join!(consume_future, reconnection_future);
    }

    /// Publishes a single event to the RabbitMQ message queue.
    /// Blocks until the event can be successfully published,
    /// potentially indefinitely.
    /// This can cause events to start being dropped on the bounded queue.
    async fn publish_event(&self, event: GatewayEvent) {
        let logger = self.logger.new(slog::o!(
            "event_timestamp" => event.ingress_timestamp,
            "guild_id" => event.guild_id,
        ));

        // Serialize the event to raw bytes
        let mut raw_bytes = Vec::with_capacity(event.encoded_len());
        if let Err(encode_err) = event.encode(&mut raw_bytes) {
            slog::warn!(
                logger,
                "could not encode gateway event to bytes";
                "error" => ?encode_err,
                "event" => ?event,
            );
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
    logger: Logger,
    config: Arc<Configuration>,
    state: Mutex<HandleFactoryState>,
    reconnect_tx: mpsc::UnboundedSender<ReconnectRequest>,
}

impl HandleFactory {
    fn new(
        queue_connection: Connection,
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
    ) {
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
                connection: existing_connection,
                next_generation,
            } = request;

            // Ensure we have a valid connection,
            // and declare the queue if needed.
            let connection = self
                .do_reconnect_and_declare_topology(existing_connection)
                .await;

            // Create a pool for the RMQ channels
            let manager = Manager::new(connection);
            let mut pool_config = self.config.gateway_queue.connection_pool.clone();
            pool_config.runtime = Runtime::Tokio1;
            let channel_pool = Pool::from_config(manager, pool_config);

            self.notify_new_connection(channel_pool, ready_tx, next_generation);
        }
    }

    async fn do_reconnect_and_declare_topology(
        &self,
        existing_connection: Option<Connection>,
    ) -> Connection {
        let mut existing_connection = existing_connection;

        // Reconnect to the Rabbit MQ instance and ensure the queue has been declared
        loop {
            let connection = if let Some(connection) = existing_connection.take() {
                connection
            } else {
                // Continuously attempt to make the connection
                self.reconnect().await
            };

            // Declare the RMQ queue to publish incoming events to
            let declare_future =
                declare_event_queue(&connection, Arc::clone(&self.config), self.logger.clone());
            match declare_future.await {
                Ok(_) => return connection,
                Err(err) => {
                    slog::warn!(
                        self.logger,
                        "failed to declare event queue after successful reconnect";
                        "error" => ?err,
                    );

                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        }
    }

    fn notify_new_connection(
        &self,
        channel_pool: Pool,
        ready_tx: broadcast::Sender<()>,
        next_generation: usize,
    ) {
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
            let connect_future = crate::connect::connect_to_queue_attempt(Arc::clone(&self.config));
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
