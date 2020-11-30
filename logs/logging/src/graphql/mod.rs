pub mod json;

use crate::config::Configuration;
use crate::elasticsearch_api::search::Response as SearchResponse;
use crate::stored_event::StoredEvent;
use elasticsearch::{Elasticsearch, SearchParts};
use juniper::parser::ScalarToken;
use juniper::{
    graphql_value, EmptyMutation, EmptySubscription, FieldError, FieldResult, ParseScalarResult,
    ParseScalarValue, RootNode, ScalarValue,
};
use log::debug;
use serde_json::json;
use std::cmp;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

/// Defines the context factory for search requests
#[derive(Clone)]
pub struct SearchProvider {
    context: Arc<Context>,
    schema: Arc<Schema>,
}

impl SearchProvider {
    pub fn new(elasticsearch: &Arc<Elasticsearch>, config: &Configuration) -> Self {
        Self {
            context: Arc::new(Context {
                elasticsearch: Arc::clone(elasticsearch),
                default_limit: cmp::max(config.graphql.default_limit, 0),
            }),
            schema: Arc::new(Schema::new(
                Root,
                EmptyMutation::<Context>::new(),
                EmptySubscription::<Context>::new(),
            )),
        }
    }

    pub fn schema(&self) -> Arc<Schema> {
        Arc::clone(&self.schema)
    }

    pub fn context(&self) -> Arc<Context> {
        Arc::clone(&self.context)
    }
}

/// The root GraphQL schema type,
/// including the query root (and empty mutations/subscriptions)
pub type Schema = RootNode<'static, Root, EmptyMutation<Context>, EmptySubscription<Context>>;

/// Defines the context object that is passed to query methods
pub struct Context {
    elasticsearch: Arc<Elasticsearch>,
    /// Default limit of items to fetch if none is given
    default_limit: i32,
}

impl juniper::Context for Context {}

/// Defines the GraphQL query root
pub struct Root;

#[juniper::graphql_object(context = Context)]
impl Root {
    // TODO add filter fields as individual parameters and then construct `EventFilterInput`
    /// Queries for a single event in the log store,
    /// returning the first event if found
    fn event() -> FieldResult<Option<StoredEvent>> {
        unimplemented!("not yet implemented")
    }

    /// Queries all event nodes in the log store,
    /// returning connection objects that allow consumers to traverse the log graph
    async fn all_event(
        context: &Context,
        filter: Option<EventFilterInput>,
        sort: Option<EventSortInput>,
        snapshot: Option<String>,
        after: Option<i32>,
        limit: Option<i32>,
    ) -> FieldResult<EventConnection> {
        // Resolve the limit and after fields
        let limit = limit
            .map(|limit| {
                if limit > 0 {
                    Ok(limit)
                } else {
                    Err(format!("Invalid value given to `limit`: {}", limit))
                }
            })
            .transpose()?
            .unwrap_or(context.default_limit);
        let after = after
            .map(|after| {
                if after >= 0 {
                    Ok(limit)
                } else {
                    Err(format!("Invalid value given to `after`: {}", after))
                }
            })
            .transpose()?
            .unwrap_or(0);

        // Resolve the filter and sort expressions into the Elasticsearch search DSL
        let mut elasticsearch_params = ElasticsearchParams::from_inputs(filter, sort);

        // Add in the snapshot field if given
        let mut previous_snapshot = None;
        if let Some(raw_snapshot) = snapshot {
            match FromStr::from_str(&raw_snapshot) {
                Ok(token) => {
                    elasticsearch_params.add_snapshot_filter(&token);
                    previous_snapshot = Some(token);
                }
                Err(err) => {
                    let message = format!("{:?}", err);
                    return Err(FieldError::new(
                        "Bad snapshot token given",
                        graphql_value!({
                            "internal": message,
                        }),
                    ));
                }
            }
        }

        // Send the Elasticsearch search request
        let body = json!({
            "from": after,
            "size": limit,
            "sort": elasticsearch_params.sort,
            "query": {
                "bool": {
                    "filter": elasticsearch_params.filter,
                    "must_not": elasticsearch_params.anti_filter,
                }
            }
        });
        debug!("Sending search body to Elasticsearch: {:?}", body);
        let send_future = context
            .elasticsearch
            .search(SearchParts::Index(&["events"]))
            .body(body)
            .send();
        let query_time = architus_id::time::millisecond_ts();
        let response = send_future.await?;

        let response_body = response.json::<serde_json::Value>().await?;
        debug!("Received response from Elasticsearch: {:?}", response_body);
        let search_result: SearchResponse<StoredEvent> = serde_json::from_value(response_body)?;
        let total_count = i32::try_from(search_result.hits.total.value).unwrap_or(0);
        let nodes = search_result
            .hits
            .hits
            .into_iter()
            .map(|h| h.source)
            .collect::<_>();

        Ok(EventConnection {
            total_count,
            nodes,
            query_time,
            previous_snapshot,
        })
    }
}

/// Connection object that allows consumers to traverse the log graph
pub struct EventConnection {
    total_count: i32,
    nodes: Vec<StoredEvent>,
    previous_snapshot: Option<SnapshotToken>,
    query_time: u64,
}

#[juniper::graphql_object]
impl EventConnection {
    fn total_count(&self) -> i32 {
        self.total_count
    }

