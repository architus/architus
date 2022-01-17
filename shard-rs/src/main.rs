#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)]

mod comic;
mod config;

use crate::config::Configuration;
use anyhow::Context;
use futures_util::stream::StreamExt;
use slog::Logger;
use sloggers::Config;
use std::sync::Arc;
use std::time::Duration;
use twilight_gateway::{Event, Intents, Shard};
use twilight_http::Client;
use twilight_model::application::callback::InteractionResponse;
use twilight_model::application::command::{
    ChoiceCommandOptionData, CommandOption, CommandOptionChoice,
};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::{ApplicationCommand, Interaction};
use twilight_model::id::{ApplicationId, InteractionId};

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
    let bot = BotServer::new(config, logger);

    bot.create_commands()
        .await
        .context("could not create commands")?;
    bot.listen_for_events()
        .await
        .context("could not listen for events")?;

    Ok(())
}

pub struct BotServer {
    logger: Logger,
    discord_api_client: Client,
    config: Arc<Configuration>,
}

impl BotServer {
    fn new(config: Arc<Configuration>, logger: Logger) -> Arc<Self> {
        Arc::new(Self {
            logger,
            discord_api_client: Client::builder()
                .token(config.discord_token.clone())
                .application_id(ApplicationId(config.discord_app_id))
                .timeout(Duration::from_secs(5))
                .build(),
            config,
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

        Ok(())
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

                match interaction_create_event.0 {
                    Interaction::ApplicationCommand(command) => match command.data.name.as_ref() {
                        "comic" => {
                            tokio::task::spawn(
                                Arc::clone(&self).handle_comic_command(interaction_id, command),
                            );
                        }
                        _ => {
                            slog::warn!(
                                logger,
                                "received unknown command interaction!";
                                "name" => command.data.name,
                            );
                        }
                    },
                    _ => {
                        slog::warn!(
                            logger,
                            "received non-application command interaction!";
                        );
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_comic_command(
        self: Arc<Self>,
        interaction_id: InteractionId,
        command: Box<ApplicationCommand>,
    ) {
        let mut logger = self.logger.new(slog::o!(
            "guild_id" => command.guild_id.map_or(0, |i| u64::from(i.0)),
            "interaction_id" => u64::from(interaction_id.0),
            "channel_id" => u64::from(command.channel_id.0),
        ));

        let option_option = command.data.options.iter().find(|o| o.name == "source");
        let comic_source = if let Some(option) = option_option {
            if let CommandOptionValue::String(s) = &option.value {
                s.clone()
            } else {
                slog::warn!(
                    logger,
                    "received interaction with non-string source option; not responding";
                    "comic_source" => ?option.value,
                );
                return;
            }
        } else {
            slog::warn!(
                logger,
                "received interaction without source option present; not responding";
            );
            return;
        };

        logger = logger.new(slog::o!("comic_source" => comic_source.clone()));

        let comic_data_result = match comic_source.as_ref() {
            "source_xkcd" => comic::get_latest_xkcd().await,
            "source_smbc" => comic::get_latest_smbc().await,
            _ => {
                slog::warn!(
                    logger,
                    "received interaction with unknown source specified; not responding";
                );
                return;
            }
        };

        let response = match comic_data_result {
            Ok(result) => twilight_util::builder::CallbackDataBuilder::new()
                .embeds(vec![result.into()])
                .build(),
            Err(err) => {
                slog::warn!(
                    logger,
                    "failed to obtain comic";
                    "error" => ?err,
                );
                twilight_util::builder::CallbackDataBuilder::new()
                    .content(String::from(
                        "An unexpected error occurred while fetching the comic",
                    ))
                    .build()
            }
        };

        let result = self
            .discord_api_client
            .interaction_callback(
                interaction_id,
                &command.token,
                &InteractionResponse::ChannelMessageWithSource(response),
            )
            .exec()
            .await;

        match result {
            Ok(_) => {
                slog::info!(
                    logger,
                    "responded to interaction with message";
                );
            }
            Err(err) => {
                slog::warn!(
                    logger,
                    "failed to respond to interaction with message";
                    "error" => ?err,
                );
            }
        }
    }
}
