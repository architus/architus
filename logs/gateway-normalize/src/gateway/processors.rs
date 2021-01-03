use crate::gateway::{Context, ProcessorFleet};
use anyhow::Context as _;
use jmespath::Variable;
use serde::de::DeserializeOwned;

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
    use crate::gateway::ProcessorFleet;

    pub fn register_all(_fleet: &mut ProcessorFleet) {
        // TODO implement ReactionAdd processor
        // TODO implement ReactionRemove processor
        // TODO implement ReactionBulkRemove processor
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
