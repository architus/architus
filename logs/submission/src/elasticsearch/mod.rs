/// Convenience wrappers around Elasticsearch
pub mod api_bindings;

use crate::config::Configuration;
use ::elasticsearch::http::headers::HeaderMap;
use ::elasticsearch::http::transport::Transport;
use ::elasticsearch::http::{Method, StatusCode};
use ::elasticsearch::indices::IndicesCreateParts;
use ::elasticsearch::{BulkParts, Elasticsearch, Error as LibError};
use bytes::Bytes;
use serde::Serialize;
use slog::Logger;
use std::iter::IntoIterator;
use std::sync::Arc;
use thiserror::Error;

pub struct Client {
    inner: Elasticsearch,
    logger: Logger,
}

pub fn new_client(config: Arc<Configuration>, logger: Logger) -> Result<Client, LibError> {
    let es_path = &config.services.elasticsearch;
    let es_transport = Transport::single_node(es_path)?;
    let elasticsearch = Elasticsearch::new(es_transport);

    Ok(Client {
        inner: elasticsearch,
        logger,
    })
}

#[derive(Error, Debug)]
pub enum PingError {
    #[error("pinging elasticsearch failed")]
    Failed(#[source] LibError),
    #[error("pinging elasticsearch failed with a non-success status code {0}")]
    ErrorStatusCode(StatusCode),
}

impl Client {
    pub async fn ping(&self) -> Result<(), PingError> {
        let response = self.inner.ping().send().await.map_err(PingError::Failed)?;

        let status_code = response.status_code();
        if status_code.is_success() {
            Ok(())
        } else {
            Err(PingError::ErrorStatusCode(status_code))
        }
    }
}

#[derive(Clone, Debug)]
pub enum IndexStatus {
    CreatedSuccessfully,
    AlreadyExists,
}

#[derive(Error, Debug)]
pub enum EnsureIndexExistsError {
    #[error("ensuring the index exists failed")]
    Failed(#[source] LibError),
    #[error("failed to read response body from elasticsearch")]
    BodyReadFailure(#[source] LibError),
    #[error("ensuring the index exists failed with a non-success status code {0}")]
    ErrorStatusCode(StatusCode),
}

impl Client {
    pub async fn ensure_index_exists(
        &self,
        index: impl AsRef<str>,
        index_settings: Bytes,
    ) -> Result<IndexStatus, EnsureIndexExistsError> {
        let index_ref = index.as_ref();
        let create_parts = IndicesCreateParts::Index(index_ref).url();

        // Use the untyped send API so that the raw bytes can be used
        let create_future = self.inner.send(
            Method::Put,
            create_parts.as_ref(),
            HeaderMap::new(),
            Option::<&serde_json::Value>::None,
            Some(index_settings),
            None,
        );

        match create_future.await {
            Ok(response) => {
                let status_code = response.status_code();
                if status_code.is_success() {
                    Ok(IndexStatus::CreatedSuccessfully)
                } else {
                    let content = response
                        .text()
                        .await
                        .map_err(EnsureIndexExistsError::BodyReadFailure)?;

                    // Check to see if the index already existed (if so, it's fine)
                    let is_exist_error = if status_code == StatusCode::BAD_REQUEST {
                        content.contains("resource_already_exists_exception")
                    } else {
                        false
                    };

                    if is_exist_error {
                        Ok(IndexStatus::AlreadyExists)
                    } else {
                        Err(EnsureIndexExistsError::ErrorStatusCode(status_code))
                    }
                }
            }
            Err(err) => Err(EnsureIndexExistsError::Failed(err)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BulkOperation {
    action: Bytes,
    source: Option<Bytes>,
}

#[derive(Error, Debug)]
pub enum MakeBulkOperationError {
    #[error("failed to serialize action JSON object")]
    ActionSerializationFailure(#[source] serde_json::Error),
    #[error("failed to serialize source JSON object")]
    SourceSerializationFailure(#[source] serde_json::Error),
}

impl BulkOperation {
    pub fn index(
        id: impl Into<String>,
        document: impl Serialize,
    ) -> Result<Self, MakeBulkOperationError> {
        let id = id.into();

        // Create the "operation" JSON line using the ID
        let operation_json_value = serde_json::json!({"index": {"_id": id }});
        let action_buf = match serde_json::to_vec(&operation_json_value) {
            Ok(vec) => Bytes::from(vec),
            Err(err) => {
                return Err(MakeBulkOperationError::ActionSerializationFailure(err));
            }
        };

        // Create the document JSON line
        let source_buf = match serde_json::to_vec(&document) {
            Ok(vec) => Bytes::from(vec),
            Err(err) => {
                return Err(MakeBulkOperationError::SourceSerializationFailure(err));
            }
        };

        Ok(BulkOperation {
            action: action_buf,
            source: Some(source_buf),
        })
    }
}

#[derive(Clone, Debug)]
pub struct BulkStatus {
    pub took: i64,
    pub errors: bool,
    pub items: Vec<BulkItem>,
}

#[derive(Clone, Debug)]
pub enum BulkItem {
    Create(api_bindings::bulk::ResultItemAction),
    Delete(api_bindings::bulk::ResultItemAction),
    Index(api_bindings::bulk::ResultItemAction),
    Update(api_bindings::bulk::ResultItemAction),
}

impl BulkItem {
    pub fn id(&self) -> &String {
        match self {
            Self::Create(api_bindings::bulk::ResultItemAction { ref id, .. }) => id,
            Self::Delete(api_bindings::bulk::ResultItemAction { ref id, .. }) => id,
            Self::Index(api_bindings::bulk::ResultItemAction { ref id, .. }) => id,
            Self::Update(api_bindings::bulk::ResultItemAction { ref id, .. }) => id,
        }
    }
}

#[derive(Error, Debug)]
pub enum BulkError {
    #[error("performing bulk operation failed")]
    Failure(#[source] LibError),
    #[error("failed to decode response body from elasticsearch")]
    FailedToDecode(#[source] LibError),
}

impl Client {
    pub async fn bulk(
        &self,
        index: impl AsRef<str>,
        operations: impl IntoIterator<Item = &BulkOperation>,
    ) -> Result<BulkStatus, BulkError> {
        let index_ref = index.as_ref();

        // Collect the operation into a list of bytes
        let operations: Vec<Bytes> = operations
            .into_iter()
            .cloned()
            .flat_map(|op| match op.source {
                Some(source) => vec![op.action, source],
                None => vec![op.action],
            })
            .collect::<Vec<_>>();

        let bulk_future = self
            .inner
            .bulk(BulkParts::Index(index_ref))
            .body(operations)
            .send();
        match bulk_future.await {
            Ok(response) => {
                // Try to decode the response body using the hand-made bindings
                match response.json::<api_bindings::bulk::Response>().await {
                    Ok(response_struct) => Ok(self.convert_to_status(response_struct)),
                    Err(decode_err) => return Err(BulkError::FailedToDecode(decode_err)),
                }
            }
            Err(err) => return Err(BulkError::Failure(err)),
        }
    }

    fn convert_to_status(&self, response: api_bindings::bulk::Response) -> BulkStatus {
        let api_bindings::bulk::Response {
            took,
            errors,
            items: raw_items,
        } = response;

        let mut items = Vec::<BulkItem>::with_capacity(raw_items.len());
        for raw_item in raw_items {
            let mut already_had_action = false;
            if let Some(create_action) = raw_item.create {
                already_had_action = true;
                items.push(BulkItem::Create(create_action));
            }

            if let Some(delete_action) = raw_item.delete {
                if already_had_action {
                    slog::warn!(
                        self.logger,
                        "bulk response from elasticsearch contained more than one action in an item";
                        "last_action" => ?items.last(),
                        "this_action" => ?delete_action
                    );
                }

                already_had_action = true;
                items.push(BulkItem::Create(delete_action));
            }

            if let Some(index_action) = raw_item.index {
                if already_had_action {
                    slog::warn!(
                        self.logger,
                        "bulk response from elasticsearch contained more than one action in an item";
                        "last_action" => ?items.last(),
                        "this_action" => ?index_action
                    );
                }

                already_had_action = true;
                items.push(BulkItem::Create(index_action));
            }

            if let Some(update_action) = raw_item.update {
                if already_had_action {
                    slog::warn!(
                        self.logger,
                        "bulk response from elasticsearch contained more than one action in an item";
                        "last_action" => ?items.last(),
                        "this_action" => ?update_action
                    );
                }

                already_had_action = true;
                items.push(BulkItem::Create(update_action));
            }
        }

        BulkStatus {
            took,
            errors,
            items,
        }
    }
}
