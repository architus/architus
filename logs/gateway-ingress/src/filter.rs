//! Handles the filtering/conversion logic that both:
//! - determines whether to forward a raw gateway event to the durable message queue
//! - and converts the raw gateway event into a partially deserialized state,
//!   where the guild id, event type, timestamp, and inner payload JSON
//!   have all been extracted

use crate::rpc::gateway_queue_lib::GatewayEvent;
use lazy_static::lazy_static;
use slog::Logger;
use std::str::Utf8Error;
use twilight_gateway::{Event, EventType, Intents};
use twilight_model::gateway::event::gateway::GatewayEventDeserializer;
use twilight_model::gateway::OpCode;

pub enum ConvertRawEventError {
    UnknownEventType(Event),
    RawEventUtf8DecodeError(Utf8Error),
    RawEventScanFailed {
        raw_event: String,
    },
    NoEventTypeFound {
        raw_event: String,
    },
    JsonDeserializeError {
        inner: serde_json::Error,
        event_type: String,
        raw_event: String,
    },
    NoMessagePayloadFound {
        event_type: String,
        raw_event: String,
    },
    NoGuildIdFound {
        event_type: String,
        raw_event: String,
    },
    MessagePackSerializeError {
        inner: rmp_serde::encode::Error,
        event_type: String,
        raw_event: String,
    },
}

impl ConvertRawEventError {
    pub fn log(&self, logger: &Logger) {
        match self {
            ConvertRawEventError::UnknownEventType(event) => {
                slog::warn!(
                    logger,
                    "unknown event type piped to raw event conversion";
                    "event" => ?event,
                );
            }
            ConvertRawEventError::RawEventUtf8DecodeError(err) => {
                slog::warn!(
                    logger,
                    "an error occurred when parsing raw event JSON as UTF-8";
                    "error" => ?err,
                );
            }
            ConvertRawEventError::RawEventScanFailed { raw_event } => {
                slog::warn!(
                    logger,
                    "could not find op-code and event type fields when scanning event JSON";
                    "raw_event" => raw_event,
                );
            }
            ConvertRawEventError::NoEventTypeFound { raw_event } => {
                slog::warn!(
                    logger,
                    "no event type field found in event JSON";
                    "raw_event" => raw_event,
                );
            }
            ConvertRawEventError::JsonDeserializeError {
                inner,
                event_type,
                raw_event,
            } => {
                slog::warn!(
                    logger,
                    "could not fully deserialize raw gateway event";
                    "error" => ?inner,
                    "event_type" => event_type,
                    "raw_event" => raw_event,
                );
            }
            ConvertRawEventError::NoMessagePayloadFound {
                event_type,
                raw_event,
            } => {
                slog::warn!(
                    logger,
                    "no message payload field was found in gateway event JSON";
                    "event_type" => event_type,
                    "raw_event" => raw_event,
                );
            }
            ConvertRawEventError::NoGuildIdFound {
                event_type,
                raw_event,
            } => {
                slog::warn!(
                    logger,
                    "no guild ID found in gateway event payload";
                    "event_type" => event_type,
                    "raw_event" => raw_event,
                );
            }
            ConvertRawEventError::MessagePackSerializeError {
                inner,
                event_type,
                raw_event,
            } => {
                slog::warn!(
                    logger,
                    "could not re-serialize inner gateway event payload to MessagePack";
                    "error" => ?inner,
                    "event_type" => event_type,
                    "raw_event" => raw_event,
                );
            }
        }
    }
}

