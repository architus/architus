//! Contains the core submission logic that submits a batch of events at once to Elasticsearch
//! and notifiers a separate oneshot channel for each event of the result.

use crate::config::Configuration;
use crate::elasticsearch::{EnsureIndexExistsError, IndexStatus};
use crate::rpc::logs::event::Event as ProtoEvent;
use crate::rpc::logs_submission_schema::StoredEvent;
use anyhow::Context;
use bytes::Bytes;
use slog::Logger;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::oneshot;
use tonic::Status;

#[derive(Debug, Clone)]
pub struct Failure {
    pub status: Status,
    pub internal_details: String,
    pub correlation_id: usize,
}

/// Ok() contains the correlation id of the submission operation
pub type OperationResult = Result<usize, Failure>;

type Notifier = oneshot::Sender<OperationResult>;

#[derive(Debug)]
pub struct Event {
    pub id: String,
    pub inner: Box<ProtoEvent>,
    pub notifier: Notifier,
}

#[derive(Debug, Clone)]
struct InternalFailure {
    pub status: Status,
    pub internal_details: String,
}

#[derive(Debug, Clone)]
struct InternalResult {
    id: String,
    result: Result<(), InternalFailure>,
}

/// Creates the Elasticsearch index before submitting any events
/// by reading in the file that contains the mappings at the path in the config
/// and sending it to Elasticsearch
pub async fn create_index(
    config: Arc<Configuration>,
    logger: Logger,
    client: Arc<crate::elasticsearch::Client>,
) -> anyhow::Result<()> {
    let path = &config.elasticsearch_index_config_path;
    let index = &config.elasticsearch_index;

    slog::info!(
        logger,
        "reading and parsing elasticsearch index config file";
        "path" => #?&path,
        "elasticsearch_index" => &index,
    );

    // Load in the file from the file system
    let index_settings = read_index_settings_file(path).await?;

    // Send the index config bytes to Elasticsearch
    let initialization_backoff = config.initialization_backoff.build();
    let try_ensure_index_exists = || async {
        // Cloning this is cheap
        let index_settings = index_settings.clone();

        client
            .ensure_index_exists(index, index_settings)
            .await
            .map_err(backoff::Error::Transient)
    };

    let retry_future = backoff::future::retry(initialization_backoff, try_ensure_index_exists);
    match retry_future.await {
        Ok(IndexStatus::CreatedSuccessfully) => {
            slog::info!(
                logger,
                "successfully created index in elasticsearch";
                "elasticsearch_index" => #?&config.elasticsearch_index,
            );
        }
        Ok(IndexStatus::AlreadyExists) => {
            slog::info!(
                logger,
                "index already existed in elasticsearch";
                "elasticsearch_index" => #?&config.elasticsearch_index,
            );
        }
        Err(EnsureIndexExistsError::Failed(err)) => {
            return Err(err).context("could not create index on elasticsearch");
        }
        Err(EnsureIndexExistsError::BodyReadFailure(err)) => {
            return Err(err).context("could not read response body from elasticsearch");
        }
        Err(EnsureIndexExistsError::ErrorStatusCode(status_code)) => {
            return Err(anyhow::anyhow!(format!(
                "could not create index on elasticsearch; server responded with {:?}",
                status_code
            )));
        }
    }

    Ok(())
}

async fn read_index_settings_file(path: impl AsRef<Path>) -> anyhow::Result<Bytes> {
    let path_ref = path.as_ref();
    let mut index_config_file = File::open(path_ref).context(format!(
        "could not open index settings file at '{:#?}'",
        path_ref
    ))?;

    let mut file_contents = Vec::new();
    index_config_file
        .read_to_end(&mut file_contents)
        .context(format!(
            "could not read index settings file at '{:#?}'",
            path_ref
        ))?;
    std::mem::drop(index_config_file);

    // Parse the JSON to validate that it's valid before re-serializing
    let _parse_result: serde_json::Value =
        serde_json::from_slice(&file_contents).context(format!(
            "could not deserialize JSON index settings file at '{:#?}'",
            path_ref
        ))?;

    Ok(Bytes::from(file_contents))
}

/// Contains all of the behavior to perform a bulk submission to Elasticsearch
pub struct BatchSubmit {
    pub correlation_id: usize,
    pub config: Arc<Configuration>,
    pub logger: Logger,
    pub elasticsearch: Arc<crate::elasticsearch::Client>,
}

struct WithId<T> {
    id: String,
    inner: T,
}

impl<T> WithId<T> {
    fn new(inner: T, id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            inner,
        }
    }

    fn split(self) -> (String, T) {
        (self.id, self.inner)
    }
}

