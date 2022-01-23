#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)]

mod comic;
mod config;
mod connect;
mod timeout;

use crate::config::Configuration;
use anyhow::Context;
use deadpool_postgres::Pool;
use futures_util::stream::StreamExt;
use serde::Deserialize;
use slog::Logger;
use sloggers::Config;
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use twilight_gateway::{Event, Intents, Shard};
use twilight_http::Client;
use twilight_model::application::callback::InteractionResponse;
use twilight_model::application::command::{
    BaseCommandOptionData, ChoiceCommandOptionData, CommandOption, CommandOptionChoice,
};
use twilight_model::application::component::button::ButtonStyle;
use twilight_model::application::component::{ActionRow, Button, Component};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::{ApplicationCommand, Interaction};
use twilight_model::id::{ApplicationId, GuildId, InteractionId, UserId};

/// Loads the config and bootstraps the service
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse the config
    let config_path = std::env::args().nth(1).expect(
        "no config path given \
        \nUsage: \
        \nshard-rs [config-path]",
    );
    let config = Arc::new(Configuration::try_load(&config_path)?);

    // Set up the logger from the config
    let logger = config
        .logging
        .build_logger()
        .context("could not build logger from config values")?;

    slog::info!(
        logger,
        "starting service";
        "config_path" => config_path,
        "arguments" => ?std::env::args().collect::<Vec<_>>(),
    );
    slog::debug!(logger, "configuration dump"; "config" => ?config);
    slog::debug!(logger, "env dump"; "env" => ?std::env::vars().collect::<Vec<_>>());

    match run(config, logger.clone()).await {
        Ok(_) => slog::info!(logger, "service exited";),
        Err(err) => {
            slog::error!(
                logger,
                "an error occurred during service execution";
                "error" => ?err,
            );
        }
    }
    Ok(())
}

/// Attempts to initialize the bot and listen for gateway events
#[allow(clippy::too_many_lines)]
async fn run(config: Arc<Configuration>, logger: Logger) -> anyhow::Result<()> {
    let database_pool = connect::connect_to_db(Arc::clone(&config), logger.clone()).await?;
    let bot = BotServer::new(config, logger, database_pool);

    bot.create_commands()
        .await
        .context("could not create commands")?;
    bot.listen_for_events()
        .await
        .context("could not listen for events")?;

    Ok(())
}

struct BotServer {
    logger: Logger,
    discord_api_client: Client,
    config: Arc<Configuration>,
    database_pool: Pool,
    gulag_votes: Mutex<BTreeMap<InteractionId, GulagVoteState>>,
}

struct GulagVoteState {
    target: UserId,
    votes: BTreeSet<UserId>,
    threshold: u64,
    severity: u64,
    is_treason: bool,
    expires_after: Duration,
    started_at: Instant,
}

enum InteractionSuccess {
    Single(InteractionResponse),
    Multiple(Vec<InteractionResponse>),
}

struct InteractionError {
    custom_message: Option<String>,
    inner: anyhow::Error,
    fields: Option<slog::OwnedKVList>,
}

impl From<anyhow::Error> for InteractionError {
    fn from(other: anyhow::Error) -> Self {
        Self {
            custom_message: None,
            inner: other,
            fields: None,
        }
    }
}

impl BotServer {
    fn new(config: Arc<Configuration>, logger: Logger, database_pool: Pool) -> Arc<Self> {
        Arc::new(Self {
            logger,
            discord_api_client: Client::builder()
                .token(config.discord_token.clone())
                .application_id(ApplicationId(config.discord_app_id))
                .timeout(Duration::from_secs(5))
                .build(),
            config,
            database_pool,
            gulag_votes: Mutex::new(BTreeMap::new()),
        })
    }

