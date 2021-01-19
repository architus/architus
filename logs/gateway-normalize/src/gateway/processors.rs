use crate::gateway::{Context, ProcessorFleet};
use anyhow::Context as _;
use jmespath::Variable;
use serde::de::{DeserializeOwned, DeserializeSeed};
use twilight_model::guild::member::MemberDeserializer;
use twilight_model::guild::Member;
use twilight_model::id::GuildId;

/// Registers all pre-configured processors
/// to handle all gateway-originating events
pub fn register_all(fleet: &mut ProcessorFleet) {
    member::register_all(fleet);
    message::register_all(fleet);
    reaction::register_all(fleet);
    interaction::register_all(fleet);
}

/// Defines processors for `MemberJoin` and `MemberLeave` events
mod member {
    use super::{chain, extract, extract_id};
    use crate::event::{Content, Entity, Nickname, UserLike};
    use crate::gateway::path::Path;
    use crate::gateway::source::{OnFailure, Source};
    use crate::gateway::{Processor, ProcessorFleet};
    use crate::rpc::submission::EventType;
    use chrono::DateTime;
    use lazy_static::lazy_static;
    use std::convert::TryFrom;
    use std::fmt;
    use twilight_model::gateway::event::EventType as GatewayEventType;

    lazy_static! {
        static ref ID_PATH: Path = Path::from("user.id");
        static ref USERNAME_PATH: Path = Path::from("user.username");
        static ref DISCRIMINATOR_PATH: Path = Path::from("user.discriminator");
        static ref NICKNAME_PATH: Path = Path::from("nick");
    }

    pub fn register_all(fleet: &mut ProcessorFleet) {
        // Register MemberJoin processor
        fleet.register(
            GatewayEventType::MemberAdd,
            Processor {
                event_type: Source::Constant(EventType::MemberJoin),
                audit_log: None,
                timestamp: Source::gateway(
                    Path::from("joined_at"),
                    chain(extract::<String>, |s, _ctx| {
                        let date = DateTime::parse_from_rfc3339(&s)?;
                        Ok(u64::try_from(date.timestamp_millis()).unwrap_or(0))
                    }),
                    OnFailure::Abort,
                ),
                reason: Source::Constant(None),
                channel: Source::Constant(None),
                agent: Source::Constant(None),
                subject: Source::sync_fn(
                    |ctx| {
                        let id = ctx.gateway(&ID_PATH, extract_id)?;
                        let username = ctx.gateway(&USERNAME_PATH, extract::<String>).ok();
                        let discriminator = ctx
                            .gateway(
                                &DISCRIMINATOR_PATH,
                                chain(extract::<String>, |s, _ctx| Ok(s.parse::<u16>()?)),
                            )
                            .ok();
                        let nickname = ctx
                            .gateway(&NICKNAME_PATH, extract::<Option<String>>)
                            .ok()
                            .map(Nickname::from);
                        Ok(Some(Entity::UserLike(UserLike {
                            id,
                            name: username,
                            nickname,
                            discriminator,
                            ..UserLike::default()
                        })))
                    },
                    OnFailure::Abort,
                ),
                auxiliary: Source::Constant(None),
                content: Source::sync_fn(
                    |ctx| {
                        let id = ctx.gateway(&ID_PATH, extract_id)?;
                        let mut content = String::from("");
                        write_mention(&mut content, id)?;
                        content.push_str(" joined");
                        Ok(Content {
                            inner: content,
                            users_mentioned: vec![id],
                            ..Content::default()
                        })
                    },
                    OnFailure::Abort,
                ),
            },
        );
        // Register MemberLeave processor
        fleet.register(
            GatewayEventType::MemberRemove,
            Processor {
                event_type: Source::Constant(EventType::MemberLeave),
                audit_log: None,
                timestamp: Source::sync_fn(|ctx| Ok(ctx.event.ingress_timestamp), OnFailure::Abort),
                reason: Source::Constant(None),
                channel: Source::Constant(None),
                agent: Source::Constant(None),
                subject: Source::sync_fn(
                    |ctx| {
                        let id = ctx.gateway(&ID_PATH, extract_id)?;
                        let username = ctx.gateway(&USERNAME_PATH, extract::<String>).ok();
                        let discriminator = ctx
                            .gateway(
                                &DISCRIMINATOR_PATH,
                                chain(extract::<String>, |s, _ctx| Ok(s.parse::<u16>()?)),
                            )
                            .ok();
                        Ok(Some(Entity::UserLike(UserLike {
                            id,
                            name: username,
                            discriminator,
                            ..UserLike::default()
                        })))
                    },
                    OnFailure::Abort,
                ),
                auxiliary: Source::Constant(None),
                content: Source::sync_fn(
                    |ctx| {
                        let id = ctx.gateway(&ID_PATH, extract_id)?;
                        let mut content = String::from("");
                        write_mention(&mut content, id)?;
                        content.push_str(" left");
                        Ok(Content {
                            inner: content,
                            users_mentioned: vec![id],
                            ..Content::default()
                        })
                    },
                    OnFailure::Abort,
                ),
            },
        );
    }

