//! Defines processors to source the following events:
//! - `MemberJoin` (from `GatewayEventType::MemberAdd`)
//! - `MemberLeave` (from `GatewayEventType::MemberRemove`)

use super::{extract, extract_id};
use crate::event::{Content, Entity, IdParams, Nickname, NormalizedEvent, Source, UserLike};
use crate::gateway::path::{json_path, Path};
use crate::gateway::{Processor, ProcessorContext, ProcessorFleet};
use crate::rpc::logs::event::{EventOrigin, EventType};
use chrono::DateTime;
use std::convert::TryFrom;
use std::fmt;
use twilight_model::gateway::event::EventType as GatewayEventType;

pub fn register_all(fleet: &mut ProcessorFleet) {
    fleet.register(GatewayEventType::MemberAdd, Processor::sync(member_add));
    fleet.register(
        GatewayEventType::MemberRemove,
        Processor::sync(member_remove),
    );
}

/// Handles `GatewayEventType::MemberAdd`
fn member_add(ctx: ProcessorContext<'_>) -> anyhow::Result<NormalizedEvent> {
    let user_id = ctx.gateway(json_path!("user.id"), extract_id)?;
    let username = ctx
        .gateway(json_path!("user.username"), extract::<String>)
        .ok();
    let discriminator = ctx
        .gateway(json_path!("user.discriminator"), extract::<String>)
        .ok()
        .and_then(|d| d.parse::<u16>().ok());
    let nickname = ctx
        .gateway(json_path!("nick"), extract::<Option<String>>)
        .ok()
        .map(Nickname::from);
    let joined_at = ctx.gateway(json_path!("joined_at"), extract::<String>)?;
    let joined_at_date = DateTime::parse_from_rfc3339(&joined_at)?;
    let joined_at_ms_timestamp = u64::try_from(joined_at_date.timestamp_millis())?;

    let mut content = String::from("");
    write_user_mention(&mut content, user_id)?;
    content.push_str(" joined");

    Ok(NormalizedEvent {
        event_type: EventType::MemberJoin,
        id_params: IdParams::Two(user_id, joined_at_ms_timestamp),
        timestamp: joined_at_ms_timestamp,
        guild_id: ctx.event.guild_id,
        reason: None,
        audit_log_id: None,
        channel: None,
        agent: None,
        subject: Some(Entity::UserLike(UserLike {
            id: user_id,
            name: username,
            nickname,
            discriminator,
            ..UserLike::default()
        })),
        auxiliary: None,
        content: Content {
            inner: content,
            users_mentioned: vec![user_id],
            ..Content::default()
        },
        origin: EventOrigin::Gateway,
        source: Source {
            gateway: Some(ctx.source),
            ..Source::default()
        },
    })
}

/// Handles `GatewayEventType::MemberRemove`
fn member_remove(ctx: ProcessorContext<'_>) -> anyhow::Result<NormalizedEvent> {
    let user_id = ctx.gateway(json_path!("user.id"), extract_id)?;
    let username = ctx
        .gateway(json_path!("user.username"), extract::<String>)
        .ok();
    let discriminator = ctx
        .gateway(json_path!("user.discriminator"), extract::<String>)
        .ok()
        .and_then(|d| d.parse::<u16>().ok());

    let mut content = String::from("");
    write_user_mention(&mut content, user_id)?;
    content.push_str(" left");

    Ok(NormalizedEvent {
        event_type: EventType::MemberLeave,
        id_params: IdParams::Two(user_id, ctx.event.ingress_timestamp),
        timestamp: ctx.event.ingress_timestamp,
        guild_id: ctx.event.guild_id,
        reason: None,
        audit_log_id: None,
        channel: None,
        agent: None,
        subject: Some(Entity::UserLike(UserLike {
            id: user_id,
            name: username,
            discriminator,
            ..UserLike::default()
        })),
        auxiliary: None,
        content: Content {
            inner: content,
            users_mentioned: vec![user_id],
            ..Content::default()
        },
        origin: EventOrigin::Gateway,
        source: Source {
            gateway: Some(ctx.source),
            ..Source::default()
        },
    })
}

/// Writes a user mention that will be displayed using rich formatting
pub fn write_user_mention(writer: &mut impl fmt::Write, id: u64) -> Result<(), fmt::Error> {
    write!(writer, "<@{}>", id)
}