    async fn create_commands(&self) -> anyhow::Result<()> {
        let command = self
            .discord_api_client
            .create_global_command("comic")?
            .chat_input(&self.config.comic_description)?
            .command_options(&[CommandOption::String(ChoiceCommandOptionData {
                autocomplete: false,
                choices: vec![
                    CommandOptionChoice::String {
                        name: String::from("XKCD"),
                        value: String::from("source_xkcd"),
                    },
                    CommandOptionChoice::String {
                        name: String::from("SMBC"),
                        value: String::from("source_smbc"),
                    },
                ],
                description: String::from("The type of webcomic to view"),
                name: String::from("source"),
                required: true,
            })])?
            .exec()
            .await?
            .model()
            .await?;

        slog::info!(
            self.logger,
            "created new command";
            "command_name" => command.name,
        );

        let command2 = self
            .discord_api_client
            .create_guild_command(GuildId(self.config.temp_test_guild), "Start gulag vote")?
            .user()
            .exec()
            .await?
            .model()
            .await?;

        slog::info!(
            self.logger,
            "created new command on test guild";
            "command_name" => command2.name,
            "guild_id" => u64::from(self.config.temp_test_guild),
        );

        let command3 = self
            .discord_api_client
            .create_guild_command(GuildId(self.config.temp_test_guild), "gulag")?
            .chat_input(&self.config.gulag_description)?
            .command_options(&[CommandOption::User(BaseCommandOptionData {
                description: String::from("The user to gulag"),
                name: String::from("user"),
                required: true,
            })])?
            .exec()
            .await?
            .model()
            .await?;

        slog::info!(
            self.logger,
            "created new command on test guild";
            "command_name" => command3.name,
            "guild_id" => u64::from(self.config.temp_test_guild),
        );

        Ok(())
    }

    fn handle_interaction<F>(
        self: Arc<Self>,
        id: InteractionId,
        command: Arc<ApplicationCommand>,
        fut: F,
    ) where
        F: Future<Output = Result<InteractionSuccess, InteractionError>> + Send + 'static,
    {
        tokio::task::spawn(async move {
            let mut logger = self.logger.new(slog::o!(
                "guild_id" => command.guild_id.map_or(0, |i| u64::from(i.0)),
                "interaction_id" => u64::from(id.0),
                "channel_id" => u64::from(command.channel_id.0),
                "user_id" => command.member.as_ref()
                    .and_then(|m| m.user.as_ref())
                    .map_or(0, |u| u64::from(u.id.0)),
            ));

            match fut.await {
                Ok(success) => {
                    let responses = match success {
                        InteractionSuccess::Single(response) => vec![response],
                        InteractionSuccess::Multiple(responses) => responses,
                    };

                    for (i, response) in responses.iter().enumerate() {
                        let result = self
                            .discord_api_client
                            .interaction_callback(id, &command.token, &response)
                            .exec()
                            .await;

                        match result {
                            Ok(_) => {
                                slog::info!(
                                    logger,
                                    "responded to application command";
                                    "command" => &command.data.name,
                                    "response_idx" => i,
                                );
                            }
                            Err(err) => {
                                slog::warn!(
                                    logger,
                                    "failed to respond to application command";
                                    "error" => ?err,
                                    "command" => &command.data.name,
                                    "response_idx" => i,
                                );
                            }
                        }
                    }
                }
                Err(err) => {
                    if let Some(fields) = err.fields {
                        logger = logger.new(slog::OwnedKV(fields));
                    }

                    let message = err.custom_message.unwrap_or_else(|| {
                        String::from("An unexpected error occurred when processing this command :(")
                    });

                    slog::warn!(
                        logger,
                        "failed to process application command";
                        "error" => ?err.inner,
                        "user_message" => &message,
                    );

                    let callback_data = twilight_util::builder::CallbackDataBuilder::new()
                        .content(message)
                        .build();
                    let result = self
                        .discord_api_client
                        .interaction_callback(
                            id,
                            &command.token,
                            &InteractionResponse::ChannelMessageWithSource(callback_data),
                        )
                        .exec()
                        .await;

                    if let Err(err) = result {
                        slog::warn!(
                            logger,
                            "failed to notify user of command failure";
                            "error" => ?err,
                            "command" => &command.data.name,
                        );
                    }
                }
            }
        });
    }