    /// Writes a user mention that will be displayed using rich formatting
    pub fn write_mention(writer: &mut impl fmt::Write, id: u64) -> Result<(), fmt::Error> {
        write!(writer, "<@{}>", id)
    }
}

/// Defines processors for `MessageSend`, `MessageReply`, `MessageEdit`,
/// `MessageDelete` (hybrid), and `MessageBulkDelete` (hybrid) events
mod message {
    use crate::gateway::ProcessorFleet;

    pub fn register_all(_fleet: &mut ProcessorFleet) {
        // TODO implement MessageSend processor
        // TODO implement MessageReply processor
        // TODO implement MessageEdit processor
        // TODO implement MessageDelete processor
        // TODO implement MessageBulkDelete processor
    }
}

/// Defines processors for `ReactionAdd`, `ReactionRemove`, and `ReactionBulkRemove` events
mod reaction {
    use super::{chain, extract, extract_id, extract_member};
    use crate::event::{Agent, Channel, Content, Emoji, Entity, Message, Nickname, UserLike};
    use crate::gateway::path::Path;
    use crate::gateway::source::{OnFailure, Source};
    use crate::gateway::{Context, Processor, ProcessorFleet};
    use crate::rpc::submission::EventType;
    use lazy_static::lazy_static;
    use std::fmt::{self, Write as _};
    use twilight_model::channel::ReactionType;
    use twilight_model::gateway::event::EventType as GatewayEventType;

    lazy_static! {
        static ref USER_ID_PATH: Path = Path::from("user_id");
        static ref MEMBER_PATH: Path = Path::from("member");
        static ref EMOJI_PATH: Path = Path::from("emoji");
    }