use crate::elasticsearch::{BulkError, BulkItem, BulkOperation, MakeBulkOperationError};

impl BatchSubmit {
    pub fn new(
        correlation_id: usize,
        config: Arc<Configuration>,
        logger: &Logger,
        elasticsearch: Arc<crate::elasticsearch::Client>,
    ) -> Self {
        Self {
            correlation_id,
            config,
            logger: logger.new(slog::o!("correlation_id" => correlation_id)),
            elasticsearch,
        }
    }

    /// Performs the bulk submission operation,
    /// sending each event to Elasticsearch in a bulk index operation
    /// before notifying all submitters of the results
    pub async fn run(self, events: Vec<Event>) {
        slog::debug!(
            self.logger,
            "preparing to send batch of events to elasticsearch";
            "event_ids" => ?events.iter().map(|event| &event.id).cloned().collect::<Vec<_>>()
        );

        let send_future = self.submit_all(events);
        let notifiers_and_results = send_future.await;
        self.notify_all(notifiers_and_results);
    }

    fn construct_bulk_bodies(
        &self,
        stored_events_mut: &mut Vec<StoredEvent>,
    ) -> (Vec<WithId<BulkOperation>>, Vec<WithId<InternalFailure>>) {
        // Grab the current time as milliseconds.
        // This is used as the "ingestion_timestamp" field on each document.
        let time_ms: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
            .try_into()
            .expect("System time could not fit into u64");

        let mut operations: Vec<WithId<BulkOperation>> =
            Vec::with_capacity(stored_events_mut.len());
        let mut failures: Vec<WithId<InternalFailure>> = Vec::new();
        for stored_event in stored_events_mut.iter_mut() {
            // Mutate the stored event in-place
            stored_event.ingestion_timestamp = time_ms;

            match crate::elasticsearch::BulkOperation::index(&stored_event.id, &stored_event) {
                Ok(operation) => operations.push(WithId::new(operation, &stored_event.id)),
                Err(err) => {
                    // Create a failure based on the error type
                    let failure = match err {
                        MakeBulkOperationError::ActionSerializationFailure(err) => InternalFailure {
                            status: Status::internal(
                                "could not perform trivial serialization of operation JSON object",
                            ),
                            internal_details: format!("{:?}", err),
                        },
                        MakeBulkOperationError::SourceSerializationFailure(err) => InternalFailure {
                            status: Status::internal(
                                "could not serialize event before sending to Elasticsearch",
                            ),
                            internal_details: format!("{:?}", err),
                        },
                    };
                    failures.push(WithId::new(failure, &stored_event.id));
                }
            }
        }

        (operations, failures)
    }

