//! Contains the core submission logic that submits a batch of events at once to Elasticsearch
//! and notifiers a separate oneshot channel for each event of the result.

use crate::config::Configuration;
use crate::rpc::logs::event::Event as ProtoEvent;
use crate::rpc::logs_submission_schema::StoredEvent;
use anyhow::Context;
use bytes::Bytes;
use elasticsearch::http::headers::HeaderMap;
use elasticsearch::http::{Method, StatusCode};
use elasticsearch::indices::IndicesCreateParts;
use elasticsearch::{BulkParts, Elasticsearch};
use slog::Logger;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;
use std::fs::File;
use std::io::Read;
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

#[derive(Debug)]
pub struct Event {
    pub id: String,
    pub inner: Box<ProtoEvent>,
    pub notifier: oneshot::Sender<OperationResult>,
}

// // Serialize the inner event to JSON
// let json = serde_json::to_vec(&stored_event).map_err(|err| {
//     slog::warn!(
//         logger,
//         "could not serialize event to JSON";
//         "event" => ?stored_event,
//         "error" => ?err,
//     );
//     Status::invalid_argument(format!("could not encode event to JSON: {:?}", err))
// })?;
// let json_body = Bytes::from(json);

/// Creates the Elasticsearch index before submitting any events
/// by reading in the file that contains the mappings at the path in the config
/// and sending it to Elasticsearch
pub async fn create_index(
    config: Arc<Configuration>,
    logger: Logger,
    elasticsearch: Arc<Elasticsearch>,
) -> anyhow::Result<()> {
    // Load in the file from the file system
    slog::info!(
        logger,
        "reading and parsing elasticsearch index config file";
        "path" => #?&config.elasticsearch_index_config_path,
        "elasticsearch_index" => &config.elasticsearch_index,
    );
    let mut index_config_file =
        File::open(&config.elasticsearch_index_config_path).context(format!(
            "could not open index config file at '{:#?}'",
            config.elasticsearch_index_config_path
        ))?;
    let mut file_contents = Vec::new();
    index_config_file
        .read_to_end(&mut file_contents)
        .context(format!(
            "could not read index config file at '{:#?}'",
            config.elasticsearch_index_config_path
        ))?;
    std::mem::drop(index_config_file);

    // Parse the JSON to validate that it's valid before re-serializing
    let parsed_json: serde_json::Value =
        serde_json::from_slice(&file_contents).context(format!(
            "could not deserialize JSON index config file at '{:#?}'",
            config.elasticsearch_index_config_path
        ))?;

    // Re-serialize the JSON back to bytes
    let bytes = Bytes::from(serde_json::to_vec(&parsed_json).context(format!(
        "could not re-serialize JSON index config file at '{:#?}'",
        config.elasticsearch_index_config_path
    ))?);

    // Send the index config bytes to Elasticsearch
    ensure_index_exists(config, logger, elasticsearch, bytes).await?;
    Ok(())
}

/// Attempts to create the index on Elasticsearch,
/// detecting and ignoring if it already exists.
async fn ensure_index_exists(
    config: Arc<Configuration>,
    logger: Logger,
    elasticsearch: Arc<Elasticsearch>,
    body: Bytes,
) -> anyhow::Result<()> {
    // Send the JSON as bytes to Elasticsearch
    let response =
        create_index_with_retry(Arc::clone(&config), logger.clone(), elasticsearch, body)
            .await
            .context("could not create index on elasticsearch")?;
    let status_code = response.status_code();
    if !status_code.is_success() {
        let content = response
            .text()
            .await
            .context("could not read response body from elasticsearch")?;

        // Check to see if the index already existed (if so, it's fine)
        let is_exist_error = if status_code == StatusCode::BAD_REQUEST {
            content.contains("resource_already_exists_exception")
        } else {
            false
        };

        if is_exist_error {
            slog::info!(
                logger,
                "index already existed in elasticsearch; ignoring";
                "elasticsearch_index" => #?&config.elasticsearch_index,
            );
            return Ok(());
        }

        return Err(anyhow::anyhow!(
            "creating index in elasticsearch failed with response code {:?}:\n{}",
            status_code,
            content,
        ));
    }

    slog::info!(
        logger,
        "successfully created index in elasticsearch";
        "elasticsearch_index" => #?&config.elasticsearch_index,
    );
    Ok(())
}

