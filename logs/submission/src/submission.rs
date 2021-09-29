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
    fn construct_bulk_bodies2(
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

    async fn submit_all2(&self, events: Vec<Event>) -> Vec<(Notifier, InternalResult)> {
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

            let (operations, mut failures) = self.construct_bulk_bodies2(&mut stored_events);
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
    fn notify_all2(&self, notifiers_and_results: Vec<(Notifier, InternalResult)>) {
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

        let send_future = self.submit_all2(events);
        let notifiers_and_results = send_future.await;
        self.notify_all2(notifiers_and_results);

        // // Consume the results to send to each channel
        // let mut id_to_event = source_events
        //     .into_iter()
        //     .map(|event| (event.id.clone(), event))
        //     .collect::<BTreeMap<_, _>>();

        // for (id, result) in results {

        //     // Notify the submitter
        //     if let Err(send_err) = event.notifier.send(result) {
        //         slog::warn!(
        //             self.logger,
        //             "sending submission result to notifier failed; ignoring";
        //             "error" => ?send_err,
        //         );
        //     }
        // }

        // // If there are any remaining events, they were somehow dropped;
        // // notify the sender.
        // let remaining_length = id_to_event.len();
        // for (id, event) in id_to_event {
        //     let result = Err(Failure {
        //         status: Status::internal("Event did not have corresponding submission result"),
        //         internal_details: format!("remaining_length: {}", remaining_length),
        //         correlation_id: self.correlation_id,
        //     });

        //     // Notify the submitter
        //     if let Err(send_err) = event.notifier.send(result) {
        //         slog::warn!(
        //             self.logger,
        //             "sending submission result to notifier failed; ignoring";
        //             "error" => ?send_err,
        //         );
        //     }
        // }
        // let send_future = self.bulk_index_with_retry();
        // let response = match send_future.await {
        //     Ok(response) => response,
        //     Err(err) => {
        //     }
        // };

        // // Try to decode the response into the typed struct
        // let response_struct = match response
        //     .json::<crate::elasticsearch_api::bulk::Response>()
        //     .await
        // {
        //     Ok(response_struct) => response_struct,
        //     Err(decode_err) => {
        //         slog::warn!(
        //             self.logger,
        //             "decoding response from elasticsearch failed";
        //             "error" => ?decode_err,
        //             "elasticsearch_index" => &self.config.elasticsearch_index,
        //         );

        //         let failure = Failure {
        //             status: Status::unavailable("Elasticsearch sent malformed response"),
        //             internal_details: String::from("see original event"),
        //             correlation_id: self.correlation_id,
        //         };
        //         return self.consume_and_notify_all(&Err(failure));
        //     }
        // };

        // slog::info!(
        //     self.logger,
        //     "sending batch index to elasticsearch succeeded"
        // );
        // return self.handle_api_response(response_struct);
    }

    // /// Consumes the composite results for each
    // fn notify_all(&self, events_and_results: Vec<(Event, OperationResult)>) {
    //     for (event, result) in events_and_results {
    //         if let Err(send_err) = event.notifier.send(result) {
    //             slog::warn!(
    //                 self.logger,
    //                 "sending submission result to notifier failed; ignoring";
    //                 "error" => ?send_err,
    //             );
    //         }
    //     }
    // }

    // /// Constructs the separate JSON lines used for the bulk Elasticsearch API,
    // /// using the current timestamp as the ingestion timestamp
    // /// for each wrapped document.
    // // The API format appears as:
    // // ```
    // // { "index": { "_id": <id> } }
    // // <document>
    // // ```
    // // which is then repeated for each document in the operation.
    // // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html
    // // TODO return actual aggregate struct here
    // fn construct_bulk_bodies(
    //     &self,
    //     source_events: &Vec<Event>,
    // ) -> (Vec<Bytes>, BTreeSet<String>, BTreeMap<String, Failure>) {
    //     // Grab the current time as milliseconds.
    //     // This is used as the "ingestion_timestamp" field on each document.
    //     let time_ms: u64 = SystemTime::now()
    //         .duration_since(UNIX_EPOCH)
    //         .expect("Time went backwards")
    //         .as_millis()
    //         .try_into()
    //         .expect("System time could not fit into u64");

    //     let mut bodies = Vec::<Bytes>::with_capacity(source_events.len() * 2);
    //     let mut successes = BTreeSet::<String>::new();
    //     let mut failures = BTreeMap::<String, Failure>::new();
    //     for event in source_events {
    //         match self.construct_bulk_index_group(event, time_ms) {
    //             Ok((operation_line, document_line)) => {
    //                 bodies.push(Bytes::from(operation_line));
    //                 bodies.push(Bytes::from(document_line));
    //                 successes.insert(event.id.clone());
    //             }
    //             Err(failure) => {
    //                 failures.insert(event.id.clone(), failure);
    //             }
    //         }
    //     }

    //     (bodies, successes, failures)
    // }

    // /// Constructs a single "group" of bulk index JSON lines for a single document.
    // /// This produces two serialized byte buffers containing two JSON objects:
    // // ```
    // // { "index": { "_id": <id> } }
    // // <document>
    // // ```
    // // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html
    // fn construct_bulk_index_group(
    //     &self,
    //     event: &Event,
    //     ingestion_timestamp: u64,
    // ) -> Result<(Bytes, Bytes), Failure> {
    //     // Create the "operation" JSON line using the ID
    //     let operation_json_value = serde_json::json!({"index": {"_id": event.id.clone() }});
    //     let operation_buf = match serde_json::to_vec(&operation_json_value) {
    //         Ok(vec) => Bytes::from(vec),
    //         Err(err) => {
    //             return Err(Failure {
    //                 status: Status::internal(
    //                     "could not perform trivial serialization of operation JSON object",
    //                 ),
    //                 internal_details: format!("{:?}", err),
    //                 correlation_id: self.correlation_id,
    //             });
    //         }
    //     };

    //     // Construct the document with its ID and ingestion timestamp
    //     let document = StoredEvent {
    //         id: event.id.clone(),
    //         inner: Some(*event.inner.clone()),
    //         ingestion_timestamp,
    //     };

    //     // Create the document JSON line
    //     let document_buf = match serde_json::to_vec(&document) {
    //         Ok(vec) => Bytes::from(vec),
    //         Err(err) => {
    //             return Err(Failure {
    //                 status: Status::internal(
    //                     "could not serialize event before sending to Elasticsearch",
    //                 ),
    //                 internal_details: format!("{:?}", err),
    //                 correlation_id: self.correlation_id,
    //             });
    //         }
    //     };

    //     Ok((operation_buf, document_buf))
    // }

    // /// Sends a bulk index operation to the Elasticsearch data store
    // /// using retry parameters specified in the config.
    // /// Returns a list of tuples containing the result
    // /// for the submission of each event.
    // async fn submit_all(&self, events: Vec<Event>) -> Vec<InternalResult> {
    //     let submission_backoff = self.config.submission_backoff.build();
    //     let send_to_elasticsearch = || async {
    //         let elasticsearch = Arc::clone(&self.elasticsearch);

    //         // Construct all of the bulk operations as separate JSON objects.
    //         // We do this every operation so that we have the timestamp
    //         // that the events actually went into Elasticsearch.
    //         let (bodies, successes, failed) = self.construct_bulk_bodies(&events);
    //         let mut failed = failed;

    //         // Only perform the operation if there are any documents to send.
    //         let index_results = if bodies.len() > 0 {
    //             match self.bulk_index(bodies).await {
    //                 Ok(response) => {
    //                     slog::info!(
    //                         self.logger,
    //                         "sending batch index to elasticsearch succeeded"
    //                     );

    //                     let response_struct = match response
    //                         .json::<crate::elasticsearch_api::bulk::Response>()
    //                         .await
    //                     {
    //                         Ok(response_struct) => response_struct,
    //                         Err(decode_err) => {
    //                             slog::warn!(
    //                                 self.logger,
    //                                 "decoding response from elasticsearch failed";
    //                                 "error" => ?decode_err,
    //                                 "elasticsearch_index" => &self.config.elasticsearch_index,
    //                             );

    //                             // Elasticsearch is unavailable;
    //                             // mark each event that originally succeeded serialization
    //                             // as having failed its submission
    //                             let failure = Failure {
    //                                 status: Status::unavailable("Elasticsearch was unavailable"),
    //                                 internal_details: String::from("see original log line"),
    //                                 correlation_id: self.correlation_id,
    //                             };
    //                             successes.iter().for_each(|id| {
    //                                 failed.insert(
    //                                     id.clone(),
    //                                     Failure {
    //                                         status: Status::unavailable(
    //                                             "Elasticsearch sent malformed response",
    //                                         ),
    //                                         internal_details: String::from("see original log line"),
    //                                         correlation_id: self.correlation_id,
    //                                     },
    //                                 );
    //                             });
    //                             return Ok((failed, vec![]));
    //                         }
    //                     };

    //                     let mut index_results =
    //                         Vec::<crate::elasticsearch_api::bulk::ResultItemAction>::new();
    //                     for response_item in response_struct.items {
    //                         match response_item.index {
    //                             Some(action) => {
    //                                 index_results.push(action);
    //                             }
    //                             None => {
    //                                 slog::warn!(
    //                                     self.logger,
    //                                     "response item from elasticsearch missing 'index' action field, ignoring";
    //                                     "response_item" => ?response_item,
    //                                 );
    //                             }
    //                         }
    //                     }
    //                     index_results
    //                 }
    //                 Err(err) => {
    //                     slog::warn!(
    //                         self.logger,
    //                         "sending to elasticsearch failed";
    //                         "error" => ?err,
    //                     );

    //                     return Err(backoff::Error::Transient(err));
    //                 }
    //             }
    //         } else {
    //             vec![]
    //         };

    //         Ok((failed, index_results))
    //     };

    //     let submit_future = backoff::future::retry(submission_backoff, send_to_elasticsearch);
    //     match submit_future.await {
    //         Ok((failed, index_results)) => {
    //             self.coalesce_submission_results(events, failed, index_results)
    //         }
    //         Err(err) => {
    //             slog::warn!(
    //                 self.logger,
    //                 "sending to elasticsearch failed all retries";
    //                 "error" => ?err,
    //                 "elasticsearch_index" => &self.config.elasticsearch_index,
    //             );

    //             // Elasticsearch is unavailable;
    //             // mark each event as having failed its submission
    //             let failure = Failure {
    //                 status: Status::unavailable("Elasticsearch was unavailable"),
    //                 internal_details: String::from("see original log line"),
    //                 correlation_id: self.correlation_id,
    //             };
    //             events
    //                 .into_iter()
    //                 .map(|event| (event, Err(failure.clone())))
    //                 .collect::<Vec<_>>()
    //         }
    //     }
    // }

    // fn coalesce_submission_results(
    //     &self,
    //     events: Vec<Event>,
    //     failed: BTreeMap<String, Failure>,
    //     index_results: Vec<crate::elasticsearch_api::bulk::ResultItemAction>,
    // ) -> Vec<(Event, OperationResult)> {
    //     // TODO implement
    //     vec![]
    // }

    // async fn bulk_index(
    //     &self,
    //     bodies: Vec<Bytes>,
    // ) -> Result<elasticsearch::http::response::Response, elasticsearch::Error> {
    //     self.elasticsearch
    //         .bulk(BulkParts::Index(&self.config.elasticsearch_index))
    //         .body(bodies)
    //         .send()
    //         .await
    // }

    // /// Consumes the parsed bulk API response,
    // /// notifying all submitters of the results by examining each response item individually
    // fn handle_api_response(self, response: crate::elasticsearch_api::bulk::Response) {
    //     for response_item in response.items {
    //         if let Some(action) = unwrap_index_action(&response_item, &self.logger) {
    //             let logger = self.logger.new(slog::o!("event_id" => action.id.clone()));
    //             let id = &action.id;

    //             // Remove the event from the map to move ownership of it
    //             let event = if let Some(event) = id_to_event.remove(id) {
    //                 event
    //             } else {
    //                 slog::warn!(
    //                     logger,
    //                     "response item from elasticsearch contained unknown or duplicate event id; ignoring";
    //                     "response_item" => ?response_item,
    //                 );
    //                 continue;
    //             };

    //             // Create the submission result depending on whether an error occurred or not
    //             let submission_result = match &action.error {
    //                 Some(err) => Err(Failure {
    //                     status: Status::internal("Elasticsearch failed index operation for event"),
    //                     internal_details: format!("error object: {:?}", err),
    //                     correlation_id: self.correlation_id,
    //                 }),
    //                 None => Ok(self.correlation_id),
    //             };

    //             // Notify the submitter
    //             if let Err(send_err) = event.notifier.send(submission_result) {
    //                 slog::warn!(
    //                     logger,
    //                     "sending submission result to notifier failed; ignoring";
    //                     "error" => ?send_err,
    //                 );
    //             }
    //         }
    //     }
    // }
}

// /// Attempts to unwrap the `ResultItem` struct into the inner `ResultItemAction`
// /// that should exist at the `index` field since the original actions were index operations.
// fn unwrap_index_action(
//     response_item: crate::elasticsearch_api::bulk::ResultItem,
//     logger: Logger,
// ) -> Option<&'a crate::elasticsearch_api::bulk::ResultItemAction> {
//     match &response_item.index {
//         Some(action) => Some(action),
//         None => {

//             None
//         }
//     }
// }
