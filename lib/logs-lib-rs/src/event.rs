#![allow(clippy::module_name_repetitions)]

use crate::rpc::logs::submission::{
    EntityRevisionMetadata, EventDeterministicIdParams, SubmitIdempotentRequest, SubmittedEvent,
};
use std::convert::Into;
use tonic::{IntoRequest, Request};
use twilight_model::user::User as DiscordUser;

// Re-export enums from `crate::rpc::logs::event` as their direct names.
pub use crate::rpc::logs::event::{AgentSpecialType, EntityType, EventOrigin, EventType};
// Re-export structs from `crate::rpc::logs::event` with `Proto` added in the front
// to mark them as the non-ergonomic versions.
pub use crate::rpc::logs::event::{
    ContentMetadata as ProtoContentMetadata, Event as ProtoEvent, EventSource as ProtoEventSource,
};

/// Normalized log event to send to log ingestion.
/// This is a utility wrapper around `Event` to provide a more ergonomic API.
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, PartialEq, Debug)]
pub struct NormalizedEvent {
    /// Fields used to deterministically generate the ID for this normalized event
    /// (combined with the event type to produce a unique ID).
    /// This can be any arbitrary data,
    /// but it must be deterministic and consistent
    /// **across all event normalizations/generations for that event type
    /// across the codebase**.
    pub id_params: IdParams,
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

/// Fields used to deterministically generate the ID for this normalized event
/// (combined with the event type to produce a unique ID).
/// This can be any arbitrary data,
/// but it must be deterministic and consistent
/// **across all event normalizations/generations for that event type
/// across the codebase**.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Debug)]
pub enum IdParams {
    One(u64),
    Two(u64, u64),
    Three(u64, u64, u64),
}

impl From<IdParams> for EventDeterministicIdParams {
    fn from(original: IdParams) -> Self {
        match original {
            IdParams::One(field1) => Self {
                field1,
                field2: 0,
                field3: 0,
            },
            IdParams::Two(field1, field2) => Self {
                field1,
                field2,
                field3: 0,
            },
            IdParams::Three(field1, field2, field3) => Self {
                field1,
                field2,
                field3,
            },
        }
    }
}

impl IntoRequest<SubmitIdempotentRequest> for NormalizedEvent {
    fn into_request(self) -> Request<SubmitIdempotentRequest> {
        Request::new(SubmitIdempotentRequest {
            event: Some(self.into()),
        })
    }
}

impl From<NormalizedEvent> for SubmittedEvent {
    fn from(original: NormalizedEvent) -> Self {
        // Convert the normalized event struct (specific to this service)
        // into the `ProtoEvent` struct, which is the gRPC-serializable struct
        let (content, content_metadata) = original.content.split();
        Self {
            inner: Some(ProtoEvent {
                timestamp: original.timestamp,
                source: Some(original.source.into()),
                origin: original.origin.into(),
                r#type: original.event_type.into(),
                guild_id: original.guild_id,
                reason: original.reason.unwrap_or_else(|| String::from("")),
                audit_log_id: original.audit_log_id.unwrap_or(0_u64),
                channel_id: original.channel.as_ref().map_or(0_u64, |c| c.id),
                agent_id: original
                    .agent
                    .as_ref()
                    .and_then(|a| a.entity.id())
                    .unwrap_or(0_u64),
                agent_type: original
                    .agent
                    .as_ref()
                    .map(|a| &a.entity)
                    .map_or(EntityType::None, Entity::r#type) as i32,
                agent_special_type: original
                    .agent
                    .as_ref()
                    .map_or(AgentSpecialType::Default, |a| a.special_type)
                    as i32,
                agent_webhook_username: original
                    .agent
                    .as_ref()
                    .and_then(|a| a.webhook_username.clone())
                    .unwrap_or_else(|| String::from("")),
                subject_id: original
                    .subject
                    .as_ref()
                    .and_then(Entity::id)
                    .unwrap_or(0_u64),
                subject_type: original
                    .subject
                    .as_ref()
                    .map_or(EntityType::None, Entity::r#type) as i32,
                auxiliary_id: original
                    .auxiliary
                    .as_ref()
                    .and_then(Entity::id)
                    .unwrap_or(0_u64),
                auxiliary_type: original
                    .auxiliary
                    .as_ref()
                    .map_or(EntityType::None, Entity::r#type)
                    as i32,
                content,
                content_metadata: Some(content_metadata),
            }),
            channel_name: original
                .channel
                .and_then(|c| c.name)
                .unwrap_or_else(|| String::from("")),
            agent_metadata: original
                .agent
                .map(|a| a.entity)
                .and_then(Entity::into_revision_metadata),
            subject_metadata: original.subject.and_then(Entity::into_revision_metadata),
            auxiliary_metadata: original.auxiliary.and_then(Entity::into_revision_metadata),
            id_params: Some(original.id_params.into()),
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
    pub webhook_username: Option<String>,
}

impl Agent {
    /// Attempts to resolve the special type of the agent based on their ID.
    /// Checks to see if the user is the same as Architus
    #[must_use]
    pub fn type_from_id(id: u64, bot_user_id: Option<u64>) -> AgentSpecialType {
        if bot_user_id.map(|i| i == id).unwrap_or(false) {
            AgentSpecialType::Architus
        } else {
            AgentSpecialType::Default
        }
    }

    /// Attempts to resolve the special type of the agent based on the Discord user.
    /// Checks to see if the user is the same as Architus
    #[must_use]
    pub fn type_from_discord_user(
        user: &DiscordUser,
        bot_user_id: Option<u64>,
    ) -> AgentSpecialType {
        if bot_user_id.map(|i| i == user.id.0).unwrap_or(false) {
            AgentSpecialType::Architus
        } else if user.system.unwrap_or(false) {
            AgentSpecialType::System
        } else if user.bot {
            AgentSpecialType::Bot
        } else {
            AgentSpecialType::Default
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
#[allow(dead_code)]
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

impl From<Nickname> for Option<String> {
    fn from(original: Nickname) -> Self {
        match original {
            Nickname::Custom(n) => Some(n),
            Nickname::Name => None,
        }
    }
}

impl Entity {
    #[must_use]
    pub const fn id(&self) -> Option<u64> {
        match self {
            Self::UserLike(UserLike { id, .. })
            | Self::Role(Role { id, .. })
            | Self::Channel(Channel { id, .. })
            | Self::Message(Message { id, .. })
            | Self::Emoji(Emoji { id, .. }) => Some(*id),
        }
    }

    #[must_use]
    pub const fn r#type(&self) -> EntityType {
        match self {
            Self::UserLike { .. } => EntityType::UserLike,
            Self::Role { .. } => EntityType::Role,
            Self::Channel(_) => EntityType::Channel,
            Self::Message { .. } => EntityType::Message,
            Self::Emoji { .. } => EntityType::Emoji,
        }
    }

    #[must_use]
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
    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    fn split(self) -> (String, ProtoContentMetadata) {
        (
            self.inner,
            ProtoContentMetadata {
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

impl From<Source> for ProtoEventSource {
    fn from(original: Source) -> Self {
        Self {
            gateway: original
                .gateway
                .and_then(|json| serde_json::to_string(&json).ok())
                .unwrap_or_else(|| String::from("")),
            audit_log: original
                .audit_log
                .and_then(|json| serde_json::to_string(&json).ok())
                .unwrap_or_else(|| String::from("")),
            internal: original
                .internal
                .and_then(|json| serde_json::to_string(&json).ok())
                .unwrap_or_else(|| String::from("")),
        }
    }
}