    pub fn register_all(fleet: &mut ProcessorFleet) {
        // Register ReactionAdd processor
        fleet.register(
            GatewayEventType::ReactionAdd,
            Processor {
                event_type: Source::Constant(EventType::ReactionAdd),
                audit_log: None,
                timestamp: Source::sync_fn(|ctx| Ok(ctx.event.ingress_timestamp), OnFailure::Abort),
                reason: Source::Constant(None),
                channel: Source::gateway(
                    Path::from("channel_id"),
                    chain(extract_id, |id, _ctx| {
                        Ok(Some(Channel {
                            id,
                            ..Channel::default()
                        }))
                    }),
                    OnFailure::Abort,
                ),
                agent: Source::sync_fn(
                    |ctx| {
                        // Reaction add events include a partial member object that we can use
                        let member_option = ctx.gateway(&MEMBER_PATH, extract_member).ok();
                        let user = if let Some(member) = member_option {
                            UserLike {
                                id: member.user.id.0,
                                name: Some(member.user.name.clone()),
                                nickname: Some(Nickname::from(member.nick.clone())),
                                discriminator: member.user.discriminator.parse::<u16>().ok(),
                                ..UserLike::default()
                            }
                        } else {
                            let id = ctx.gateway(&USER_ID_PATH, extract_id)?;
                            UserLike {
                                id,
                                ..UserLike::default()
                            }
                        };
                        Ok(Some(Agent {
                            special_type: Agent::type_from_id(user.id, ctx.config),
                            entity: Entity::UserLike(user),
                        }))
                    },
                    OnFailure::Abort,
                ),
                subject: Source::gateway(
                    Path::from("message_id"),
                    chain(extract_id, |id, _ctx| {
                        Ok(Some(Entity::Message(Message { id })))
                    }),
                    OnFailure::Abort,
                ),
                auxiliary: Source::gateway(
                    Path::from("emoji.id"),
                    chain(extract_id, |id, _ctx| Ok(Some(Entity::Emoji(Emoji { id })))),
                    OnFailure::Or(None),
                ),
                content: Source::sync_fn(format_content, OnFailure::Abort),
            },
        );
        // Register ReactionRemove processor
        fleet.register(
            GatewayEventType::ReactionRemove,
            Processor {
                event_type: Source::Constant(EventType::ReactionRemove),
                audit_log: None,
                timestamp: Source::sync_fn(|ctx| Ok(ctx.event.ingress_timestamp), OnFailure::Abort),
                reason: Source::Constant(None),
                channel: Source::gateway(
                    Path::from("channel_id"),
                    chain(extract_id, |id, _ctx| {
                        Ok(Some(Channel {
                            id,
                            ..Channel::default()
                        }))
                    }),
                    OnFailure::Abort,
                ),
                agent: Source::gateway(
                    Path::from("user_id"),
                    chain(extract_id, |id, ctx| {
                        Ok(Some(Agent {
                            special_type: Agent::type_from_id(id, ctx.config),
                            entity: Entity::UserLike(UserLike {
                                id,
                                ..UserLike::default()
                            }),
                        }))
                    }),
                    OnFailure::Abort,
                ),
                subject: Source::gateway(
                    Path::from("message_id"),
                    chain(extract_id, |id, _ctx| {
                        Ok(Some(Entity::Message(Message { id })))
                    }),
                    OnFailure::Abort,
                ),
                auxiliary: Source::gateway(
                    Path::from("emoji.id"),
                    chain(extract_id, |id, _ctx| Ok(Some(Entity::Emoji(Emoji { id })))),
                    OnFailure::Or(None),
                ),
                content: Source::sync_fn(format_content, OnFailure::Abort),
            },
        );
        // Register ReactionBulkRemove processors
        fleet.register(
            GatewayEventType::ReactionRemoveEmoji,
            Processor {
                event_type: Source::Constant(EventType::ReactionBulkRemove),
                audit_log: None,
                timestamp: Source::sync_fn(|ctx| Ok(ctx.event.ingress_timestamp), OnFailure::Abort),
                reason: Source::Constant(None),
                channel: Source::gateway(
                    Path::from("channel_id"),
                    chain(extract_id, |id, _ctx| {
                        Ok(Some(Channel {
                            id,
                            ..Channel::default()
                        }))
                    }),
                    OnFailure::Abort,
                ),
                agent: Source::Constant(None),
                subject: Source::gateway(
                    Path::from("message_id"),
                    chain(extract_id, |id, _ctx| {
                        Ok(Some(Entity::Message(Message { id })))
                    }),
                    OnFailure::Abort,
                ),
                auxiliary: Source::gateway(
                    Path::from("emoji.id"),
                    chain(extract_id, |id, _ctx| Ok(Some(Entity::Emoji(Emoji { id })))),
                    OnFailure::Or(None),
                ),
                content: Source::sync_fn(format_content, OnFailure::Abort),
            },
        );
        fleet.register(
            GatewayEventType::ReactionRemoveAll,
            Processor {
                event_type: Source::Constant(EventType::ReactionBulkRemove),
                audit_log: None,
                timestamp: Source::sync_fn(|ctx| Ok(ctx.event.ingress_timestamp), OnFailure::Abort),
                reason: Source::Constant(None),
                channel: Source::gateway(
                    Path::from("channel_id"),
                    chain(extract_id, |id, _ctx| {
                        Ok(Some(Channel {
                            id,
                            ..Channel::default()
                        }))
                    }),
                    OnFailure::Abort,
                ),
                agent: Source::Constant(None),
                subject: Source::gateway(
                    Path::from("message_id"),
                    chain(extract_id, |id, _ctx| {
                        Ok(Some(Entity::Message(Message { id })))
                    }),
                    OnFailure::Abort,
                ),
                auxiliary: Source::Constant(None),
                content: Source::sync_fn(
                    |_ctx| {
                        Ok(Content {
                            inner: String::from("all reactions removed"),
                            ..Content::default()
                        })
                    },
                    OnFailure::Abort,
                ),
            },
        );
    }

