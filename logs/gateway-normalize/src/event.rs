use crate::config::Configuration;
use crate::rpc::submission::{
    AgentSpecialType, ContentMetadata, EntityRevisionMetadata, EntityType, Event as LogEvent,
    EventOrigin, EventSource, EventType, SubmitIdempotentRequest,
};
use architus_id::HoarFrost;
use std::convert::Into;
use tonic::{IntoRequest, Request};

/// Normalized log event to send to log ingestion
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, PartialEq, Debug)]
pub struct NormalizedEvent {
    /// Id using snowflake format,
    /// using the *time that the event was received by wherever it's ingested*
    pub id: HoarFrost,
    /// Unix timestamp of the *time of the underlying event* (if available)
    pub timestamp: u64,
    /// The source data, including the original gateway/audit log entries
    pub source: Source,
    /// The origin of the event
    pub origin: EventOrigin,
    /// The type of action the event is
    pub event_type: EventType,
    /// Related guild the event occurred in
    pub guild_id: u64,
    /// An optional *human-readable* reason/message of the event
    pub reason: Option<String>,
    /// Id of the corresponding audit log entry this event corresponds to, if any
    /// (included for indexing purposes)
    pub audit_log_id: Option<u64>,
    /// Channel that the event occurred in
    pub channel: Option<Channel>,
    /// The entity that caused the event to occur
    pub agent: Option<Agent>,
    /// The entity that the event is about/affects
    pub subject: Option<Entity>,
    /// Some other related entity involved in the event, if applicable
    pub auxiliary: Option<Entity>,
    /// The rich-formatted content of the log message
    pub content: Content,
}

impl IntoRequest<SubmitIdempotentRequest> for NormalizedEvent {
    fn into_request(self) -> Request<SubmitIdempotentRequest> {
        Request::new(SubmitIdempotentRequest {
            event: Some(self.into()),
        })
    }
}

