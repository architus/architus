//! Contains the core submission logic that submits a batch of events at once to Elasticsearch
//! and notifiers a separate oneshot channel for each event of the result.

use crate::config::Configuration;
use anyhow::Context;
use bytes::Bytes;
use elasticsearch::http::headers::HeaderMap;
use elasticsearch::http::{Method, StatusCode};
use elasticsearch::indices::IndicesCreateParts;
use elasticsearch::{BulkParts, Elasticsearch};
use slog::Logger;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::result::Result as StdResult;
use std::sync::Arc;
use tokio::sync::oneshot;
use tonic::Status;

#[derive(Debug, Clone)]
pub struct Failure {
    pub status: Status,
    pub internal_details: String,
    pub correlation_id: usize,
}

/// Ok() contains the correlation id of the submission operation
pub type Result = std::result::Result<usize, Failure>;

#[derive(Debug)]
pub struct Event {
    pub event_id: String,
    pub event_json: Bytes,
    pub notifier: oneshot::Sender<Result>,
}

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
) -> StdResult<elasticsearch::http::response::Response, elasticsearch::Error> {
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
    pub events: Vec<Event>,
    pub correlation_id: usize,
    pub config: Arc<Configuration>,
    pub logger: Logger,
    pub elasticsearch: Arc<Elasticsearch>,
}

impl BatchSubmit {
    pub fn new(
        events: Vec<Event>,
        correlation_id: usize,
        config: Arc<Configuration>,
        logger: &Logger,
        elasticsearch: Arc<Elasticsearch>,
    ) -> Self {
        Self {
            events,
            correlation_id,
            config,
            logger: logger.new(slog::o!("correlation_id" => correlation_id)),
            elasticsearch,
        }
    }