    fn nodes(&self) -> &Vec<StoredEvent> {
        &self.nodes
    }

    fn snapshot(&self) -> SnapshotToken {
        // Turn the query time Unix timestamp into a snowflake-encoded ID,
        // and use that as an inclusive upper bound for the snapshot token
        self.previous_snapshot
            .as_ref()
            .map(|t| t.clone())
            .unwrap_or_else(|| SnapshotToken(architus_id::id_bound_from_ts(self.query_time)))
    }
}

/// Represents an opaque snapshot token that just includes an inner ID.
/// Used for stateless pagination sessions where we want some stability in the results,
/// so it takes advantage of the chrono-sequential nature of Snowflake-encoded IDs
/// and the immutability of historical log entries to stabilize the results
/// by only including entries with an ID less than or equal to the token (if given)
#[derive(Debug, Clone)]
pub struct SnapshotToken(u64);

impl fmt::Display for SnapshotToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &base64::encode(self.0.to_be_bytes()))
    }
}

#[derive(Error, Debug)]
pub enum SnapshotParsingError {
    #[error("could not decode snapshot token base64: {0}")]
    FailedBase64Decode(base64::DecodeError),
    #[error("invalid {0} length for base64 sequence; expected 8 bytes")]
    InvalidLength(usize),
}

#[juniper::graphql_scalar(description = "SnapshotToken")]
impl<S> GraphQLScalar for SnapshotToken
where
    S: ScalarValue,
{
    fn resolve(&self) -> juniper::Value {
        juniper::Value::scalar(self.to_string())
    }

    fn from_input_value(v: &juniper::InputValue) -> Option<SnapshotToken> {
        v.as_scalar_value()
            .and_then(|v| v.as_str())
            .and_then(|s| FromStr::from_str(s).ok())
    }

    fn from_str(value: ScalarToken) -> ParseScalarResult<S> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
}

impl FromStr for SnapshotToken {
    type Err = SnapshotParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let base64_bytes = base64::decode(s).map_err(SnapshotParsingError::FailedBase64Decode)?;
        let id =
            u64::from_be_bytes(base64_bytes.try_into().map_err(|original: Vec<u8>| {
                SnapshotParsingError::InvalidLength(original.len())
            })?);
        Ok(SnapshotToken(id))
    }
}

/// Represents the constructed Elasticsearch filter + sort object.
/// The filter/anti-filter map to the Bool query in the ES Query DSL:
/// `https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-bool-query.html`
/// The sort field maps to the `sort` param in the ES Query DSL:
/// `https://www.elastic.co/guide/en/elasticsearch/reference/7.10/sort-search-results.html`
#[derive(Debug, Default, Clone)]
pub struct ElasticsearchParams {
    filter: Option<Vec<serde_json::Value>>,
    anti_filter: Option<Vec<serde_json::Value>>,
    sort: Option<Vec<serde_json::Value>>,
}

impl ElasticsearchParams {
    fn new() -> Self {
        Self::default()
    }

    /// Adds a negative filter clause to the Elasticsearch boolean query clause.
    /// Specifically, adds a clause to the `filter` sub-field:
    /// `https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-bool-query.html`
    fn add_filter(&mut self, filter: serde_json::Value) {
        match self.filter.as_mut() {
            Some(filters) => {
                filters.push(filter);
            }
            None => {
                self.filter = Some(vec![filter]);
            }
        }
    }

    /// Adds a negative filter clause to the Elasticsearch boolean query clause.
    /// Specifically, adds a clause to the `must_not` sub-field:
    /// `https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl-bool-query.html`
    fn add_anti_filter(&mut self, filter: serde_json::Value) {
        match self.anti_filter.as_mut() {
            Some(anti_filters) => {
                anti_filters.push(filter);
            }
            None => {
                self.anti_filter = Some(vec![filter]);
            }
        }
    }

    /// Adds a sort clause to the Elasticsearch query object.
    /// `https://www.elastic.co/guide/en/elasticsearch/reference/7.10/sort-search-results.html`
    fn add_sort(&mut self, sort: serde_json::Value) {
        match self.sort.as_mut() {
            Some(sorts) => {
                sorts.push(sort);
            }
            None => {
                self.sort = Some(vec![sort]);
            }
        }
    }

    /// Adds a snapshot filter, only including items with an ID leq
    /// to the given snapshot's inner value
    fn add_snapshot_filter(&mut self, snapshot: &SnapshotToken) {
        self.add_filter(json!({"range": {"id": {"lte": snapshot.0 }}}))
    }

    /// Converts the given input structs into the compound params
    fn from_inputs(_filter: Option<EventFilterInput>, _sort: Option<EventSortInput>) -> Self {
        // TODO implement
        let mut params = Self::new();
        params.add_sort(json!({"id": {"order": "desc"}}));
        params
    }
}

/// Filter input object that allows for filtering the results of `allEvent`
#[derive(juniper::GraphQLInputObject)]
pub struct EventFilterInput {
    // TODO use real input fields
    id: Option<i32>,
}

/// Sort input object that allows for sorting the results of `allEvent`
#[derive(juniper::GraphQLInputObject)]
pub struct EventSortInput {
    // TODO use real input fields
    id: Option<i32>,
}