/// Sends a an index create operation to the Elasticsearch data store
/// using retry parameters specified in the config
async fn create_index_with_retry(
    config: Arc<Configuration>,
    logger: Logger,
    elasticsearch: Arc<Elasticsearch>,
    body: Bytes,
) -> Result<elasticsearch::http::response::Response, elasticsearch::Error> {
    let index_creation_backoff = config.index_creation_backoff.build();
    let create_index = || async {
        let elasticsearch = Arc::clone(&elasticsearch);
        let body = body.clone();
        // Use the untyped send API so that the raw bytes can be used
        elasticsearch
            .send(
                Method::Put,
                IndicesCreateParts::Index(&config.elasticsearch_index)
                    .url()
                    .as_ref(),
                HeaderMap::new(),
                Option::<&serde_json::Value>::None,
                Some(body),
                None,
            )
            .await
            .map_err(|err| {
                slog::warn!(
                    logger,
                    "creating elasticsearch index failed";
                    "error" => ?err,
                    "elasticsearch_index" => &config.elasticsearch_index,
                );
                backoff::Error::Transient(err)
            })
    };

    backoff::future::retry(index_creation_backoff, create_index).await
}


/// Contains all of the behavior to perform a bulk submission to Elasticsearch
pub struct BatchSubmit {
    pub correlation_id: usize,
    pub config: Arc<Configuration>,
    pub logger: Logger,
    pub elasticsearch: Arc<Elasticsearch>,
}

