//! Convenience wrappers around Elasticsearch

pub mod api_bindings;
pub mod filters;

use crate::config::Configuration;
use anyhow::Context as _;
use elasticsearch::auth::Credentials;
use elasticsearch::http::transport::{SingleNodeConnectionPool, TransportBuilder};
use elasticsearch::http::{StatusCode, Url};
use elasticsearch::{Elasticsearch, Error as LibError, SearchParts};
use serde::de::DeserializeOwned;
use slog::Logger;
use thiserror::Error;

/// Wrapped Elasticsearch client struct
pub struct Client {
    inner: Elasticsearch,
    logger: Logger,
}

/// Instantiates a new client.
/// Note: returning Ok(client) from this function
/// does not guarantee that the server is reachable;
/// client.ping() should be called to ensure this is the case.
pub fn new_client(config: &Configuration, logger: Logger) -> anyhow::Result<Client> {
    let url = &config.elasticsearch.url;
    let parsed_url = Url::parse(url).context("could not parse Elasticsearch URL")?;
    let connection_pool = SingleNodeConnectionPool::new(parsed_url);
    let mut builder = TransportBuilder::new(connection_pool);

    // Add in user authentication if configured
    if !config.elasticsearch.auth_username.is_empty() {
        builder = builder.auth(Credentials::Basic(
            config.elasticsearch.auth_username.clone(),
            config.elasticsearch.auth_password.clone(),
        ));
    }

    let transport = builder
        .build()
        .context("could not build Elasticsearch transport")?;
    let elasticsearch = Elasticsearch::new(transport);

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
    /// Pings the remote Elasticsearch,
    /// returning `Ok(())` if the ping was successful.
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

/// Represents the constructed Elasticsearch filter + sort object.
/// The filter/anti-filter map to the Bool query in the ES Query DSL:
/// `https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-bool-query.html`
/// The sort field maps to the `sort` param in the ES Query DSL:
/// `https://www.elastic.co/guide/en/elasticsearch/reference/7.10/sort-search-results.html`
#[derive(Debug, Default, Clone)]
pub struct SearchParams {
    pub after: usize,
    pub limit: usize,
    pub terminate_after: Option<usize>,
    /// All filter clauses in an Elasticsearch boolean query clause.
    /// Specifically, this clause gets converted to the `filter` sub-field:
    /// `https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-bool-query.html`
    pub filters: Vec<serde_json::Value>,
    /// All negative filter clauses in an Elasticsearch boolean query clause.
    /// Specifically, this clause gets converted to the `must_not` sub-field:
    /// `https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-bool-query.html`
    pub anti_filters: Vec<serde_json::Value>,
    /// All sort clauses in an Elasticsearch boolean query clause.
    pub sorts: Vec<serde_json::Value>,
}

impl SearchParams {
    pub fn new() -> Self {
        Self::default()
    }

    /// Converts the search params into the Elasticsearch query DSL
    pub fn into_search_body(self) -> serde_json::Value {
        let Self {
            after,
            limit,
            terminate_after,
            filters,
            anti_filters,
            sorts,
        } = self;

        // Reverse the sorts so that the most recently applied has the highest priority
        let reversed_sorts = sorts.into_iter().rev().collect::<Vec<_>>();

        serde_json::json!({
            "from": after,
            "size": limit,
            "terminate_after": terminate_after.unwrap_or(0),
            "sort": reversed_sorts,
            "query": {
                "bool": {
                    "filter": filters,
                    "must_not": anti_filters,
                }
            }
        })
    }
}

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("performing search operation failed")]
    Failure(#[source] LibError),
    #[error("failed to decode response body as JSON from elasticsearch")]
    JsonDecodeError(#[source] LibError),
    #[error("failed to decode response body JSON as expected response shape")]
    ResponseShapeDecodeError(#[source] serde_json::Error),
}

impl Client {
    pub async fn search<T>(
        &self,
        index: &str,
        search_params: SearchParams,
    ) -> Result<api_bindings::search::Response<T>, SearchError>
    where
        T: DeserializeOwned,
    {
        let index = [index];
        let body = search_params.into_search_body();
        slog::debug!(self.logger, "sending search body to Elasticsearch"; "body" => ?body);

        let search_future = self
            .inner
            .search(SearchParts::Index(&index))
            .body(body)
            .send();

        let response = search_future.await.map_err(SearchError::Failure)?;
        let response_body = response
            .json::<serde_json::Value>()
            .await
            .map_err(SearchError::JsonDecodeError)?;
        slog::debug!(self.logger, "received response from Elasticsearch"; "body" => ?response_body);

        // Try to decode the response body using the hand-made bindings
        let response_struct =
            serde_json::from_value::<api_bindings::search::Response<T>>(response_body)
                .map_err(SearchError::ResponseShapeDecodeError)?;
        Ok(response_struct)
    }
}
