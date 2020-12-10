use crate::graphql::json::GraphQLJson;
use crate::logging::Event;
use crate::logging::{EventOrigin, EventType};
use architus_id::HoarFrost;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use thiserror::Error;

/// Represents the JSON-serializable version of the stored Elasticsearch log event
/// See lib/ipc/proto/logging.proto for the original definition of this struct
#[derive(Debug, Deserialize, Serialize)]
pub struct StoredEvent {
    pub id: HoarFrost,
    pub timestamp: u64,
    pub source: StoredSource,
    pub origin: EventOrigin,
    pub event_type: EventType,
    pub guild_id: u64,
    #[serde(with = "id_option")]
    pub agent_id: Option<u64>,
    #[serde(with = "id_option")]
    pub subject_id: Option<u64>,
    #[serde(with = "id_option")]
    pub audit_log_id: Option<u64>,
    #[serde(with = "id_option")]
    pub channel_id: Option<u64>,
    #[serde(with = "string_option")]
    pub reason: Option<String>,
}

// Define custom resolvers for most fields
// The main reason for this is to convert all `u64`'s
// into `String`'s because GraphQL/JSON/JavaScript doesn't do non-`i32` values well
#[juniper::graphql_object(name = "Event")]
impl StoredEvent {
    fn id(&self) -> String {
        self.id.to_string()
    }

    fn timestamp(&self) -> String {
        self.timestamp.to_string()
    }

    fn source(&self) -> &StoredSource {
        &self.source
    }

    fn origin(&self) -> &EventOrigin {
        &self.origin
    }

    fn event_type(&self) -> &EventType {
        &self.event_type
    }

    fn guild_id(&self) -> String {
        self.guild_id.to_string()
    }

    fn agent_id(&self) -> Option<String> {
        self.agent_id.as_ref().map(ToString::to_string)
    }

    fn subject_id(&self) -> Option<String> {
        self.subject_id.as_ref().map(ToString::to_string)
    }

    fn audit_log_id(&self) -> Option<String> {
        self.audit_log_id.as_ref().map(ToString::to_string)
    }

    fn channel_id(&self) -> Option<String> {
        self.channel_id.as_ref().map(ToString::to_string)
    }

    fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

#[derive(Debug, Deserialize, Serialize, juniper::GraphQLObject)]
#[graphql(name = "EventSource")]
pub struct StoredSource {
    pub gateway: Option<GraphQLJson>,
    pub audit_log: Option<GraphQLJson>,
    pub internal: Option<GraphQLJson>,
}

#[derive(Error, Debug)]
pub enum ParsingError {
    #[error("an error occurred while deserializing the inner JSON: {0}")]
    SerdeError(serde_json::Error),
    #[error("no id was specified")]
    MissingId,
    #[error("no timestamp was specified")]
    MissingTimestamp,
    #[error("no guild id was specified")]
    MissingGuildId,
}

// Provide an implementation to convert the Event to a StoredEvent.
// Involves deserializing the inner JSON from the string so it can be embedded.
// Note that the protobuf definitions do not use the generic Struct message
// (from the Google well-known types) because it only supports f64 numbers,
// which may cause problems in the future.
// Instead, the internal JSON is sent as a string
impl TryFrom<Event> for StoredEvent {
    type Error = ParsingError;

    fn try_from(value: Event) -> Result<Self, Self::Error> {
        Ok(Self {
            id: HoarFrost(some_unless(value.id, 0).ok_or(ParsingError::MissingId)?),
            timestamp: some_unless(value.timestamp, 0).ok_or(ParsingError::MissingTimestamp)?,
            source: StoredSource {
                gateway: value
                    .source
                    .as_ref()
                    .map(|source| &source.gateway)
                    .filter(|json| !json.is_empty())
                    .map(|gateway| serde_json::from_str(gateway))
                    .transpose()
                    .map_err(ParsingError::SerdeError)?,
                audit_log: value
                    .source
                    .as_ref()
                    .map(|source| &source.audit_log)
                    .filter(|json| !json.is_empty())
                    .map(|audit_log| serde_json::from_str(audit_log))
                    .transpose()
                    .map_err(ParsingError::SerdeError)?,
                internal: value
                    .source
                    .as_ref()
                    .map(|source| &source.internal)
                    .filter(|json| !json.is_empty())
                    .map(|internal| serde_json::from_str(internal))
                    .transpose()
                    .map_err(ParsingError::SerdeError)?,
            },
            origin: EventOrigin::from_i32(value.origin).unwrap_or(EventOrigin::Unknown),
            event_type: EventType::from_i32(value.event_type).unwrap_or(EventType::Unknown),
            guild_id: some_unless(value.guild_id, 0).ok_or(ParsingError::MissingGuildId)?,
            agent_id: some_unless(value.agent_id, 0),
            subject_id: some_unless(value.subject_id, 0),
            audit_log_id: some_unless(value.audit_log_id, 0),
            channel_id: some_unless(value.channel_id, 0),
            reason: some_unless(value.reason, ""),
        })
    }
}

#[allow(clippy::needless_pass_by_value)]
fn some_unless<N, T: PartialEq<N>>(val: T, none_val: N) -> Option<T> {
    if val.eq(&none_val) {
        None
    } else {
        Some(val)
    }
}

/// Utility module to handle (de)serialization of `Option<String>`'s
/// such that the empty string becomes None, and vice versa
mod string_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(d: D) -> Result<Option<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s))
        }
    }

    pub fn serialize<S>(d: &Option<String>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        d.as_ref().map_or("", String::as_str).serialize(s)
    }
}

/// Utility module to handle (de)serialization of `Option<u64>`'s
/// such that 0 becomes None, and vice versa
mod id_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(d: D) -> Result<Option<u64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = u64::deserialize(d)?;
        if s == 0 {
            Ok(None)
        } else {
            Ok(Some(s))
        }
    }

    pub fn serialize<S>(d: &Option<u64>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        d.unwrap_or(0).serialize(s)
    }
}
