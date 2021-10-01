//! Defines processors to source the following events:
//! - `ReactionAdd` (from `GatewayEventType::ReactionAdd`)
//! - `ReactionRemove` (from `GatewayEventType::ReactionRemove`)
//! - `ReactionBulkRemove` (from `GatewayEventType::ReactionRemoveEmoji`
//!    and `GatewayEventType::ReactionRemoveAll`)

use super::{extract, extract_id, extract_member};
use crate::event::{
    Agent, Channel, Content, Emoji, Entity, IdParams, Message, Nickname, NormalizedEvent, Source,
    UserLike,
};
use crate::gateway::path::{json_path, Path};
use crate::gateway::{Processor, ProcessorContext, ProcessorFleet};
use crate::rpc::logs::event::{EventOrigin, EventType};
use std::fmt::{self, Write as _};
use twilight_model::channel::ReactionType;
use twilight_model::gateway::event::EventType as GatewayEventType;

pub fn register_all(fleet: &mut ProcessorFleet) {
    fleet.register(GatewayEventType::ReactionAdd, Processor::sync(reaction_add));
    fleet.register(
        GatewayEventType::ReactionRemove,
        Processor::sync(reaction_remove),
    );
    fleet.register(
        GatewayEventType::ReactionRemoveEmoji,
        Processor::sync(reaction_remove_emoji),
    );
    fleet.register(
        GatewayEventType::ReactionRemoveAll,
        Processor::sync(reaction_remove_all),
    );
}

fn reaction_add(ctx: ProcessorContext<'_>) -> anyhow::Result<NormalizedEvent> {
    // Reaction add events include a partial member object that we can use
    let member_option = ctx.gateway(json_path!("member"), extract_member).ok();
    let user = if let Some(member) = member_option {
        UserLike {
            id: member.user.id.0,
            name: Some(member.user.name.clone()),
            nickname: Some(Nickname::from(member.nick.clone())),
            discriminator: member.user.discriminator.parse::<u16>().ok(),
            ..UserLike::default()
        }
    } else {
        let id = ctx.gateway(json_path!("user_id"), extract_id)?;
        UserLike {
            id,
            ..UserLike::default()
        }
    };

    let reaction = ctx.gateway(json_path!("emoji"), extract::<ReactionType>)?;
    let channel_id = ctx.gateway(json_path!("channel_id"), extract_id)?;
    let message_id = ctx.gateway(json_path!("message_id"), extract_id)?;

    Ok(NormalizedEvent {
        event_type: EventType::ReactionAdd,
        id_params: IdParams::Three(user.id, message_id, ctx.event.ingress_timestamp),
        timestamp: ctx.event.ingress_timestamp,
        guild_id: ctx.event.guild_id,
        reason: None,
        audit_log_id: None,
        channel: Some(Channel {
            id: channel_id,
            ..Channel::default()
        }),
        agent: Some(Agent {
            special_type: Agent::type_from_id(user.id, ctx.config),
            entity: Entity::UserLike(user),
        }),
        subject: Some(Entity::Message(Message { id: message_id })),
        auxiliary: match reaction {
            ReactionType::Unicode { .. } => None,
            ReactionType::Custom { id, .. } => Some(Entity::Emoji(Emoji { id: id.0 })),
        },
        content: format_content(reaction, &ctx)?,
        origin: EventOrigin::Gateway,
        source: Source {
            gateway: Some(ctx.source),
            ..Source::default()
        },
    })
}

fn reaction_remove(ctx: ProcessorContext<'_>) -> anyhow::Result<NormalizedEvent> {
    let user_id = ctx.gateway(json_path!("user_id"), extract_id)?;
    let reaction = ctx.gateway(json_path!("emoji"), extract::<ReactionType>)?;
    let channel_id = ctx.gateway(json_path!("channel_id"), extract_id)?;
    let message_id = ctx.gateway(json_path!("message_id"), extract_id)?;

    Ok(NormalizedEvent {
        event_type: EventType::ReactionRemove,
        id_params: IdParams::Three(user_id, message_id, ctx.event.ingress_timestamp),
        timestamp: ctx.event.ingress_timestamp,
        guild_id: ctx.event.guild_id,
        reason: None,
        audit_log_id: None,
        channel: Some(Channel {
            id: channel_id,
            ..Channel::default()
        }),
        agent: Some(Agent {
            special_type: Agent::type_from_id(user_id, ctx.config),
            entity: Entity::UserLike(UserLike {
                id: user_id,
                ..UserLike::default()
            }),
        }),
        subject: Some(Entity::Message(Message { id: message_id })),
        auxiliary: match reaction {
            ReactionType::Unicode { .. } => None,
            ReactionType::Custom { id, .. } => Some(Entity::Emoji(Emoji { id: id.0 })),
        },
        content: format_content(reaction, &ctx)?,
        origin: EventOrigin::Gateway,
        source: Source {
            gateway: Some(ctx.source),
            ..Source::default()
        },
    })
}

