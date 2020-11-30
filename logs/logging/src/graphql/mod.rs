pub mod json;

use crate::config::Configuration;
use crate::elasticsearch_api::search::Response as SearchResponse;
use crate::stored_event::StoredEvent;
use elasticsearch::{Elasticsearch, SearchParts};
use juniper::{EmptyMutation, EmptySubscription, FieldResult, RootNode};
use serde_json::json;
use std::convert::TryFrom;
use std::sync::Arc;

/// Defines the context factory for search requests
#[derive(Clone)]
pub struct SearchProvider {
    context: Arc<Context>,
    schema: Arc<Schema>,
}

impl SearchProvider {
    pub fn new(elasticsearch: &Arc<Elasticsearch>, _config: &Configuration) -> Self {
        Self {
            context: Arc::new(Context {
                elasticsearch: Arc::clone(elasticsearch),
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
}

impl juniper::Context for Context {}

/// Defines the GraphQL query root
pub struct Root;

#[juniper::graphql_object(context = Context)]
impl Root {
    // TODO add filter fields
    /// Queries for a single event in the log store,
    /// returning the first event if found
    fn event() -> FieldResult<Option<StoredEvent>> {
        unimplemented!("not yet implemented")
    }

    /// Queries all event nodes in the log store,
    /// returning connection objects that allow consumers to traverse the log graph
    async fn all_events(context: &Context) -> FieldResult<EventConnection> {
        // Send the Elasticsearch search request
        // TODO optimize, this is a very basic initial implementation
        let response = context
            .elasticsearch
            .search(SearchParts::Index(&["events"]))
            .body(json!({
                "size": 1000,
                "query": {
                    "match_all": {}
                }
            }))
            .send()
            .await?;

        let response_body = response.json::<SearchResponse<StoredEvent>>().await?;
        let total_count = i32::try_from(response_body.hits.total.value).unwrap_or(0);
        let nodes = response_body
            .hits
            .hits
            .into_iter()
            .map(|h| h.source)
            .collect::<_>();

        Ok(EventConnection { total_count, nodes })
    }
}

/// Connection object that allows consumers to traverse the log graph
pub struct EventConnection {
    total_count: i32,
    nodes: Vec<StoredEvent>,
}

#[juniper::graphql_object]
impl EventConnection {
    fn total_count(&self) -> i32 {
        self.total_count
    }

    fn nodes(&self) -> &Vec<StoredEvent> {
        &self.nodes
    }
}
