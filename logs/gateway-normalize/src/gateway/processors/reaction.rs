//! Defines processors to source the following events:
//! - `ReactionAdd` (from `GatewayEventType::ReactionAdd`)
//! - `ReactionRemove` (from `GatewayEventType::ReactionRemove`)
//! - `ReactionBulkRemove` (from `GatewayEventType::ReactionRemoveEmoji`
//!    and `GatewayEventType::ReactionRemoveAll`)

use super::{extract, extract_id, extract_member};
use crate::gateway::path::{json_path, Path};
use crate::gateway::{Processor, ProcessorContext, ProcessorError, ProcessorFleet};
use architus_logs_lib::event::{
    Agent, Channel, Content, Emoji, Entity, EventOrigin, EventType, IdParams, Message, Nickname,
    NormalizedEvent, Source, UserLike,
};
use std::fmt::Write as _;
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

fn reaction_add(ctx: ProcessorContext<'_>) -> Result<NormalizedEvent, ProcessorError> {
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
        let id = ctx
            .gateway(json_path!("user_id"), extract_id)
            .map_err(ProcessorError::Fatal)?;
        UserLike {
            id,
            ..UserLike::default()
        }
    };

    let reaction = ctx
        .gateway(json_path!("emoji"), extract::<ReactionType>)
        .map_err(ProcessorError::Fatal)?;
    let channel_id = ctx
        .gateway(json_path!("channel_id"), extract_id)
        .map_err(ProcessorError::Fatal)?;
    let message_id = ctx
        .gateway(json_path!("message_id"), extract_id)
        .map_err(ProcessorError::Fatal)?;

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
            special_type: Agent::type_from_id(user.id, Some(ctx.config.bot_user_id)),
            entity: Entity::UserLike(user),
            webhook_username: None,
        }),
        subject: Some(Entity::Message(Message { id: message_id })),
        auxiliary: match reaction {
            ReactionType::Unicode { .. } => None,
            ReactionType::Custom { id, .. } => Some(Entity::Emoji(Emoji { id: id.0 })),
        },
        content: format_content(reaction, &ctx).map_err(ProcessorError::Fatal)?,
        origin: EventOrigin::Gateway,
        source: Source {
            gateway: Some(ctx.source),
            ..Source::default()
        },
    })
}

fn reaction_remove(ctx: ProcessorContext<'_>) -> Result<NormalizedEvent, ProcessorError> {
    let user_id = ctx
        .gateway(json_path!("user_id"), extract_id)
        .map_err(ProcessorError::Fatal)?;
    let reaction = ctx
        .gateway(json_path!("emoji"), extract::<ReactionType>)
        .map_err(ProcessorError::Fatal)?;
    let channel_id = ctx
        .gateway(json_path!("channel_id"), extract_id)
        .map_err(ProcessorError::Fatal)?;
    let message_id = ctx
        .gateway(json_path!("message_id"), extract_id)
        .map_err(ProcessorError::Fatal)?;

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
            special_type: Agent::type_from_id(user_id, Some(ctx.config.bot_user_id)),
            entity: Entity::UserLike(UserLike {
                id: user_id,
                ..UserLike::default()
            }),
            webhook_username: None,
        }),
        subject: Some(Entity::Message(Message { id: message_id })),
        auxiliary: match reaction {
            ReactionType::Unicode { .. } => None,
            ReactionType::Custom { id, .. } => Some(Entity::Emoji(Emoji { id: id.0 })),
        },
        content: format_content(reaction, &ctx).map_err(ProcessorError::Fatal)?,
        origin: EventOrigin::Gateway,
        source: Source {
            gateway: Some(ctx.source),
            ..Source::default()
        },
    })
}

fn reaction_remove_emoji(ctx: ProcessorContext<'_>) -> Result<NormalizedEvent, ProcessorError> {
    let reaction = ctx
        .gateway(json_path!("emoji"), extract::<ReactionType>)
        .map_err(ProcessorError::Fatal)?;
    let channel_id = ctx
        .gateway(json_path!("channel_id"), extract_id)
        .map_err(ProcessorError::Fatal)?;
    let message_id = ctx
        .gateway(json_path!("message_id"), extract_id)
        .map_err(ProcessorError::Fatal)?;

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
        content: format_content(reaction, &ctx).map_err(ProcessorError::Fatal)?,
        origin: EventOrigin::Gateway,
        source: Source {
            gateway: Some(ctx.source),
            ..Source::default()
        },
    })
}

fn reaction_remove_all(ctx: ProcessorContext<'_>) -> Result<NormalizedEvent, ProcessorError> {
    let channel_id = ctx
        .gateway(json_path!("channel_id"), extract_id)
        .map_err(ProcessorError::Fatal)?;
    let message_id = ctx
        .gateway(json_path!("message_id"), extract_id)
        .map_err(ProcessorError::Fatal)?;

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
            architus_logs_lib::content::write_custom_emoji(
                &mut content,
                id.0,
                name.as_deref(),
                animated,
            )?;
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
