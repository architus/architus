pub mod inputs;
pub mod json;

use crate::config::{Configuration, GraphQL as GraphQLConfig};
use crate::elasticsearch_api::search::{HitsTotal, Response as SearchResponse};
use crate::graphql::inputs::{EventFilterInput, EventSortInput};
use crate::stored_event::StoredEvent;
use elasticsearch::{Elasticsearch, SearchParts};
use juniper::{graphql_value, EmptyMutation, EmptySubscription, FieldError, FieldResult, RootNode};
use log::debug;
use serde_json::json;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

/// Defines the context factory for search requests
#[derive(Clone)]
pub struct SearchProvider {
    base_context: Arc<Context>,
    schema: Arc<Schema>,
}

impl SearchProvider {
    pub fn new(elasticsearch: &Arc<Elasticsearch>, config: &Configuration) -> Self {
        Self {
            base_context: Arc::new(Context {
                elasticsearch: Arc::clone(elasticsearch),
                config: Arc::new(config.graphql.clone()),
                index: Arc::from(config.log_index.as_str()),
                guild_id: None,
                channel_whitelist: None,
            }),
            schema: Arc::new(Schema::new(
                Query,
                EmptyMutation::<Context>::new(),
                EmptySubscription::<Context>::new(),
            )),
        }
    }

    pub fn schema(&self) -> Arc<Schema> {
        Arc::clone(&self.schema)
    }

    pub fn context(&self, guild_id: Option<u64>, channel_whitelist: Option<Vec<u64>>) -> Context {
        let mut context = (*self.base_context).clone();
        context.guild_id = guild_id;
        context.channel_whitelist = channel_whitelist;
        context
    }
}

/// The root GraphQL schema type,
/// including the query root (and empty mutations/subscriptions)
pub type Schema = RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;

/// Defines the context object that is passed to query methods
#[derive(Clone)]
pub struct Context {
    elasticsearch: Arc<Elasticsearch>,
    config: Arc<GraphQLConfig>,
    index: Arc<str>,
    guild_id: Option<u64>,
    channel_whitelist: Option<Vec<u64>>,
}

impl juniper::Context for Context {}

/// Defines the GraphQL query root
pub struct Query;

#[juniper::graphql_object(context = Context)]
impl Query {
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
            .map(usize::try_from)
            .transpose()
            .map_err(|original| format!("Invalid value given to `limit`: {}", original))?
            .unwrap_or(context.config.default_page_size);
        let after = after
            .map(usize::try_from)
            .transpose()
            .map_err(|original| format!("Invalid value given to `after`: {}", original))?
            .unwrap_or(0);

        // Make sure the after & limit are valid
        if limit > context.config.max_page_size {
            return Err(format!(
                "Invalid value given to `limit`: {} > max ({})",
                limit, context.config.max_page_size
            )
            .into());
        }
        if limit + after > context.config.max_pagination_amount {
            return Err(format!(
                "Invalid values given to `limit` and `after`: limit + after ({}) > max ({})",
                limit + after,
                context.config.max_pagination_amount
            )
            .into());
        }

        // Resolve the filter and sort expressions into the Elasticsearch search DSL
        let mut elasticsearch_params = ElasticsearchParams::from_inputs(filter, sort)?;

        // Add in the guild id filter
        if let Some(guild_id) = context.guild_id.as_ref() {
            elasticsearch_params.add_filter(json!({"match": {"guild_id": *guild_id}}));
        }

        // Add in the channel id filter
        if let Some(channel_id_whitelist) = context.channel_whitelist.as_ref() {
            elasticsearch_params.add_filter(json!({
                "bool": {
                    "minimum_should_match": 1,
                    "should": channel_id_whitelist.clone(),
                },
            }));
        }

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

        // Reverse the sorts so that the most recently applied has the highest priority
        let reversed_sorts = elasticsearch_params
            .sort
            .map(|s| s.into_iter().rev().collect::<serde_json::Value>());

        // Send the Elasticsearch search request
        let body = json!({
            "from": after,
            "size": limit,
            "sort": reversed_sorts,
            "query": {
                "bool": {
                    "filter": elasticsearch_params.filter,
                    "must_not": elasticsearch_params.anti_filter,
                }
            }
        });
        debug!("Sending search body to Elasticsearch: {:?}", body);
        let index = [context.index.as_ref()];
        let send_future = context
            .elasticsearch
            .search(SearchParts::Index(&index))
            .body(body)
            .send();
        let query_time = architus_id::time::millisecond_ts();
        let response = send_future.await?;

