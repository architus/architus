mod enums;
mod event;
mod inputs;
mod json;
mod page_info;
mod snapshot;

use crate::config::Configuration;
use crate::elasticsearch::SearchParams;
use crate::graphql::event::LogEvent;
use crate::graphql::inputs::QueryInput;
use crate::graphql::page_info::{PageInfo, PageInfoInputs};
use crate::rpc::logs::event::{AgentSpecialType, EntityType, EventOrigin, EventType};
use juniper::{graphql_value, EmptyMutation, EmptySubscription, FieldError, FieldResult, RootNode};
use serde_json::json;
use slog::Logger;
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// The root GraphQL schema type,
/// including the query root (and empty mutations/subscriptions)
pub type Schema = RootNode<'static, Query, EmptyMutation<Context>, EmptySubscription<Context>>;

/// Defines the context object that is passed to query methods
#[derive(Clone)]
pub struct Context {
    elasticsearch: Arc<crate::elasticsearch::Client>,
    config: Arc<Configuration>,
    logger: Logger,

    // Search parameters
    guild_id: u64,
    channel_allowlist: Option<Vec<u64>>,
}

impl juniper::Context for Context {}

impl Context {
    fn apply_guild_id_filter(&self, params: &mut SearchParams) {
        params
            .filters
            .push(json!({"term": {"inner.guild_id": self.guild_id}}));
    }

    fn apply_channel_id_filter(&self, params: &mut SearchParams) {
        if let Some(channel_id_allowlist) = self.channel_allowlist.as_ref() {
            let channel_id_allowlist_dsl_should = channel_id_allowlist
                .iter()
                .map(|channel_id| json!({"term": {"inner.channel_id": channel_id}}))
                .collect::<Vec<_>>();
            params.filters.push(json!({
                "bool": {
                    "minimum_should_match": 1,
                    "should": channel_id_allowlist_dsl_should,
                },
            }));
        }
    }
}

/// Defines the context factory for search requests
#[derive(Clone)]
pub struct SearchProvider {
    elasticsearch: Arc<crate::elasticsearch::Client>,
    config: Arc<Configuration>,
    schema: Arc<Schema>,
}