    async fn submit_all(&self, events: Vec<Event>) -> Vec<(Notifier, InternalResult)> {
        let logger = self.logger.new(slog::o!(
            "elasticsearch_index" => &self.config.elasticsearch_index,
            "attempted_count" => events.len()
        ));

        // Convert the events list to a list of the actual documents that will be stored
        // This is re-used between retries to prevent needing to clone the data.
        // Since we're moving the events out of their original collection,
        // we also create a list for the notifiers
        let (notifiers, mut stored_events): (Vec<_>, Vec<_>) = events
            .into_iter()
            .map(|event| {
                // Split the event into the notifier and the StoredEvent document
                let listener = WithId::new(event.notifier, &event.id);
                let stored_event = StoredEvent {
                    id: event.id.clone(),
                    inner: Some(*event.inner),
                    // This will be populated later
                    ingestion_timestamp: 0,
                };

                (listener, stored_event)
            })
            .unzip();

        let submission_backoff = self.config.submission_backoff.build();
        let send_to_elasticsearch = || {
            let logger = logger.clone();
            let elasticsearch_client = Arc::clone(&self.elasticsearch);
            let index = self.config.elasticsearch_index.clone();

            let (operations, mut failures) = self.construct_bulk_bodies(&mut stored_events);
            let (submitting_ids, submitting_operations): (Vec<_>, Vec<_>) =
                operations.into_iter().map(WithId::split).unzip();

            // Convert the list of failures to a proper result list,
            // which can be added to later.
            let mut results: Vec<InternalResult> = failures
                .into_iter()
                .map(|f| InternalResult {
                    id: f.id,
                    result: Err(f.inner),
                })
                .collect::<Vec<_>>();

            async move {
                if submitting_operations.is_empty() {
                    return Ok(results);
                }

                let submitted_ids_set = submitting_ids.into_iter().collect::<BTreeSet<_>>();
                let logger = logger.new(slog::o!("submitted_count" => submitted_ids_set.len()));
                let bulk_future = elasticsearch_client
                    .bulk(&self.config.elasticsearch_index, &submitting_operations);
                match bulk_future.await {
                    Ok(status) => {
                        for response_item in status.items {
                            match response_item {
                                BulkItem::Index(action) => {
                                    if submitted_ids_set.contains(&action.id) {
                                        // The returned ID is valid/expected
                                        results.push(InternalResult {
                                            id: action.id,
                                            result: Ok(()),
                                        });
                                    } else {
                                        slog::warn!(
                                            logger,
                                            "elasticsearch bulk API response contained unknown document ID";
                                            "document_id" => action.id,
                                        );
                                    }
                                }
                                _ => {
                                    slog::warn!(
                                        logger,
                                        "elasticsearch bulk API response contained non-index operation result";
                                        "document_id" => response_item.id(),
                                        "operation_result" => ?response_item,
                                    );
                                }
                            }
                        }

                        Ok(results)
                    }
                    Err(err) => {
                        // Create a failure that can be re-used for each submitted document ID.
                        // This failure is only used if the retry was exhausted,
                        // otherwise it will try again.
                        let failure = match err {
                            BulkError::Failure(err) => {
                                slog::warn!(
                                    logger,
                                    "sending to elasticsearch failed";
                                    "error" => ?err,
                                );

                                InternalFailure {
                                    status: Status::unavailable("Elasticsearch was unavailable"),
                                    internal_details: format!("{:?}", err),
                                }
                            }
                            BulkError::FailedToDecode(err) => {
                                slog::warn!(
                                    logger,
                                    "decoding response from elasticsearch failed";
                                    "error" => ?err,
                                );

                                InternalFailure {
                                    status: Status::unavailable(
                                        "Elasticsearch sent malformed response",
                                    ),
                                    internal_details: format!("{:?}", err),
                                }
                            }
                        };

                        // Clone the failure for each submitted document ID.
                        for submitted_id in submitted_ids_set {
                            results.push(InternalResult {
                                id: submitted_id,
                                result: Err(failure.clone()),
                            });
                        }

                        // Return the results, but try again unless the retry is exhausted
                        return Err(backoff::Error::Transient((err, results)));
                    }
                }
            }
        };

        let results = match backoff::future::retry(submission_backoff, send_to_elasticsearch).await
        {
            Ok(results) => results,
            Err((err, results)) => {
                slog::warn!(
                    logger,
                    "sending to elasticsearch failed all retries";
                    "error" => ?err,
                    "elasticsearch_index" => &self.config.elasticsearch_index,
                );

                results
            }
        };

        let notifiers_and_results: Vec<(Notifier, InternalResult)> =
            Vec::with_capacity(notifiers.len());
        let mut notifier_map = notifiers
            .into_iter()
            .map(WithId::split)
            .collect::<BTreeMap<_, _>>();

        for result in results {
            // Remove the notifier from the map to move ownership of it
            let notifier = match notifier_map.remove(&result.id) {
                Some(notifier) => notifier,
                None => {
                    slog::warn!(
                        self.logger,
                        "submission result contained unknown or duplicate event id; ignoring";
                        "result" => ?result,
                    );
                    continue;
                }
            };

            notifiers_and_results.push((notifier, result));
        }

        // If there are any remaining elements in notifier_map,
        // give them a fallback failure and log that this happened
        if notifier_map.len() < 0 {
            let dropped_ids = notifier_map.keys().collect::<Vec<_>>();
            let dropped_count = dropped_ids.len();
            slog::warn!(
                logger,
                "submission result dropped events";
                "dropped_ids" => dropped_ids,
                "dropped_count" => dropped_count,
            );

            for (id, notifier) in notifier_map {
                notifiers_and_results.push((
                    notifier,
                    InternalResult {
                        id,
                        result: Err(InternalFailure {
                            status: Status::internal("event did not have corresponding submission result"),
                            internal_details: format!("dropped_count: {}", dropped_count),
                        }),
                    },
                ));
            }
        }

        notifiers_and_results
    }

    /// Consumes the composite results for each
    fn notify_all(&self, notifiers_and_results: Vec<(Notifier, InternalResult)>) {
        for (notifier, result) in notifiers_and_results {
            if let Err(send_err) = notifier.send(self.finalize_result(result)) {
                slog::warn!(
                    self.logger,
                    "sending submission result to notifier failed; ignoring";
                    "error" => ?send_err,
                );
            }
        }
    }

    fn finalize_result(&self, internal_result: InternalResult) -> OperationResult {
        match internal_result.result {
            Ok(_) => Ok(self.correlation_id),
            Err(internal_failure) => Err(Failure{
                status: internal_failure.status,
                internal_details: internal_failure.internal_details,
                correlation_id: self.correlation_id,
            })
        }
    }
}