        let response_body = response.json::<serde_json::Value>().await?;
        debug!("Received response from Elasticsearch: {:?}", response_body);
        let search_result: SearchResponse<StoredEvent> = serde_json::from_value(response_body)?;
        let total = search_result.hits.total;
        let nodes = search_result
            .hits
            .hits
            .into_iter()
            .map(|h| h.source)
            .collect::<_>();

        Ok(EventConnection {
            total,
            nodes,
            previous_snapshot,
            query_time,
            after,
            limit,
            max_pagination_amount: context.config.max_pagination_amount,
        })
    }
}

/// Connection object that allows consumers to traverse the log graph
pub struct EventConnection {
    total: HitsTotal,
    nodes: Vec<StoredEvent>,
    previous_snapshot: Option<SnapshotToken>,
    query_time: u64,
    after: usize,
    limit: usize,
    max_pagination_amount: usize,
}

#[juniper::graphql_object]
impl EventConnection {
    fn nodes(&self) -> &Vec<StoredEvent> {
        &self.nodes
    }

    fn page_info(&self) -> PageInfo {
        let current_page = self.after / self.limit;
        let previous_page_start = self.after.saturating_sub(self.limit);
        let next_page_start = self.after.saturating_add(self.limit);
        let next_page_end = next_page_start.saturating_add(self.limit);
        let total_count = usize::try_from(self.total.value).unwrap_or(0);
        let total_pageable = total_count - (self.after % self.limit);
        let page_count = (total_pageable.saturating_sub(1) / self.limit).saturating_add(1);
        PageInfo {
            current_page: i32::try_from(current_page).unwrap_or(i32::max_value()),
            has_previous_page: current_page > 0 && previous_page_start <= total_count,
            has_next_page: next_page_end <= self.max_pagination_amount
                && next_page_start < total_count,
            item_count: i32::try_from(self.nodes.len()).unwrap_or(i32::max_value()),
            page_count: i32::try_from(page_count).unwrap_or(i32::max_value()),
            per_page: i32::try_from(self.limit).unwrap_or(i32::max_value()),
            total_count: i32::try_from(total_count).unwrap_or(i32::max_value()),
        }
    }

    fn snapshot(&self) -> SnapshotToken {
        // Turn the query time Unix timestamp into a snowflake-encoded ID,
        // and use that as an inclusive upper bound for the snapshot token
        self.previous_snapshot
            .as_ref()
            .cloned()
            .unwrap_or_else(|| SnapshotToken(architus_id::id_bound_from_ts(self.query_time)))
    }
}

/// Includes metadata about a search's pagination
#[derive(juniper::GraphQLObject)]
pub struct PageInfo {
    current_page: i32,
    has_previous_page: bool,
    has_next_page: bool,
    item_count: i32,
    page_count: i32,
    per_page: i32,
    total_count: i32,
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
impl<S> juniper::GraphQLScalar for SnapshotToken
where
    S: juniper::ScalarValue,
{
    fn resolve(&self) -> juniper::Value {
        juniper::Value::scalar(self.to_string())
    }

    fn from_input_value(v: &juniper::InputValue) -> Option<SnapshotToken> {
        v.as_scalar_value()
            .and_then(juniper::ScalarValue::as_str)
            .and_then(|s| FromStr::from_str(s).ok())
    }

    fn from_str(value: juniper::parser::ScalarToken) -> juniper::ParseScalarResult<S> {
        <String as juniper::ParseScalarValue<S>>::from_str(value)
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
        Ok(Self(id))
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
    fn from_inputs(
        filter: Option<EventFilterInput>,
        sort: Option<EventSortInput>,
    ) -> FieldResult<Self> {
        let mut params = Self::new();
        params.add_sort(json!({"id": {"order": "desc"}}));
        filter.to_elasticsearch(&mut params)?;
        sort.to_elasticsearch(&mut params)?;
        Ok(params)
    }
}

trait QueryInput {
    fn to_elasticsearch(&self, params: &mut ElasticsearchParams) -> FieldResult<()>;
}

impl<T: QueryInput> QueryInput for Option<T> {
    fn to_elasticsearch(&self, params: &mut ElasticsearchParams) -> FieldResult<()> {
        if let Some(inner_value) = self {
            inner_value.to_elasticsearch(params)?;
        }

        Ok(())
    }
}