    /// Performs the bulk submission operation,
    /// sending each event to Elasticsearch in a bulk index operation
    /// before notifying all submitters of the results
    pub async fn run(self) {
        slog::debug!(
            self.logger,
            "preparing to send batch of events to elasticsearch";
            "event_ids" => ?self.events.iter().map(|event| &event.event_id).cloned().collect::<Vec<_>>()
        );

        // Construct all of the bulk operations as separate JSON objects
        let bodies = self.construct_bulk_bodies();
        let send_future = self.bulk_index_with_retry(bodies);
        let response = match send_future.await {
            Ok(response) => response,
            Err(err) => {
                slog::warn!(
                    self.logger,
                    "sending to elasticsearch failed all retries";
                    "error" => ?err,
                    "elasticsearch_index" => &self.config.elasticsearch_index,
                );

                // Elasticsearch is unavailable: notify all senders
                let failure = Failure {
                    status: Status::unavailable("Elasticsearch was unavailable"),
                    internal_details: String::from("see original event"),
                    correlation_id: self.correlation_id,
                };
                return self.consume_and_notify_all(&Err(failure));
            }
        };

        // Try to decode the response into the typed struct
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

                let failure = Failure {
                    status: Status::unavailable("Elasticsearch sent malformed response"),
                    internal_details: String::from("see original event"),
                    correlation_id: self.correlation_id,
                };
                return self.consume_and_notify_all(&Err(failure));
            }
        };

        slog::info!(
            self.logger,
            "sending batch index to elasticsearch succeeded"
        );
        return self.handle_api_response(response_struct);
    }

    /// Consumes self and sends all submitters the same submission result
    fn consume_and_notify_all(self, result: &Result) {
        for event in self.events {
            let Event {
                event_id, notifier, ..
            } = event;
            if let Err(send_err) = notifier.send(result.clone()) {
                slog::info!(
                    self.logger,
                    "sending submission result to notifier failed; ignoring";
                    "err" => ?send_err,
                    "event_id" => event_id,
                );
            }
        }
    }

    /// Constructs the separate JSON lines used for the bulk Elasticsearch API.
    // The format appears as:
    // ```
    // { "index": { "_id": <id> } }
    // <document>
    // ```
    // which is then repeated for each document in the operation.
    // https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html
    fn construct_bulk_bodies(&self) -> Vec<Bytes> {
        let mut bodies = Vec::<Bytes>::with_capacity(self.events.len() * 2);
        for event in &self.events {
            let json_value = serde_json::json!({"index": {"_id": event.event_id.to_string() }});
            let operation_buf = serde_json::to_vec(&json_value)
                .map_err(|err| {
                    slog::error!(
                        self.logger,
                        "trivial serialization of index operation failed";
                        "error" => ?err,
                        "event_id" => event.event_id.clone(),
                    )
                })
                // Unwrap here because this should never fail
                .unwrap();
            bodies.push(Bytes::from(operation_buf));
            bodies.push(event.event_json.clone());
        }

        bodies
    }

    /// Sends a bulk index operation to the Elasticsearch data store
    /// using retry parameters specified in the config
    async fn bulk_index_with_retry(
        &self,
        bulk_bodies: Vec<Bytes>,
    ) -> StdResult<elasticsearch::http::response::Response, elasticsearch::Error> {
        let submission_backoff = self.config.submission_backoff.build();
        let send_to_elasticsearch = || async {
            let elasticsearch = Arc::clone(&self.elasticsearch);
            // The inner `Bytes` are small & cheap to clone (basically Rc<Vec<u8>>),
            // so the main cost of this is cloning the Vec's storage
            let bulk_bodies = bulk_bodies.clone();

            elasticsearch
                .bulk(BulkParts::Index(&self.config.elasticsearch_index))
                .body(bulk_bodies)
                .send()
                .await
                .map_err(|err| {
                    slog::warn!(
                        self.logger,
                        "sending to elasticsearch failed";
                        "error" => ?err,
                    );
                    backoff::Error::Transient(err)
                })
        };

        backoff::future::retry(submission_backoff, send_to_elasticsearch).await
    }

    /// Consumes the parsed bulk API response,
    /// notifying all submitters of the results by examining each response item individually
    fn handle_api_response(self, response: crate::elasticsearch_api::bulk::Response) {
        let mut id_to_event = self
            .events
            .into_iter()
            .map(|event| (event.event_id.clone(), event))
            .collect::<BTreeMap<_, _>>();
        for response_item in response.items {
            if let Some(action) = unwrap_index_action(&response_item, &self.logger) {
                let logger = self.logger.new(slog::o!("event_id" => action.id.clone()));
                let id = &action.id;

                // Remove the event from the map to move ownership of it
                let event = if let Some(event) = id_to_event.remove(id) {
                    event
                } else {
                    slog::warn!(
                        logger,
                        "response item from elasticsearch contained unknown or duplicate event id; ignoring";
                        "response_item" => ?response_item,
                    );
                    continue;
                };

                // Create the submission result depending on whether an error occurred or not
                let submission_result = match &action.error {
                    Some(err) => Err(Failure {
                        status: Status::internal("Elasticsearch failed index operation for event"),
                        internal_details: format!("error object: {:?}", err),
                        correlation_id: self.correlation_id,
                    }),
                    None => Ok(self.correlation_id),
                };

                // Notify the submitter
                if let Err(send_err) = event.notifier.send(submission_result) {
                    slog::warn!(
                        logger,
                        "sending submission result to notifier failed; ignoring";
                        "error" => ?send_err,
                    );
                }
            }
        }
    }
}

/// Attempts to unwrap the `ResultItem` struct into the inner `ResultItemAction`
/// that should exist at the `index` field since the original actions were index operations.
fn unwrap_index_action<'a, 'b>(
    response_item: &'a crate::elasticsearch_api::bulk::ResultItem,
    logger: &'b Logger,
) -> Option<&'a crate::elasticsearch_api::bulk::ResultItemAction> {
    match &response_item.index {
        Some(action) => Some(action),
        None => {
            slog::warn!(
                logger,
                "response item from elasticsearch missing 'index' action field, ignoring";
                "response_item" => ?response_item,
            );

            None
        }
    }
}