impl BatchSubmit {
    pub fn new(
        correlation_id: usize,
        config: Arc<Configuration>,
        logger: &Logger,
        elasticsearch: Arc<Elasticsearch>,
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
    pub async fn run(self, events: Vec<Event>,) {
        slog::debug!(
            self.logger,
            "preparing to send batch of events to elasticsearch";
            "event_ids" => ?events.iter().map(|event| &event.id).cloned().collect::<Vec<_>>()
        );

        let send_future = self.submit_all(events);
        let events_and_results = send_future.await;
        self.notify_all(events_and_results);





        // // Consume the results to send to each channel
        // let mut id_to_event = source_events
        //     .into_iter()
        //     .map(|event| (event.id.clone(), event))
        //     .collect::<BTreeMap<_, _>>();

        // for (id, result) in results {
        //     // Remove the event from the map to move ownership of it
        //     let event = if let Some(event) = id_to_event.remove(&id) {
        //         event
        //     } else {
        //         slog::warn!(
        //             self.logger,
        //             "submission result contained unknown or duplicate event id; ignoring";
        //             "result" => ?result,
        //         );
        //         continue;
        //     };

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

    /// Consumes the composite results for each
    fn notify_all(&self, events_and_results: Vec<(Event, OperationResult)>) {
        for (event, result) in events_and_results {
            if let Err(send_err) = event.notifier.send(result) {
                slog::warn!(
                    self.logger,
                    "sending submission result to notifier failed; ignoring";
                    "error" => ?send_err,
                );
            }
        }
    }

    /// Constructs the separate JSON lines used for the bulk Elasticsearch API,
    /// using the current timestamp as the ingestion timestamp
    /// for each wrapped document.
    // The API format appears as:
    // ```
    // { "index": { "_id": <id> } }
    // <document>
    // ```
    // which is then repeated for each document in the operation.
    // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html
    // TODO return actual aggregate struct here
    fn construct_bulk_bodies(&self, source_events: &Vec<Event>) -> (Vec<Bytes>, BTreeSet<String>, BTreeMap<String, Failure>) {
        // Grab the current time as milliseconds.
        // This is used as the "ingestion_timestamp" field on each document.
        let time_ms: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
            .try_into()
            .expect("System time could not fit into u64");

        let mut bodies = Vec::<Bytes>::with_capacity(source_events.len() * 2);
        let mut successes = BTreeSet::<String>::new();
        let mut failures = BTreeMap::<String, Failure>::new();
        for event in source_events {
            match self.construct_bulk_index_group(event, time_ms) {
                Ok((operation_line, document_line)) => {
                    bodies.push(Bytes::from(operation_line));
                    bodies.push(Bytes::from(document_line));
                    successes.insert(event.id.clone());
                },
                Err(failure) => {
                    failures.insert(event.id.clone(), failure);
                }
            }

        }

        (bodies, successes, failures)
    }

    /// Constructs a single "group" of bulk index JSON lines for a single document.
    /// This produces two serialized byte buffers containing two JSON objects:
    // ```
    // { "index": { "_id": <id> } }
    // <document>
    // ```
    // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html
    fn construct_bulk_index_group(&self, event: &Event, ingestion_timestamp: u64) -> Result<(Bytes, Bytes), Failure> {
        // Create the "operation" JSON line using the ID
        let operation_json_value = serde_json::json!({"index": {"_id": event.id.clone() }});
        let operation_buf = match serde_json::to_vec(&operation_json_value) {
            Ok(vec) => Bytes::from(vec),
            Err(err) => {
                return Err(Failure{
                    status: Status::internal("could not perform trivial serialization of operation JSON object"),
                    internal_details: format!("{:?}", err),
                    correlation_id: self.correlation_id,
                });
            }
        };

        // Construct the document with its ID and ingestion timestamp
        let document = StoredEvent {
            id: event.id.clone(),
            inner: Some(*event.inner.clone()),
            ingestion_timestamp,
        };

        // Create the document JSON line
        let document_buf = match serde_json::to_vec(&document) {
            Ok(vec) => Bytes::from(vec),
            Err(err) => {
                return Err(Failure{
                    status: Status::internal("could not serialize event before sending to Elasticsearch"),
                    internal_details: format!("{:?}", err),
                    correlation_id: self.correlation_id,
                });
            }
        };

        Ok((operation_buf, document_buf))
    }

    /// Sends a bulk index operation to the Elasticsearch data store
    /// using retry parameters specified in the config.
    /// Returns a list of tuples containing the result
    /// for the submission of each event.
    async fn submit_all(
        &self,
        events: Vec<Event>,
    ) -> Vec<(Event, OperationResult)> {
        let submission_backoff = self.config.submission_backoff.build();
        let send_to_elasticsearch = || async {
            let elasticsearch = Arc::clone(&self.elasticsearch);

            // Construct all of the bulk operations as separate JSON objects.
            // We do this every operation so that we have the timestamp
            // that the events actually went into Elasticsearch.
            let (bodies, successes, failed) = self
                .construct_bulk_bodies(&events);
            let mut failed = failed;

            // Only perform the operation if there are any documents to send.
            let index_results = if bodies.len() > 0 {
                match self.bulk_index(bodies).await {
                    Ok(response) => {
                        slog::info!(
                            self.logger,
                            "sending batch index to elasticsearch succeeded"
                        );

                        let response_struct = match response
                            .json::<crate::elasticsearch_api::bulk::Response>()
                            .await
                        {
                            Ok(response_struct) => response_struct,
                            Err(decode_err) => {
                                slog::warn!(
                                    self.logger,
                                    "decoding response from elasticsearch failed";
                                    "error" => ?decode_err,
                                    "elasticsearch_index" => &self.config.elasticsearch_index,
                                );

                                // Elasticsearch is unavailable;
                                // mark each event that originally succeeded serialization
                                // as having failed its submission
                                let failure = Failure {
                                    status: Status::unavailable("Elasticsearch was unavailable"),
                                    internal_details: String::from("see original log line"),
                                    correlation_id: self.correlation_id,
                                };
                                successes.iter().for_each(|id| {
                                    failed.insert(id.clone(), Failure {
                                        status: Status::unavailable("Elasticsearch sent malformed response"),
                                        internal_details: String::from("see original log line"),
                                        correlation_id: self.correlation_id,
                                    });
                                });
                                return Ok((failed, vec!()));
                            }
                        };

                        let mut index_results = Vec::<crate::elasticsearch_api::bulk::ResultItemAction>::new();
                        for response_item in response_struct.items {
                            match response_item.index {
                                Some(action) => {
                                    index_results.push(action);
                                },
                                None => {
                                    slog::warn!(
                                        self.logger,
                                        "response item from elasticsearch missing 'index' action field, ignoring";
                                        "response_item" => ?response_item,
                                    );
                                }
                            }
                        }
                        index_results
                    },
                    Err(err) => {
                        slog::warn!(
                            self.logger,
                            "sending to elasticsearch failed";
                            "error" => ?err,
                        );

                        return Err(backoff::Error::Transient(err));
                    }
                }
            } else {
                vec!()
            };

            Ok((failed, index_results))
        };

        let submit_future = backoff::future::retry(submission_backoff, send_to_elasticsearch);
        match submit_future.await {
            Ok((failed, index_results)) => self.coalesce_submission_results(events, failed, index_results),
            Err(err) => {
                slog::warn!(
                    self.logger,
                    "sending to elasticsearch failed all retries";
                    "error" => ?err,
                    "elasticsearch_index" => &self.config.elasticsearch_index,
                );

                // Elasticsearch is unavailable;
                // mark each event as having failed its submission
                let failure = Failure {
                    status: Status::unavailable("Elasticsearch was unavailable"),
                    internal_details: String::from("see original log line"),
                    correlation_id: self.correlation_id,
                };
                events.into_iter().map(|event| (event, Err(failure.clone()))).collect::<Vec<_>>()
            }
        }
    }

    fn coalesce_submission_results(&self, events: Vec<Event>, failed: BTreeMap<String, Failure>, index_results: Vec<crate::elasticsearch_api::bulk::ResultItemAction>) -> Vec<(Event, OperationResult)> {
        // TODO implement
        vec!()
    }

    async fn bulk_index(&self, bodies: Vec<Bytes>) -> Result<elasticsearch::http::response::Response, elasticsearch::Error> {
        self.elasticsearch
            .bulk(BulkParts::Index(&self.config.elasticsearch_index))
            .body(bodies)
            .send()
            .await
    }

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