    /// Writes an embedded emoji that will be displayed using rich formatting.
    /// If a name is not supplied, then the embed will still work in the logs UI
    pub fn write_emoji(
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
    pub fn format_content(ctx: Context<'_>) -> Result<Content, anyhow::Error> {
        let reaction = ctx.gateway(&EMOJI_PATH, extract::<ReactionType>)?;
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
                write_emoji(&mut content, name.as_deref(), id.0, animated)?;
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
}

/// Defines processors for `InteractionCreate` events
mod interaction {
    use crate::gateway::ProcessorFleet;

    pub fn register_all(_fleet: &mut ProcessorFleet) {
        // TODO implement InteractionCreate processor
    }
}

/// Chains two extractor functions together
fn chain<T, R, A, B>(
    a: A,
    b: B,
) -> impl (Fn(&Variable, Context<'_>) -> Result<T, anyhow::Error>) + Send + Sync + 'static
where
    T: Clone + Sync,
    A: (Fn(&Variable, Context<'_>) -> Result<R, anyhow::Error>) + Send + Sync + 'static,
    B: (Fn(R, Context<'_>) -> Result<T, anyhow::Error>) + Send + Sync + 'static,
{
    move |variable: &Variable, ctx: Context<'_>| b(a(variable, ctx.clone())?, ctx)
}

/// u64 extractor that returns the underlying timestamp for a snowflake-encoded ID
const fn timestamp_from_id(id: u64, _ctx: Context<'_>) -> Result<u64, anyhow::Error> {
    Ok(architus_id::extract_timestamp(id))
}

/// Extractor function for a `u64` from a JSON string
fn extract_id(variable: &Variable, _ctx: Context<'_>) -> Result<u64, anyhow::Error> {
    match variable {
        Variable::String(s) => s
            .parse::<u64>()
            .context("cannot extract u64 from JSON string"),
        _ => Err(anyhow::anyhow!(
            "variable was not of type String: {:?}",
            variable
        )),
    }
}

/// Performs a generic extraction to create a value T from JSON
fn extract<T>(variable: &Variable, _ctx: Context<'_>) -> Result<T, anyhow::Error>
where
    T: DeserializeOwned,
{
    let value = serde_json::to_value(variable)?;
    let t = serde_json::from_value::<T>(value)?;
    Ok(t)
}

/// Attempts to extract a member struct using twilight's MemberDeserializer struct
/// and the guild id associated with the context's inner event struct
fn extract_member(variable: &Variable, ctx: Context<'_>) -> Result<Member, anyhow::Error> {
    let member_deserializer = MemberDeserializer::new(GuildId(ctx.event.guild_id));
    let value = serde_json::to_value(variable)?;
    let member = member_deserializer.deserialize(value).with_context(|| {
        format!(
            "could not deserialize Member value for guild id {}",
            ctx.event.guild_id
        )
    })?;
    Ok(member)
}