impl Into<LogEvent> for NormalizedEvent {
    fn into(self) -> LogEvent {
        // Convert the normalized event struct (specific to this service)
        // into the `LogEvent` struct, which is the gRPC-serializable struct
        let (content, content_metadata) = self.content.split();
        LogEvent {
            id: self.id.0,
            timestamp: self.timestamp,
            source: Some(self.source.into()),
            origin: self.origin.into(),
            r#type: self.event_type.into(),
            guild_id: self.guild_id,
            reason: self.reason.unwrap_or_else(|| String::from("")),
            audit_log_id: self.audit_log_id.unwrap_or(0_u64),
            channel_id: self.channel.as_ref().map_or(0_u64, |c| c.id),
            channel_name: self
                .channel
                .and_then(|c| c.name)
                .unwrap_or_else(|| String::from("")),
            agent_id: self
                .agent
                .as_ref()
                .and_then(|a| a.entity.id())
                .unwrap_or(0_u64),
            agent_type: self
                .agent
                .as_ref()
                .map(|a| &a.entity)
                .map_or(EntityType::None, Entity::r#type) as i32,
            agent_special_type: self
                .agent
                .as_ref()
                .map_or(AgentSpecialType::Default, |a| a.special_type)
                as i32,
            agent_metadata: self
                .agent
                .map(|a| a.entity)
                .and_then(Entity::into_revision_metadata),
            subject_id: self.subject.as_ref().and_then(Entity::id).unwrap_or(0_u64),
            subject_type: self
                .subject
                .as_ref()
                .map_or(EntityType::None, Entity::r#type) as i32,
            subject_metadata: self.subject.and_then(Entity::into_revision_metadata),
            auxiliary_id: self
                .auxiliary
                .as_ref()
                .and_then(Entity::id)
                .unwrap_or(0_u64),
            auxiliary_type: self
                .auxiliary
                .as_ref()
                .map_or(EntityType::None, Entity::r#type) as i32,
            auxiliary_metadata: self.auxiliary.and_then(Entity::into_revision_metadata),
            content,
            content_metadata: Some(content_metadata),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Channel {
    pub id: u64,
    // This field are optional
    // and if provided, controls the display behavior
    // for the channel with the id (used in mentions)
    pub name: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct UserLike {
    pub id: u64,
    // These fields are optional
    // and if provided, control the display behavior
    // for the user with the id (used in mentions)
    pub name: Option<String>,
    pub nickname: Option<Nickname>,
    pub discriminator: Option<u16>,
    pub color: Option<u32>,
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Role {
    pub id: u64,
    // These fields are optional
    // and if provided, control the display behavior
    // for the role with the id (used in mentions)
    pub name: Option<String>,
    pub color: Option<u32>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Message {
    pub id: u64,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Emoji {
    pub id: u64,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Agent {
    pub entity: Entity,
    pub special_type: AgentSpecialType,
}

impl Agent {
    /// Attempts to resolve the special type of the agent based on their ID.
    /// Checks to see if the user is the same as Architus
    pub const fn type_from_id(id: u64, config: &Configuration) -> AgentSpecialType {
        if id == config.bot_user_id {
            AgentSpecialType::Architus
        } else {
            AgentSpecialType::Default
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Entity {
    UserLike(UserLike),
    Role(Role),
    Channel(Channel),
    Message(Message),
    Emoji(Emoji),
}

/// Represents an authoritative nickname value set
#[derive(Clone, PartialEq, Debug)]
pub enum Nickname {
    Name,
    Custom(String),
}

impl From<Option<String>> for Nickname {
    fn from(s: Option<String>) -> Self {
        match s {
            Some(n) => Self::Custom(n),
            None => Self::Name,
        }
    }
}

impl Into<Option<String>> for Nickname {
    fn into(self) -> Option<String> {
        match self {
            Self::Custom(n) => Some(n),
            Self::Name => None,
        }
    }
}

impl Entity {
    pub const fn id(&self) -> Option<u64> {
        match self {
            Self::UserLike(UserLike { id, .. })
            | Self::Role(Role { id, .. })
            | Self::Channel(Channel { id, .. })
            | Self::Message(Message { id, .. })
            | Self::Emoji(Emoji { id, .. }) => Some(*id),
        }
    }

    pub const fn r#type(&self) -> EntityType {
        match self {
            Self::UserLike { .. } => EntityType::UserLike,
            Self::Role { .. } => EntityType::Role,
            Self::Channel(_) => EntityType::Channel,
            Self::Message { .. } => EntityType::Message,
            Self::Emoji { .. } => EntityType::Emoji,
        }
    }

    pub fn into_revision_metadata(self) -> Option<EntityRevisionMetadata> {
        match self {
            Self::UserLike(UserLike {
                name,
                discriminator,
                nickname,
                color,
                ..
            }) => Some(EntityRevisionMetadata {
                name: name.unwrap_or_else(|| String::from("")),
                color: color.unwrap_or(0_u32),
                has_nickname: nickname.is_some(),
                nickname: nickname
                    .and_then(|n| n.into())
                    .unwrap_or_else(|| String::from("")),
                has_discriminator: discriminator.is_some(),
                discriminator: discriminator.map_or(0_u32, u32::from),
            }),
            Self::Role(Role { name, color, .. }) => Some(EntityRevisionMetadata {
                name: name.unwrap_or_else(|| String::from("")),
                color: color.unwrap_or(0_u32),
                ..EntityRevisionMetadata::default()
            }),
            Self::Channel(Channel { name, .. }) => Some(EntityRevisionMetadata {
                name: name.unwrap_or_else(|| String::from("")),
                ..EntityRevisionMetadata::default()
            }),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Default)]
pub struct Content {
    pub inner: String,
    pub users_mentioned: Vec<u64>,
    pub channels_mentioned: Vec<u64>,
    pub roles_mentioned: Vec<u64>,
    pub emojis_used: Vec<String>,
    pub custom_emojis_used: Vec<u64>,
    pub custom_emoji_names_used: Vec<String>,
    pub url_stems: Vec<String>,
}

impl Content {
    pub fn make<S: Into<String>>(inner: S) -> Self {
        let inner = inner.into();
        Self {
            inner,
            // TODO parse/extract mentions
            users_mentioned: Vec::new(),
            channels_mentioned: Vec::new(),
            roles_mentioned: Vec::new(),
            emojis_used: Vec::new(),
            custom_emojis_used: Vec::new(),
            custom_emoji_names_used: Vec::new(),
            url_stems: Vec::new(),
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    fn split(self) -> (String, ContentMetadata) {
        (
            self.inner,
            ContentMetadata {
                users_mentioned: self.users_mentioned,
                channels_mentioned: self.channels_mentioned,
                roles_mentioned: self.roles_mentioned,
                emojis_used: self.emojis_used,
                custom_emojis_used: self.custom_emojis_used,
                custom_emoji_names_used: self.custom_emoji_names_used,
                url_stems: self.url_stems,
            },
        )
    }
}

/// Represents the in-memory version of the original JSON that this event represents,
/// included in the event struct for future data processing
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Source {
    pub gateway: Option<serde_json::Value>,
    pub audit_log: Option<serde_json::Value>,
    pub internal: Option<serde_json::Value>,
}

impl Source {
    /// Calculates the `EventOrigin` variant for this source object,
    /// using the presence of the each sub-field to produce the result
    #[must_use]
    pub const fn origin(&self) -> EventOrigin {
        match (&self.gateway, &self.audit_log) {
            (Some(_), Some(_)) => EventOrigin::Hybrid,
            (Some(_), None) => EventOrigin::Gateway,
            (None, Some(_)) => EventOrigin::AuditLog,
            (None, None) => EventOrigin::Internal,
        }
    }
}

impl Into<EventSource> for Source {
    fn into(self) -> EventSource {
        EventSource {
            gateway: self
                .gateway
                .and_then(|json| serde_json::to_string(&json).ok())
                .unwrap_or_else(|| String::from("")),
            audit_log: self
                .audit_log
                .and_then(|json| serde_json::to_string(&json).ok())
                .unwrap_or_else(|| String::from("")),
            internal: self
                .internal
                .and_then(|json| serde_json::to_string(&json).ok())
                .unwrap_or_else(|| String::from("")),
        }
    }
}