    async fn listen_for_events(self: Arc<Self>) -> anyhow::Result<()> {
        let intents = Intents::empty();
        let (shard, mut events) = Shard::new(self.config.discord_token.clone(), intents);

        shard.start().await?;
        slog::info!(
            self.logger,
            "started shard";
        );

        while let Some(event) = events.next().await {
            if let Event::InteractionCreate(interaction_create_event) = event {
                let interaction_id = interaction_create_event.id();
                let guild_id = interaction_create_event.guild_id();

                let logger = self.logger.new(slog::o!(
                    "guild_id" => guild_id.map_or(0, |i| u64::from(i.0)),
                    "interaction_id" => u64::from(interaction_id.0)
                ));

                let self_clone = Arc::clone(&self);
                match interaction_create_event.0 {
                    Interaction::Ping(_) => {
                        slog::info!(logger, "received Interaction::Ping from Discord gateway");
                    }
                    Interaction::ApplicationCommand(command) => {
                        let command_arc: Arc<ApplicationCommand> = Arc::from(command);
                        match command_arc.data.name.as_ref() {
                            "comic" => Arc::clone(&self).handle_interaction(
                                interaction_id,
                                Arc::clone(&command_arc),
                                self_clone.handle_comic_command(command_arc),
                            ),
                            "gulag" => Arc::clone(&self).handle_interaction(
                                interaction_id,
                                Arc::clone(&command_arc),
                                self_clone.handle_gulag_text_command(interaction_id, command_arc),
                            ),
                            "Start gulag vote" => Arc::clone(&self).handle_interaction(
                                interaction_id,
                                Arc::clone(&command_arc),
                                self_clone.handle_gulag_user_command(interaction_id, command_arc),
                            ),
                            _ => {
                                slog::warn!(
                                    logger,
                                    "received unknown command interaction!";
                                    "name" => &command_arc.data.name,
                                );
                            }
                        }
                    }
                    Interaction::ApplicationCommandAutocomplete(_) => {
                        slog::warn!(
                            logger,
                            "received Interaction::ApplicationCommandAutocomplete; ignoring";
                        );
                    }
                    Interaction::MessageComponent(_) => {
                        slog::warn!(
                            logger,
                            "received Interaction::MessageComponent; ignoring";
                        );
                    }
                    _ => {
                        slog::warn!(
                            logger,
                            "received unknown Interaction::_; ignoring";
                        );
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_comic_command(
        self: Arc<Self>,
        command: Arc<ApplicationCommand>,
    ) -> Result<InteractionSuccess, InteractionError> {
        let option_option = command.data.options.iter().find(|o| o.name == "source");
        let comic_source = if let Some(option) = option_option {
            if let CommandOptionValue::String(s) = &option.value {
                s.clone()
            } else {
                return Err(InteractionError {
                    custom_message: None,
                    inner: anyhow::anyhow!("received interaction with non-string source option"),
                    fields: Some(slog::o!("comic_source" => format!("{:?}", option.value)).into()),
                });
            }
        } else {
            return Err(InteractionError {
                custom_message: None,
                inner: anyhow::anyhow!("received interaction without source option present"),
                fields: None,
            });
        };

        let comic_data_result = match comic_source.as_ref() {
            "source_xkcd" => comic::get_latest_xkcd().await,
            "source_smbc" => comic::get_latest_smbc().await,
            _ => {
                return Err(InteractionError {
                    custom_message: None,
                    inner: anyhow::anyhow!(
                        "received comic interaction with unknown source specified"
                    ),
                    fields: Some(slog::o!("comic_source" => comic_source.clone()).into()),
                });
            }
        };

        match comic_data_result {
            Ok(result) => {
                let callback_data = twilight_util::builder::CallbackDataBuilder::new()
                    .embeds(vec![result.into()])
                    .build();
                Ok(InteractionSuccess::Single(
                    InteractionResponse::ChannelMessageWithSource(callback_data),
                ))
            }
            Err(err) => Err(InteractionError {
                custom_message: Some(String::from(
                    "An unexpected error occurred while fetching the comic",
                )),
                inner: err.context("failed to obtain error"),
                fields: Some(slog::o!("comic_source" => comic_source.clone()).into()),
            }),
        }
    }

    async fn handle_gulag_text_command(
        self: Arc<Self>,
        interaction_id: InteractionId,
        command: Arc<ApplicationCommand>,
    ) -> Result<InteractionSuccess, InteractionError> {
        let guild_id = if let Some(id) = &command.guild_id {
            *id
        } else {
            // TODO add fluff senate response if gulag-ing Architus, even in DM
            return Err(InteractionError {
                custom_message: Some(String::from("Gulag is only valid in non-DM contexts")),
                inner: anyhow::anyhow!("received gulag text interaction in non-guild setting"),
                fields: None,
            });
        };

        let option_option = command.data.options.iter().find(|o| o.name == "user");
        let target_user_id = if let Some(option) = option_option {
            if let CommandOptionValue::User(id) = &option.value {
                *id
            } else {
                return Err(InteractionError {
                    custom_message: None,
                    inner: anyhow::anyhow!(
                        "received gulag text interaction with non-id user option"
                    ),
                    fields: Some(slog::o!("target_user" => format!("{:?}", option.value)).into()),
                });
            }
        } else {
            return Err(InteractionError {
                custom_message: None,
                inner: anyhow::anyhow!(
                    "received gulag text interaction without user option present"
                ),
                fields: None,
            });
        };

        self.handle_gulag(guild_id, interaction_id, command, target_user_id)
            .await
    }

    async fn handle_gulag_user_command(
        self: Arc<Self>,
        interaction_id: InteractionId,
        command: Arc<ApplicationCommand>,
    ) -> Result<InteractionSuccess, InteractionError> {
        let guild_id = if let Some(id) = &command.guild_id {
            *id
        } else {
            return Err(InteractionError {
                custom_message: None,
                inner: anyhow::anyhow!("received gulag user interaction in non-guild setting"),
                fields: None,
            });
        };

        let target_user_id = if let Some(resolved) = command.data.resolved.as_ref() {
            if let Some(id) = resolved.users.keys().next() {
                *id
            } else {
                return Err(InteractionError {
                    custom_message: None,
                    inner: anyhow::anyhow!(
                        "received gulag user interaction without user in data.resolved.users"
                    ),
                    fields: None,
                });
            }
        } else {
            return Err(InteractionError {
                custom_message: None,
                inner: anyhow::anyhow!(
                    "received gulag user interaction without data.resolved field present"
                ),
                fields: None,
            });
        };

        self.handle_gulag(guild_id, interaction_id, command, target_user_id)
            .await
    }

    #[allow(clippy::too_many_lines)]
    async fn handle_gulag(
        self: Arc<Self>,
        guild_id: GuildId,
        interaction_id: InteractionId,
        command: Arc<ApplicationCommand>,
        target_user_id: UserId,
    ) -> Result<InteractionSuccess, InteractionError> {
        // TODO validate getting this in all contexts
        let sender = command
            .member
            .as_ref()
            .and_then(|m| m.user.as_ref())
            .map(|u| u.id);

        let logger = self.logger.new(slog::o!(
            "interaction_id" => u64::from(interaction_id.0),
            "channel_id" => u64::from(command.channel_id.0),
            "guild_id" => u64::from(guild_id.0),
            "user_id" => sender.map_or(0, |id| u64::from(id.0)),
            "target_user_id" => u64::from(target_user_id.0),
        ));

        let client = timeout::timeout(Duration::from_secs(2), self.database_pool.get())
            .await
            .context("failed to get a database connection handle")?;
        let row_result = client
            .query_one(
                "SELECT json_blob FROM tb_settings WHERE guild_id=$1",
                &[&i64::try_from(u64::from(guild_id.0))
                    .context("could not convert guild_id to i64 to send to Postgres")?],
            )
            .await;

        let (threshold, severity) = if let Ok(row) = row_result {
            let value_result: Result<String, _> = row.try_get(0);
            if let Ok(value) = value_result {
                #[derive(Deserialize)]
                struct GulagSettings {
                    gulag_threshold: Option<u64>,
                    gulag_severity: Option<u64>,
                }

                match serde_json::from_str::<GulagSettings>(&value) {
                    Ok(settings) => (
                        settings
                            .gulag_threshold
                            .unwrap_or(self.config.default_gulag_threshold),
                        settings
                            .gulag_severity
                            .unwrap_or(self.config.default_gulag_severity),
                    ),
                    Err(err) => {
                        slog::warn!(
                            logger,
                            "failed to deserialize tb_settings json value";
                            "row" => ?row,
                            "value" => value,
                            "error" => ?err,
                        );
                        (
                            self.config.default_gulag_threshold,
                            self.config.default_gulag_severity,
                        )
                    }
                }
            } else {
                slog::warn!(logger, "no values obtained from tb_settings row"; "row" => ?row);
                (
                    self.config.default_gulag_threshold,
                    self.config.default_gulag_severity,
                )
            }
        } else {
            (
                self.config.default_gulag_threshold,
                self.config.default_gulag_severity,
            )
        };

        let is_treason = target_user_id == UserId(self.config.architus_user_id);
        let new_state = GulagVoteState {
            target: {
                if is_treason {
                    // TODO make sender non-optional
                    sender.unwrap_or(UserId(unsafe { NonZeroU64::new_unchecked(1) }))
                } else {
                    target_user_id
                }
            },
            votes: {
                let mut m = BTreeSet::new();
                // TODO remove conditional
                if !is_treason {
                    if let Some(sender) = sender {
                        m.insert(sender);
                    }
                }
                m
            },
            threshold,
            severity,
            is_treason,
            // TODO add to config
            expires_after: Duration::from_secs(20 * 60),
            started_at: Instant::now(),
        };

        let current_len = u64::try_from(new_state.votes.len()).unwrap_or(0);

        slog::info!(logger, "starting gulag vote";);
        let mut gulag_votes = self.gulag_votes.lock().expect("gulag votes mutex poisoned");
        gulag_votes.insert(interaction_id, new_state);
        std::mem::drop(gulag_votes);

        // TODO lookup nickname of member with ID
        // TODO check if threshold is already met
        let vote_callback_data = twilight_util::builder::CallbackDataBuilder::new()
            .content(format!(
                "{} more votes to gulag {}",
                threshold.saturating_sub(current_len),
                target_user_id.0,
            ))
            .components(vec![Component::ActionRow(ActionRow {
                components: vec![Component::Button(Button {
                    custom_id: Some(String::from("gulag_vote")),
                    disabled: false,
                    emoji: None,
                    label: Some(String::from("Cast your vote")),
                    style: ButtonStyle::Primary,
                    url: None,
                })],
            })])
            .build();
        if is_treason {
            // TODO add back once Twilight people respond
            // to message in their Discord:
            // https://discord.com/channels/745809834183753828/745811192102125578/934647199726456893
            // let treason_callback_data = twilight_util::builder::CallbackDataBuilder::new()
            //     .content(format!(
            //         "{} more votes to gulag {}",
            //         new_state
            //             .threshold
            //             .saturating_sub(u64::try_from(new_state.votes.len()).unwrap_or(0)),
            //         target_user_id.0,
            //     ))
            //     .components(vec![Component::ActionRow(ActionRow {
            //         components: vec![Component::Button(Button {
            //             custom_id: Some(String::from("gulag_vote")),
            //             disabled: false,
            //             emoji: None,
            //             label: Some(String::from("Cast your vote")),
            //             style: ButtonStyle::Primary,
            //             url: None,
            //         })],
            //     })])
            //     .build();

            // Ok(InteractionSuccess::Multiple(vec![
            //     InteractionResponse::ChannelMessageWithSource(treason_callback_data),
            //     InteractionResponse::ChannelMessageWithSource(vote_callback_data),
            // ]))
            slog::info!(logger, "treason gulag!");
            Ok(InteractionSuccess::Single(
                InteractionResponse::ChannelMessageWithSource(vote_callback_data),
            ))
        } else {
            Ok(InteractionSuccess::Single(
                InteractionResponse::ChannelMessageWithSource(vote_callback_data),
            ))
        }
    }
}