impl SearchProvider {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(
        elasticsearch: Arc<crate::elasticsearch::Client>,
        config: Arc<Configuration>,
    ) -> Self {
        Self {
            elasticsearch,
            config,
            schema: Arc::new(Schema::new(
                Query,
                EmptyMutation::<Context>::new(),
                EmptySubscription::<Context>::new(),
            )),
        }
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    pub fn context(
        &self,
        guild_id: u64,
        channel_allowlist: Option<Vec<u64>>,
        logger: Logger,
    ) -> Context {
        Context {
            guild_id,
            channel_allowlist,
            logger,
            elasticsearch: Arc::clone(&self.elasticsearch),
            config: Arc::clone(&self.config),
        }
    }
}

/// Defines the GraphQL query root
pub struct Query;

#[juniper::graphql_object(context = Context)]
impl Query {
    /// Queries for a single event in the log store,
    /// returning the first event if found
    async fn event(
        context: &Context,
        id: Option<crate::graphql::inputs::IdFilterInput>,
        timestamp: Option<crate::graphql::inputs::UIntStringFilterInput>,
        origin: Option<crate::graphql::inputs::EnumFilterInput>,
        r#type: Option<crate::graphql::inputs::EnumFilterInput>,
        guild_id: Option<crate::graphql::inputs::UIntStringFilterInput>,
        reason: Option<crate::graphql::inputs::TextFilterInput>,
        audit_log_id: Option<crate::graphql::inputs::UIntStringOptionFilterInput>,
        channel_id: Option<crate::graphql::inputs::UIntStringOptionFilterInput>,
        agent_id: Option<crate::graphql::inputs::UIntStringOptionFilterInput>,
        agent_type: Option<crate::graphql::inputs::EnumFilterInput>,
        agent_special_type: Option<crate::graphql::inputs::EnumFilterInput>,
        agent_webhook_username: Option<crate::graphql::inputs::StringFilterInput>,
        subject_id: Option<crate::graphql::inputs::UIntStringOptionFilterInput>,
        subject_type: Option<crate::graphql::inputs::EnumFilterInput>,
        auxiliary_id: Option<crate::graphql::inputs::UIntStringOptionFilterInput>,
        auxiliary_type: Option<crate::graphql::inputs::EnumFilterInput>,
        content: Option<crate::graphql::inputs::TextFilterInput>,
        content_metadata: Option<ContentMetadataFilterInput>,
    ) -> FieldResult<Option<LogEvent>> {
        let event_filter_input = EventFilterInput {
            id,
            timestamp,
            origin,
            r#type,
            guild_id,
            reason,
            audit_log_id,
            channel_id,
            agent_id,
            agent_type,
            agent_special_type,
            agent_webhook_username,
            subject_id,
            subject_type,
            auxiliary_id,
            auxiliary_type,
            content,
            content_metadata,
        };

        // For the single query, use a limit of 1 and a "terminate_after" of 1
        let mut search_params = SearchParams::new();
        search_params.limit = 1;
        search_params.terminate_after = Some(1);

        // Resolve the filter expressions into the Elasticsearch search DSL
        event_filter_input.to_elasticsearch(&mut search_params)?;
        context.apply_guild_id_filter(&mut search_params);
        context.apply_channel_id_filter(&mut search_params);

        let query_time: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
            .try_into()
            .expect("System time could not fit into u64");
        let index = &context.config.elasticsearch.index;
        let response = context
            .elasticsearch
            .search::<LogEvent>(index, search_params)
            .await
            .map_err(|err| {
                slog::warn!(
                    context.logger,
                    "an error occurred while processing search query";
                    "graphql_query" => "event",
                    "error" => ?err,
                );
                err
            })?;
        let node = response.hits.hits.into_iter().map(|h| h.source).next();

        slog::info!(
            context.logger,
            "successfully ran Elasticsearch query";
            "graphql_query" => "event",
            "query_time" => query_time,
            "took" => response.took.unwrap_or(0),
            "found" => node.is_some(),
            "index" => index,
        );

        Ok(node)
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
        // Build the elasticsearch search params DSL from the GraphQL inputs
        let mut search_params = SearchParams::new();
        search_params
            .sorts
            .push(json!({"inner.timestamp": {"order": "desc"}}));

        // Resolve the limit and after fields
        let limit = limit
            .map(usize::try_from)
            .transpose()
            .map_err(|original| format!("Invalid value given to `limit`: {}", original))?
            .unwrap_or(context.config.graphql.default_page_size);
        let after = after
            .map(usize::try_from)
            .transpose()
            .map_err(|original| format!("Invalid value given to `after`: {}", original))?
            .unwrap_or(0);

        // Make sure the after & limit are valid
        if limit > context.config.graphql.max_page_size {
            return Err(format!(
                "Invalid value given to `limit`: {} > max ({})",
                limit, context.config.graphql.max_page_size
            )
            .into());
        }
        if limit + after > context.config.graphql.max_pagination_amount {
            return Err(format!(
                "Invalid values given to `limit` and `after`: limit + after ({}) > max ({})",
                limit + after,
                context.config.graphql.max_pagination_amount
            )
            .into());
        }

        // Add the after & limit to the search params
        search_params.after = after;
        search_params.limit = limit;

        // Resolve the filter and sort expressions into the Elasticsearch search DSL
        filter.to_elasticsearch(&mut search_params)?;
        sort.to_elasticsearch(&mut search_params)?;
        context.apply_guild_id_filter(&mut search_params);
        context.apply_channel_id_filter(&mut search_params);

        // Add in the snapshot field if given
        let mut previous_snapshot = None;
        if let Some(raw_snapshot) = snapshot {
            match snapshot::Token::from_str(&raw_snapshot) {
                Ok(token) => {
                    // Add the filter corresponding to the snapshot token,
                    // where we only take log events with an ingestion timestamp
                    // less than the snapshot token.
                    // This provides some level of stability to the results,
                    // which helps with stable pagination.
                    search_params.filters.push(json!({
                        "range": {
                            "ingestion_timestamp": { "lte": token.0 }
                        },
                    }));
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

        let query_time: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis()
            .try_into()
            .expect("System time could not fit into u64");
        let index = &context.config.elasticsearch.index;
        let response = context
            .elasticsearch
            .search::<LogEvent>(index, search_params)
            .await
            .map_err(|err| {
                slog::warn!(
                    context.logger,
                    "an error occurred while processing search query";
                    "graphql_query" => "allEvent",
                    "error" => ?err,
                );
                err
            })?;
        let count = response.hits.hits.len();
        let total = usize::try_from(response.hits.total.value).unwrap_or(0_usize);
        let nodes = response
            .hits
            .hits
            .into_iter()
            .map(|h| h.source)
            .collect::<_>();

        slog::info!(
            context.logger,
            "successfully ran Elasticsearch query";
            "graphql_query" => "allEvent",
            "query_time" => query_time,
            "took" => response.took.unwrap_or(0),
            "count" => count,
            "total" => total,
            "index" => index,
            "after" => after,
            "limit" => limit,
        );

        Ok(EventConnection {
            nodes,
            previous_snapshot,
            query_time,
            query_duration: response
                .took
                .and_then(|t| i32::try_from(t).ok())
                .unwrap_or(0_i32),
            page_info: PageInfo::new(PageInfoInputs {
                count,
                total,
                after,
                limit,
                max_pagination: context.config.graphql.max_pagination_amount,
            }),
        })
    }
}

/// Connection object that allows consumers to traverse the log graph
pub struct EventConnection {
    nodes: Vec<LogEvent>,
    previous_snapshot: Option<snapshot::Token>,
    query_duration: i32,
    query_time: u64,
    page_info: PageInfo,
}

#[juniper::graphql_object]
impl EventConnection {
    fn nodes(&self) -> &Vec<LogEvent> {
        &self.nodes
    }

    fn snapshot(&self) -> snapshot::Token {
        self.previous_snapshot
            .as_ref()
            .cloned()
            .unwrap_or_else(|| snapshot::Token(self.query_time))
    }

    /// The number of milliseconds that it took to execute the query
    fn query_duration(&self) -> i32 {
        std::cmp::max(self.query_duration, 0)
    }

    /// The Unix millisecond timestamp that the query started at
    fn query_time(&self) -> String {
        self.query_time.to_string()
    }

    fn page_info(&self) -> &PageInfo {
        &self.page_info
    }
}

/// Filter input object that allows for filtering the results of `allEvent`
#[derive(juniper::GraphQLInputObject)]
pub struct EventFilterInput {
    id: Option<crate::graphql::inputs::IdFilterInput>,
    timestamp: Option<crate::graphql::inputs::UIntStringFilterInput>,
    origin: Option<crate::graphql::inputs::EnumFilterInput>,
    r#type: Option<crate::graphql::inputs::EnumFilterInput>,
    guild_id: Option<crate::graphql::inputs::UIntStringFilterInput>,
    reason: Option<crate::graphql::inputs::TextFilterInput>,
    audit_log_id: Option<crate::graphql::inputs::UIntStringOptionFilterInput>,
    channel_id: Option<crate::graphql::inputs::UIntStringOptionFilterInput>,
    agent_id: Option<crate::graphql::inputs::UIntStringOptionFilterInput>,
    agent_type: Option<crate::graphql::inputs::EnumFilterInput>,
    agent_special_type: Option<crate::graphql::inputs::EnumFilterInput>,
    agent_webhook_username: Option<crate::graphql::inputs::StringFilterInput>,
    subject_id: Option<crate::graphql::inputs::UIntStringOptionFilterInput>,
    subject_type: Option<crate::graphql::inputs::EnumFilterInput>,
    auxiliary_id: Option<crate::graphql::inputs::UIntStringOptionFilterInput>,
    auxiliary_type: Option<crate::graphql::inputs::EnumFilterInput>,
    content: Option<crate::graphql::inputs::TextFilterInput>,
    content_metadata: Option<ContentMetadataFilterInput>,
}

impl QueryInput for EventFilterInput {
    fn to_elasticsearch(&self, params: &mut SearchParams) -> FieldResult<()> {
        self.id.to_elasticsearch(params)?;
        self.timestamp
            .as_ref()
            .map(|filter| filter.apply(params, "inner.timestamp"))
            .transpose()?;
        self.origin
            .as_ref()
            .map(|filter| filter.apply::<EventOrigin>(params, "inner.origin"))
            .transpose()?;
        self.r#type
            .as_ref()
            .map(|filter| filter.apply::<EventType>(params, "inner.type"))
            .transpose()?;
        self.guild_id
            .as_ref()
            .map(|filter| filter.apply(params, "inner.guild_id"))
            .transpose()?;
        self.reason
            .as_ref()
            .map(|filter| filter.apply(params, "inner.reason"))
            .transpose()?;
        self.audit_log_id
            .as_ref()
            .map(|filter| filter.apply(params, "inner.audit_log_id"))
            .transpose()?;
        self.channel_id
            .as_ref()
            .map(|filter| filter.apply(params, "inner.channel_id"))
            .transpose()?;
        self.agent_id
            .as_ref()
            .map(|filter| filter.apply(params, "inner.agent_id"))
            .transpose()?;
        self.agent_type
            .as_ref()
            .map(|filter| filter.apply::<EntityType>(params, "inner.agent_type"))
            .transpose()?;
        self.agent_special_type
            .as_ref()
            .map(|filter| filter.apply::<AgentSpecialType>(params, "inner.agent_special_type"))
            .transpose()?;
        self.agent_webhook_username
            .as_ref()
            .map(|filter| filter.apply(params, "inner.agent_webhook_username"))
            .transpose()?;
        self.subject_id
            .as_ref()
            .map(|filter| filter.apply(params, "inner.subject_id"))
            .transpose()?;
        self.subject_type
            .as_ref()
            .map(|filter| filter.apply::<EntityType>(params, "inner.subject_type"))
            .transpose()?;
        self.auxiliary_id
            .as_ref()
            .map(|filter| filter.apply(params, "inner.auxiliary_id"))
            .transpose()?;
        self.auxiliary_type
            .as_ref()
            .map(|filter| filter.apply::<EntityType>(params, "inner.auxiliary_type"))
            .transpose()?;
        self.content
            .as_ref()
            .map(|filter| filter.apply(params, "inner.content"))
            .transpose()?;
        self.content_metadata.to_elasticsearch(params)?;

        Ok(())
    }
}

/// Filter input object that allows for filtering the `content_metadata` sub-field
#[derive(juniper::GraphQLInputObject)]
pub struct ContentMetadataFilterInput {
    users_mentioned: Option<crate::graphql::inputs::UIntStringSetFilterInput>,
    channels_mentioned: Option<crate::graphql::inputs::UIntStringSetFilterInput>,
    roles_mentioned: Option<crate::graphql::inputs::UIntStringSetFilterInput>,
    emojis_used: Option<crate::graphql::inputs::StringSetFilterInput>,
    custom_emojis_used: Option<crate::graphql::inputs::UIntStringSetFilterInput>,
    custom_emoji_names_used: Option<crate::graphql::inputs::StringSetFilterInput>,
    url_stems: Option<crate::graphql::inputs::StringSetFilterInput>,
}

impl QueryInput for ContentMetadataFilterInput {
    fn to_elasticsearch(&self, params: &mut SearchParams) -> FieldResult<()> {
        self.users_mentioned
            .as_ref()
            .map(|filter| filter.apply(params, "inner.content_metadata.users_mentioned"))
            .transpose()?;
        self.channels_mentioned
            .as_ref()
            .map(|filter| filter.apply(params, "inner.content_metadata.channels_mentioned"))
            .transpose()?;
        self.roles_mentioned
            .as_ref()
            .map(|filter| filter.apply(params, "inner.content_metadata.roles_mentioned"))
            .transpose()?;
        self.emojis_used
            .as_ref()
            .map(|filter| filter.apply(params, "inner.content_metadata.emojis_used"))
            .transpose()?;
        self.custom_emojis_used
            .as_ref()
            .map(|filter| filter.apply(params, "inner.content_metadata.custom_emojis_used"))
            .transpose()?;
        self.custom_emoji_names_used
            .as_ref()
            .map(|filter| filter.apply(params, "inner.content_metadata.custom_emoji_names_used"))
            .transpose()?;
        self.url_stems
            .as_ref()
            .map(|filter| filter.apply(params, "inner.content_metadata.url_stems"))
            .transpose()?;

        Ok(())
    }
}

/// Sort input object that allows for sorting the results of `allEvent`
#[derive(juniper::GraphQLInputObject)]
pub struct EventSortInput {
    // TODO use real input fields
    id: Option<i32>,
}

impl QueryInput for EventSortInput {
    fn to_elasticsearch(&self, _params: &mut SearchParams) -> FieldResult<()> {
        // TODO implement
        Ok(())
    }
}
