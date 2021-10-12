use crate::graphql::json::GraphQLJson;
use crate::rpc::logs::event::{
    AgentSpecialType, EntityType, Event, EventOrigin, EventSource, EventType,
};
use crate::rpc::logs_submission_schema::StoredEvent;
use lazy_static::lazy_static;
use ref_cast::RefCast;
use serde::Deserialize;

/// Wrapper around the JSON-serializable version of the stored Elasticsearch log event
/// See lib/protos/event.proto for the original definition of this struct
#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct LogEvent(StoredEvent);

impl LogEvent {
    const fn event(&self) -> Option<&Event> {
        self.0.inner.as_ref()
    }
}

// Define custom resolvers for most fields
// Reasons to do this are:
// - to convert all `u64`'s
//   into `String`'s because GraphQL/JSON/JavaScript doesn't do non-`i32` values well
// - resolve the source fields into parsed JSON
#[juniper::graphql_object(name = "Event")]
impl LogEvent {
    fn id(&self) -> String {
        // This field will always be present
        self.0.id.clone()
    }

    fn timestamp(&self) -> String {
        // This field will always be present
        self.event()
            .map(|e| e.timestamp.to_string())
            .unwrap_or_else(|| String::from(""))
    }

    fn source(&self) -> &Source {
        lazy_static! {
            static ref EMPTY_SOURCE: Source = Source(EventSource {
                gateway: String::new(),
                audit_log: String::new(),
                internal: String::new(),
            });
        }

        let source_option = self.event().and_then(|e| e.source.as_ref());
        match source_option {
            None => &EMPTY_SOURCE,
            Some(source) => Source::ref_cast(source),
        }
    }

    fn origin(&self) -> EventOrigin {
        self.event()
            .and_then(|e| EventOrigin::from_i32(e.origin))
            .unwrap_or(EventOrigin::Unknown)
    }

    fn r#type(&self) -> EventType {
        self.event()
            .and_then(|e| EventType::from_i32(e.r#type))
            .unwrap_or(EventType::Unknown)
    }

    fn guild_id(&self) -> String {
        // This field will always be present
        self.event()
            .map(|e| e.guild_id.to_string())
            .unwrap_or_else(|| String::from(""))
    }

    fn reason(&self) -> Option<&str> {
        match self.event().map(|e| e.reason.as_str()) {
            None | Some("") => None,
            Some(reason) => Some(reason),
        }
    }

    fn audit_log_id(&self) -> Option<String> {
        match self.event().map(|e| e.audit_log_id) {
            None | Some(0) => None,
            Some(id) => Some(id.to_string()),
        }
    }

    fn channel_id(&self) -> Option<String> {
        match self.event().map(|e| e.channel_id) {
            None | Some(0) => None,
            Some(id) => Some(id.to_string()),
        }
    }

    fn agent_id(&self) -> Option<String> {
        match self.event().map(|e| e.agent_id) {
            None | Some(0) => None,
            Some(id) => Some(id.to_string()),
        }
    }

    fn agent_type(&self) -> EntityType {
        self.event()
            .and_then(|e| EntityType::from_i32(e.agent_type))
            .unwrap_or(EntityType::None)
    }

    fn agent_special_type(&self) -> AgentSpecialType {
        self.event()
            .and_then(|e| AgentSpecialType::from_i32(e.agent_special_type))
            .unwrap_or(AgentSpecialType::Default)
    }

    fn subject_id(&self) -> Option<String> {
        match self.event().map(|e| e.subject_id) {
            None | Some(0) => None,
            Some(id) => Some(id.to_string()),
        }
    }

    fn subject_type(&self) -> EntityType {
        self.event()
            .and_then(|e| EntityType::from_i32(e.subject_type))
            .unwrap_or(EntityType::None)
    }

    fn auxiliary_id(&self) -> Option<String> {
        match self.event().map(|e| e.auxiliary_id) {
            None | Some(0) => None,
            Some(id) => Some(id.to_string()),
        }
    }

    fn auxiliary_type(&self) -> EntityType {
        self.event()
            .and_then(|e| EntityType::from_i32(e.auxiliary_type))
            .unwrap_or(EntityType::None)
    }

    fn content(&self) -> Option<&str> {
        match self.event().map(|e| e.content.as_str()) {
            None | Some("") => None,
            Some(reason) => Some(reason),
        }
    }
}

#[derive(Debug, Deserialize, RefCast)]
#[serde(transparent)]
#[repr(transparent)]
pub struct Source(EventSource);

#[juniper::graphql_object(name = "EventSource")]
impl Source {
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
