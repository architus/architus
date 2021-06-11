use crate::graphql::json::GraphQLJson;
use crate::proto::logs::event::{
    AgentSpecialType, EntityType, Event, EventOrigin, EventSource, EventType,
};
use lazy_static::lazy_static;
use ref_cast::RefCast;
use serde::Deserialize;

/// Represents the JSON-serializable version of the stored Elasticsearch log event
/// See lib/ipc/proto/logs/event.proto for the original definition of this struct
#[derive(Debug, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct StoredEvent(Event);

// const StoredEventSource

// Define custom resolvers for most fields
// Reasons to do this are:
// - to convert all `u64`'s
//   into `String`'s because GraphQL/JSON/JavaScript doesn't do non-`i32` values well
// - resolve the source fields into parsed JSON
#[juniper::graphql_object(name = "Event")]
impl StoredEvent {
    fn id(&self) -> String {
        // This field will always be present
        self.0.id.to_string()
    }

    fn timestamp(&self) -> String {
        // This field will always be present
        self.0.timestamp.to_string()
    }

    fn source(&self) -> &StoredEventSource {
        lazy_static! {
            static ref EMPTY_SOURCE: StoredEventSource = StoredEventSource(EventSource {
                gateway: String::new(),
                audit_log: String::new(),
                internal: String::new(),
            });
        }

        match &self.0.source {
            None => &EMPTY_SOURCE,
            Some(source) => StoredEventSource::ref_cast(source),
        }
    }

    fn origin(&self) -> EventOrigin {
        EventOrigin::from_i32(self.0.origin).unwrap_or(EventOrigin::Unknown)
    }

    fn r#type(&self) -> EventType {
        EventType::from_i32(self.0.r#type).unwrap_or(EventType::Unknown)
    }

    fn guild_id(&self) -> String {
        // This field will always be present
        self.0.guild_id.to_string()
    }

    fn reason(&self) -> Option<&str> {
        match self.0.reason.as_str() {
            "" => None,
            reason => Some(reason),
        }
    }

    fn audit_log_id(&self) -> Option<String> {
        match self.0.audit_log_id {
            0 => None,
            id => Some(id.to_string()),
        }
    }

    fn channel_id(&self) -> Option<String> {
        match self.0.channel_id {
            0 => None,
            id => Some(id.to_string()),
        }
    }

    fn agent_id(&self) -> Option<String> {
        match self.0.agent_id {
            0 => None,
            id => Some(id.to_string()),
        }
    }

    fn agent_type(&self) -> EntityType {
        EntityType::from_i32(self.0.agent_type).unwrap_or(EntityType::None)
    }

    fn agent_special_type(&self) -> AgentSpecialType {
        AgentSpecialType::from_i32(self.0.agent_special_type).unwrap_or(AgentSpecialType::Default)
    }

    fn subject_id(&self) -> Option<String> {
        match self.0.subject_id {
            0 => None,
            id => Some(id.to_string()),
        }
    }

    fn subject_type(&self) -> EntityType {
        EntityType::from_i32(self.0.subject_type).unwrap_or(EntityType::None)
    }

    fn auxiliary_id(&self) -> Option<String> {
        match self.0.auxiliary_id {
            0 => None,
            id => Some(id.to_string()),
        }
    }

    fn auxiliary_type(&self) -> EntityType {
        EntityType::from_i32(self.0.auxiliary_type).unwrap_or(EntityType::None)
    }

    fn content(&self) -> Option<&str> {
        match self.0.content.as_str() {
            "" => None,
            reason => Some(reason),
        }
    }
}

#[derive(Debug, Deserialize, RefCast)]
#[serde(transparent)]
#[repr(transparent)]
pub struct StoredEventSource(EventSource);

#[juniper::graphql_object(name = "EventSource")]
impl StoredEventSource {
    fn gateway(&self) -> Option<GraphQLJson> {
        match self.0.gateway.as_str() {
            "" => None,
            raw_json => serde_json::from_str(raw_json).ok(),
        }
    }

    fn audit_log(&self) -> Option<GraphQLJson> {
        match self.0.audit_log.as_str() {
            "" => None,
            raw_json => serde_json::from_str(raw_json).ok(),
        }
    }

    fn internal(&self) -> Option<GraphQLJson> {
        match self.0.internal.as_str() {
            "" => None,
            raw_json => serde_json::from_str(raw_json).ok(),
        }
    }
}