/// Attempts to synchronously convert a raw gateway event into our struct
/// that will eventually be published to the gateway queue
pub fn try_convert_raw_event(
    event: Event,
    timestamp: u64,
) -> Result<Option<GatewayEvent>, ConvertRawEventError> {
    // Extract the raw json from the event
    let raw_event_payload = match event {
        Event::ShardPayload(payload) => payload,
        _ => return Err(ConvertRawEventError::UnknownEventType(event)),
    };
    let json = std::str::from_utf8(&raw_event_payload.bytes)
        .map_err(ConvertRawEventError::RawEventUtf8DecodeError)?;

    // Use twilight's fast pre-deserializer to determine the op and event type
    let deserializer = GatewayEventDeserializer::from_json(json).ok_or_else(|| {
        ConvertRawEventError::RawEventScanFailed {
            raw_event: String::from(json),
        }
    })?;
    let (op, _, event_type_option) = deserializer.into_parts();
    if op != OpCode::Event as u8 {
        return Ok(None);
    }

    // Make sure there was a proper event type
    let event_type_str =
        event_type_option.ok_or_else(|| ConvertRawEventError::NoEventTypeFound {
            raw_event: String::from(json),
        })?;

    // Make sure we should forward the event by filtering by the event type.
    // Note: this deserialization could fail if we are processing an unknown event;
    // this is fine (the ingress should gracefully fallback to forwarding by default).
    let event_type = serde_json::from_str::<EventType>(&format!(r#""{}""#, event_type_str)).ok();
    if !should_forward(event_type) {
        return Ok(None);
    }

    let value = serde_json::from_str::<serde_json::Value>(json).map_err(|err| {
        ConvertRawEventError::JsonDeserializeError {
            inner: err,
            event_type: String::from(event_type_str),
            raw_event: String::from(json),
        }
    })?;

    // Attempt to find the ".d" value (contains the Gateway message payload)
    // https://discord.com/developers/docs/topics/gateway#payloads-gateway-payload-structure
    let inner_payload = match value {
        serde_json::Value::Object(mut map) => {
            map.remove("d")
                .ok_or_else(|| ConvertRawEventError::NoMessagePayloadFound {
                    event_type: String::from(event_type_str),
                    raw_event: String::from(json),
                })?
        }
        _ => {
            return Err(ConvertRawEventError::NoMessagePayloadFound {
                event_type: String::from(event_type_str),
                raw_event: String::from(json),
            })
        }
    };

    // Make sure the guild id can be extracted before forwarding
    let guild_id = try_extract_guild_id(&inner_payload).ok_or_else(|| {
        ConvertRawEventError::NoGuildIdFound {
            event_type: String::from(event_type_str),
            raw_event: String::from(json),
        }
    })?;

    // Serialize the inner event to MessagePack
    let inner_payload_bytes = rmp_serde::to_vec(&inner_payload).map_err(|err| {
        ConvertRawEventError::MessagePackSerializeError {
            inner: err,
            event_type: String::from(event_type_str),
            raw_event: String::from(json),
        }
    })?;

    Ok(Some(GatewayEvent {
        ingress_timestamp: timestamp,
        inner: inner_payload_bytes,
        event_type: event_type_str.to_owned(),
        guild_id,
    }))
}

lazy_static! {
    /// Includes all guild-related events that are processed.
    /// Signals to Discord that we intend to receive and process them
    pub static ref INTENTS: Intents = Intents::GUILD_MEMBERS
        | Intents::GUILD_MESSAGES
        | Intents::GUILD_MESSAGE_REACTIONS;
}

/// Determines whether the ingress shard should forward events to the queue
/// (certain events, such as raw gateway lifecycle events, should not be forwarded)
pub const fn should_forward(event_type_option: Option<EventType>) -> bool {
    // Don't forward lifecycle events (or typing/presence updates):
    // `https://discord.com/developers/docs/topics/gateway#commands-and-events-gateway-events`
    // Default to forwarding an event if it is not identified
    match event_type_option {
        Some(event_type) => {
            match event_type {
                // Based on the available processors in:
                // logs/gateway-normalize/src/gateway/processors.rs
                EventType::MemberAdd
                    | EventType::MemberRemove
                    | EventType::MessageCreate
                    | EventType::MessageUpdate
                    | EventType::MessageDelete
                    | EventType::MessageDeleteBulk
                    | EventType::InteractionCreate
                    | EventType::ReactionAdd
                    | EventType::ReactionRemove
                    | EventType::ReactionRemoveEmoji
                    | EventType::ReactionRemoveAll => true,
                EventType::GatewayHeartbeat
                    | EventType::GatewayHeartbeatAck
                    | EventType::GatewayHello
                    | EventType::GatewayInvalidateSession
                    | EventType::GatewayReconnect
                    | EventType::GiftCodeUpdate
                    | EventType::MemberChunk
                    | EventType::PresenceUpdate
                    | EventType::PresencesReplace
                    | EventType::Ready
                    | EventType::Resumed
                    | EventType::ShardConnected
                    | EventType::ShardConnecting
                    | EventType::ShardDisconnected
                    | EventType::ShardIdentifying
                    | EventType::ShardReconnecting
                    | EventType::ShardPayload
                    | EventType::ShardResuming
                    | EventType::TypingStart
                    | EventType::UnavailableGuild
                    | EventType::VoiceServerUpdate
                    // Disable forwarding events for guilds coming off/online
                    // Note: that means we'll have to have some other mechanism for logging bot joins/leaves
                    | EventType::GuildCreate
                    | EventType::GuildDelete
                    // Disable forwarding events for those that are handled via the audit log
                    | EventType::BanAdd
                    | EventType::BanRemove
                    | EventType::ChannelCreate
                    | EventType::ChannelDelete
                    | EventType::ChannelPinsUpdate
                    | EventType::ChannelUpdate
                    | EventType::GuildEmojisUpdate
                    | EventType::GuildIntegrationsUpdate
                    | EventType::GuildUpdate
                    | EventType::IntegrationCreate
                    | EventType::IntegrationDelete
                    | EventType::IntegrationUpdate
                    | EventType::StageInstanceCreate
                    | EventType::StageInstanceDelete
                    | EventType::StageInstanceUpdate
                    | EventType::InviteCreate
                    | EventType::InviteDelete
                    | EventType::MemberUpdate
                    | EventType::RoleCreate
                    | EventType::RoleDelete
                    | EventType::RoleUpdate
                    | EventType::UserUpdate
                    | EventType::VoiceStateUpdate
                    | EventType::WebhooksUpdate => false,
            }
        }
        // Forward events that couldn't be matched
        None => true,
    }
}

/// Attempts to extract a guild id from a partially-serialized gateway event
fn try_extract_guild_id(json_value: &serde_json::Value) -> Option<u64> {
    if let serde_json::Value::Object(map) = json_value {
        if let Some(serde_json::Value::String(guild_id_string)) = map.get("guild_id") {
            // Attempt to parse the guild id string to a u64
            return guild_id_string.parse::<u64>().ok();
        }
    }

    None
}