fn reaction_remove_emoji(ctx: ProcessorContext<'_>) -> anyhow::Result<NormalizedEvent> {
    let reaction = ctx.gateway(json_path!("emoji"), extract::<ReactionType>)?;
    let channel_id = ctx.gateway(json_path!("channel_id"), extract_id)?;
    let message_id = ctx.gateway(json_path!("message_id"), extract_id)?;

    Ok(NormalizedEvent {
        event_type: EventType::ReactionBulkRemove,
        id_params: IdParams::Two(message_id, ctx.event.ingress_timestamp),
        timestamp: ctx.event.ingress_timestamp,
        guild_id: ctx.event.guild_id,
        reason: None,
        audit_log_id: None,
        channel: Some(Channel {
            id: channel_id,
            ..Channel::default()
        }),
        agent: None,
        subject: Some(Entity::Message(Message { id: message_id })),
        auxiliary: match reaction {
            ReactionType::Unicode { .. } => None,
            ReactionType::Custom { id, .. } => Some(Entity::Emoji(Emoji { id: id.0 })),
        },
        content: format_content(reaction, &ctx)?,
        origin: EventOrigin::Gateway,
        source: Source {
            gateway: Some(ctx.source),
            ..Source::default()
        },
    })
}

fn reaction_remove_all(ctx: ProcessorContext<'_>) -> anyhow::Result<NormalizedEvent> {
    let channel_id = ctx.gateway(json_path!("channel_id"), extract_id)?;
    let message_id = ctx.gateway(json_path!("message_id"), extract_id)?;

    Ok(NormalizedEvent {
        event_type: EventType::ReactionBulkRemove,
        id_params: IdParams::Two(message_id, ctx.event.ingress_timestamp),
        timestamp: ctx.event.ingress_timestamp,
        guild_id: ctx.event.guild_id,
        reason: None,
        audit_log_id: None,
        channel: Some(Channel {
            id: channel_id,
            ..Channel::default()
        }),
        agent: None,
        subject: Some(Entity::Message(Message { id: message_id })),
        auxiliary: None,
        content: Content {
            inner: String::from("all reactions removed"),
            ..Content::default()
        },
        origin: EventOrigin::Gateway,
        source: Source {
            gateway: Some(ctx.source),
            ..Source::default()
        },
    })
}

/// Writes an embedded emoji that will be displayed using rich formatting.
/// If a name is not supplied, then the embed will still work in the logs UI
pub fn write_emoji_mention(
    writer: &mut impl fmt::Write,
    name: Option<&str>,
    id: u64,
    animated: bool,
) -> Result<(), fmt::Error> {
    let animated_prefix = if animated { "a" } else { "" };
    let name = name.unwrap_or("");
    write!(writer, "<{}:{}:{}>", animated_prefix, name, id)
}

/// Formats a reaction content block
pub fn format_content(
    reaction: ReactionType,
    ctx: &ProcessorContext<'_>,
) -> Result<Content, anyhow::Error> {
    let mut content = String::from("");
    match reaction {
        ReactionType::Unicode { name } => {
            content.push_str(&name);
            if let Some(shortcodes) = ctx.emojis.to_shortcodes(&name) {
                for shortcode in shortcodes {
                    write!(content, " :{}:", shortcode)?;
                }
            }
            Ok(Content {
                inner: content,
                emojis_used: vec![name],
                ..Content::default()
            })
        }
        ReactionType::Custom { id, animated, name } => {
            write_emoji_mention(&mut content, name.as_deref(), id.0, animated)?;
            if let Some(name) = name {
                write!(content, " :{}:", name)?;
                Ok(Content {
                    inner: content,
                    custom_emojis_used: vec![id.0],
                    custom_emoji_names_used: vec![name],
                    ..Content::default()
                })
            } else {
                write!(content, " :{}:", id)?;
                Ok(Content {
                    inner: content,
                    custom_emojis_used: vec![id.0],
                    ..Content::default()
                })
            }
        }
    }
}
