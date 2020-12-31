use crate::audit_log;
use crate::event::{Agent, Channel, Content, Entity, UserLike};
use crate::gateway::path::Path;
use crate::gateway::source::{AuditLogSource, OnFailure, Source};
use crate::gateway::{Context, Processor, ProcessorFleet};
use crate::rpc::submission::EventType;
use anyhow::Context as _;
use jmespath::Variable;
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use std::time::Duration;
use twilight_model::channel::ChannelType;
use twilight_model::gateway::event::EventType as GatewayEventType;
use twilight_model::guild::audit_log::{AuditLogEntry, AuditLogEvent};
use twilight_model::guild::Permissions;

/// Registers all pre-configured processors
/// to handle as many gateway events as possible
pub fn register_all(fleet: ProcessorFleet) -> ProcessorFleet {
    fleet
        .register(
            GatewayEventType::ChannelCreate,
            Processor {
                event_type: Source::Constant(EventType::ChannelCreate),
                audit_log: Some(AuditLogSource::new(
                    |ctx| {
                        Box::pin(async move {
                            // Make sure that the bot has permissions first
                            if !ctx.has_perms(Permissions::VIEW_AUDIT_LOG).await? {
                                return Ok(None);
                            }

                            lazy_static! {
                                static ref ID_PATH: Path = Path::from("id");
                            }
                            let guild_id = ctx.event.guild_id;
                            let channel_id = ctx.gateway(&ID_PATH, extract_id)?;
                            let channel_id_str = channel_id.to_string();

                            // Run an audit log search to find the corresponding entry
                            let search = audit_log::SearchQuery {
                                entry_type: Some(AuditLogEvent::ChannelCreate),
                                // Use the timestamp of channel ID provisioning as its creation timestamp
                                target_timestamp: Some(architus_id::extract_timestamp(channel_id)),
                                // Channel create events are unique for the matching channel id
                                strategy: audit_log::Strategy::First,
                                ..audit_log::SearchQuery::new(guild_id, |entry: &AuditLogEntry| {
                                    entry.target_id.as_ref() == Some(&channel_id_str)
                                })
                            };
                            Ok(ctx.get_audit_log_entry(search).await.ok())
                        })
                    },
                    OnFailure::Or(None),
                )),
                timestamp: Source::gateway(
                    Path::from("id"),
                    chain(extract_id, timestamp_from_id),
                    OnFailure::Abort,
                ),
                reason: Source::audit_log(
                    Path::from("reason"),
                    extract::<Option<String>>,
                    OnFailure::Or(None),
                ),
                channel: Source::Constant(None),
                agent: Source::audit_log(
                    Path::from("user_id"),
                    chain(extract_id, |id, ctx| {
                        Ok(Some(Agent {
                            special_type: Agent::type_from_id(id, &ctx.config),
                            entity: Entity::UserLike(UserLike {
                                id,
                                ..UserLike::default()
                            }),
                        }))
                    }),
                    OnFailure::Or(None),
                ),
                subject: Source::sync_fn(
                    |ctx| {
                        lazy_static! {
                            static ref ID_PATH: Path = Path::from("id");
                            static ref NAME_PATH: Path = Path::from("name");
                        }
                        // Extract the id and name of the channel
                        let id = ctx.gateway(&ID_PATH, extract_id)?;
                        let name = ctx
                            .gateway(&NAME_PATH, extract::<Option<String>>)
                            .ok()
                            .flatten();
                        Ok(Some(Entity::Channel(Channel { id, name })))
                    },
                    OnFailure::Abort,
                ),
                auxiliary: Source::Constant(None),
                content: Source::sync_fn(
                    |ctx| {
                        lazy_static! {
                            static ref ID_PATH: Path = Path::from("id");
                            static ref TYPE_PATH: Path = Path::from("type");
                        }
                        let channel_type = ctx.gateway(&TYPE_PATH, extract::<ChannelType>)?;
                        let channel_type_str = channel_type_to_string(channel_type);
                        let id = ctx.gateway(&ID_PATH, extract_id)?;
                        let mention = channel_mention(id, channel_type);
                        Ok(Content {
                            inner: format!("created {} ({})", mention, channel_type_str),
                            channels_mentioned: vec![id],
                            ..Content::default()
                        })
                    },
                    OnFailure::Abort,
                ),
            },
        )
        .register(
            GatewayEventType::ChannelDelete,
            Processor {
                event_type: Source::Constant(EventType::ChannelDelete),
                audit_log: Some(AuditLogSource::new(
                    |ctx| {
                        Box::pin(async move {
                            // Make sure that the bot has permissions first
                            if !ctx.has_perms(Permissions::VIEW_AUDIT_LOG).await? {
                                return Ok(None);
                            }

                            lazy_static! {
                                static ref ID_PATH: Path = Path::from("id");
                            }
                            let guild_id = ctx.event.guild_id;
                            let channel_id = ctx.gateway(&ID_PATH, extract_id)?;
                            let channel_id_str = channel_id.to_string();

                            // Run an audit log search to find the corresponding entry
                            let search = audit_log::SearchQuery {
                                entry_type: Some(AuditLogEvent::ChannelDelete),
                                // Channel delete events are unique for the matching channel id
                                strategy: audit_log::Strategy::First,
                                ..audit_log::SearchQuery::new(guild_id, |entry: &AuditLogEntry| {
                                    entry.target_id.as_ref() == Some(&channel_id_str)
                                })
                            };
                            Ok(ctx.get_audit_log_entry(search).await.ok())
                        })
                    },
                    OnFailure::Or(None),
                )),
                timestamp: Source::sync_fn(|ctx| Ok(ctx.event.ingress_timestamp), OnFailure::Abort),
                reason: Source::audit_log(
                    Path::from("reason"),
                    extract::<Option<String>>,
                    OnFailure::Or(None),
                ),
                channel: Source::Constant(None),
                agent: Source::audit_log(
                    Path::from("user_id"),
                    chain(extract_id, |id, ctx| {
                        Ok(Some(Agent {
                            special_type: Agent::type_from_id(id, &ctx.config),
                            entity: Entity::UserLike(UserLike {
                                id,
                                ..UserLike::default()
                            }),
                        }))
                    }),
                    OnFailure::Or(None),
                ),
                subject: Source::sync_fn(
                    |ctx| {
                        lazy_static! {
                            static ref ID_PATH: Path = Path::from("id");
                            static ref NAME_PATH: Path = Path::from("name");
                        }
                        // Extract the id and name of the channel
                        let id = ctx.gateway(&ID_PATH, extract_id)?;
                        let name = ctx
                            .gateway(&NAME_PATH, extract::<Option<String>>)
                            .ok()
                            .flatten();
                        Ok(Some(Entity::Channel(Channel { id, name })))
                    },
                    OnFailure::Abort,
                ),
                auxiliary: Source::Constant(None),

                content: Source::sync_fn(
                    |ctx| {
                        lazy_static! {
                            static ref ID_PATH: Path = Path::from("id");
                            static ref TYPE_PATH: Path = Path::from("type");
                        }
                        let channel_type = ctx.gateway(&TYPE_PATH, extract::<ChannelType>)?;
                        let channel_type_str = channel_type_to_string(channel_type);
                        let id = ctx.gateway(&ID_PATH, extract_id)?;
                        let mention = channel_mention(id, channel_type);
                        Ok(Content {
                            inner: format!("deleted {} ({})", mention, channel_type_str),
                            channels_mentioned: vec![id],
                            ..Content::default()
                        })
                    },
                    OnFailure::Abort,
                ),
            },
        )
        .register(
            GatewayEventType::ChannelUpdate,
            Processor {
                event_type: Source::Constant(EventType::ChannelUpdate),
                audit_log: Some(AuditLogSource::new(
                    |ctx| {
                        Box::pin(async move {
                            // Make sure that the bot has permissions first
                            if !ctx.has_perms(Permissions::VIEW_AUDIT_LOG).await? {
                                return Ok(None);
                            }

                            lazy_static! {
                                static ref ID_PATH: Path = Path::from("id");
                            }
                            let guild_id = ctx.event.guild_id;
                            let channel_id = ctx.gateway(&ID_PATH, extract_id)?;
                            let channel_id_str = channel_id.to_string();

                            // Run an audit log search to find the corresponding entry
                            let search = audit_log::SearchQuery {
                                entry_type: Some(AuditLogEvent::ChannelUpdate),
                                strategy: audit_log::Strategy::GrowingInterval {
                                    max: Duration::from_secs(3),
                                },
                                ..audit_log::SearchQuery::new(guild_id, |entry: &AuditLogEntry| {
                                    entry.target_id.as_ref() == Some(&channel_id_str)
                                })
                            };
                            Ok(ctx.get_audit_log_entry(search).await.ok())
                        })
                    },
                    OnFailure::Or(None),
                )),
                timestamp: Source::sync_fn(|ctx| Ok(ctx.event.ingress_timestamp), OnFailure::Abort),
                reason: Source::audit_log(
                    Path::from("reason"),
                    extract::<Option<String>>,
                    OnFailure::Or(None),
                ),
                channel: Source::Constant(None),
                agent: Source::audit_log(
                    Path::from("user_id"),
                    chain(extract_id, |id, ctx| {
                        Ok(Some(Agent {
                            special_type: Agent::type_from_id(id, &ctx.config),
                            entity: Entity::UserLike(UserLike {
                                id,
                                ..UserLike::default()
                            }),
                        }))
                    }),
                    OnFailure::Or(None),
                ),
                subject: Source::sync_fn(
                    |ctx| {
                        lazy_static! {
                            static ref ID_PATH: Path = Path::from("id");
                            static ref NAME_PATH: Path = Path::from("name");
                        }
                        // Extract the id and name of the channel
                        let id = ctx.gateway(&ID_PATH, extract_id)?;
                        let name = ctx
                            .gateway(&NAME_PATH, extract::<Option<String>>)
                            .ok()
                            .flatten();
                        Ok(Some(Entity::Channel(Channel { id, name })))
                    },
                    OnFailure::Abort,
                ),
                auxiliary: Source::Constant(None),
                content: Source::sync_fn(
                    |ctx| {
                        lazy_static! {
                            static ref ID_PATH: Path = Path::from("id");
                            static ref TYPE_PATH: Path = Path::from("type");
                        }
                        // TODO update
                        let channel_type = ctx.gateway(&TYPE_PATH, extract::<ChannelType>)?;
                        let channel_type_str = channel_type_to_string(channel_type);
                        let id = ctx.gateway(&ID_PATH, extract_id)?;
                        let mention = channel_mention(id, channel_type);
                        Ok(Content {
                            inner: format!("deleted {} ({})", mention, channel_type_str),
                            channels_mentioned: vec![id],
                            ..Content::default()
                        })
                    },
                    OnFailure::Abort,
                ),
            },
        )
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
fn timestamp_from_id(id: u64, _ctx: Context<'_>) -> Result<u64, anyhow::Error> {
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

/// Provides a human-readable version of a channel type
fn channel_type_to_string(channel_type: ChannelType) -> &'static str {
    match channel_type {
        ChannelType::Group => "group chat",
        ChannelType::GuildCategory => "channel category",
        ChannelType::GuildNews => "announcement channel",
        ChannelType::GuildStore => "store channel",
        ChannelType::GuildText => "text channel",
        ChannelType::GuildVoice => "voice channel",
        ChannelType::Private => "direct message channel",
    }
}

/// Formats a rich content channel mention
fn channel_mention(id: u64, channel_type: ChannelType) -> String {
    match channel_type {
        ChannelType::GuildVoice => format!("<#v{}", id),
        ChannelType::GuildCategory => format!("<#c{}>", id),
        _ => format!("<#{}>", id),
    }
}
