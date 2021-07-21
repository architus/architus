use log::{info, warn, LevelFilter};
use simple_logger::SimpleLogger;

mod manager;
mod receiver;
mod zipper;

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::Mutex;

use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
    framework::{
        standard::{macros::command, Args, CommandResult},
        StandardFramework,
    },
    model::{
        channel::Message,
        gateway::Ready,
        id::{ChannelId, GuildId},
        voice::VoiceState,
    },
    prelude::*,
    Result as SerenityResult,
};

use songbird::{
    driver::{Config as DriverConfig, DecodeMode},
    CoreEvent, SerenityInit, Songbird,
};

// Songbird is kinda wack. This is a struct that is used for
// tracking what voice channels architus is in. Prevents
// architus from trying to be in multiple vs on the same guild.
struct ArchitusState;

impl TypeMapKey for ArchitusState {
    type Value = HashMap<u64, Option<u64>>;
}

// More gross stuff for keeping track of architus state in vcs.
// This ones keeps track of current record handlers that
// are recording in voice channels.
struct RecordState;

impl TypeMapKey for RecordState {
    type Value = HashMap<u64, receiver::Recording>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected", ready.user.name);
    }

    // Tracks the status of architus being in a voice channel.
    // Used to ensure that architus doesn't randomly get yanked
    // out of a vc where it's playing some music.
    async fn voice_state_update(
        &self,
        ctx: Context,
        guild: Option<GuildId>,
        _: Option<VoiceState>,
        new: VoiceState,
    ) {
        let guild_id = match guild {
            Some(id) => id,
            None => return,
        };

        let bot_id = ctx.cache.current_user().await.id.0;
        if bot_id != new.user_id.0 {
            return;
        }

        let mut state = ctx.data.write().await;
        let curr_state = state.get_mut::<ArchitusState>().unwrap();
        let vc = curr_state.entry(guild_id.0).or_insert(None);
        if let Some(vid) = new.channel_id {
            *vc = Some(vid.0);
        } else {
            *vc = None;
        }
    }
}

/// Start a recording in a voice channel.
/// Takes as an argument the id of the voice channel in which to record.
/// Requires that the bot either not be in a voice channel previously or
/// to already be in the voice channel to record.
#[command]
#[only_in(guilds)]
async fn record(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    // Ensure that a voice channel id was passed
    let connect_to = match args.single::<u64>() {
        Ok(id) => ChannelId(id),
        Err(_) => {
            check_msg(
                msg.reply(ctx, "Need to pass a valid voice channel ID")
                    .await,
            );
            return Ok(());
        }
    };

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    // Need a scope so that `state` is not double borrowed.
    // This scope checks to ensure that the prereqs are satisfied.
    let mut state = ctx.data.write().await;
    {
        let curr_state = state.get_mut::<ArchitusState>().unwrap();
        let curr_vc = curr_state.entry(guild_id.0).or_insert(None);

        if let Some(id) = *curr_vc {
            if id != connect_to.0 {
                check_msg(
                    msg.reply(ctx, "Can't record while in a different voice channel")
                        .await,
                );
                return Ok(());
            }
        }
    }

    // Set up a recorder to keep track of all the audio that architus receives.
    let bot_id = ctx.cache.current_user().await.id.0;
    let recordings = state.get_mut::<RecordState>().unwrap();
    // TODO: Get actual disallowed ids from database.
    let recorder = receiver::Recording(Arc::new(Mutex::new(receiver::WAVReceiver::new(
        vec![],
        bot_id,
    ))));
    let recorder_handler = receiver::Recording(recorder.0.clone());
    recordings.insert(guild_id.0, recorder);

    let manager = songbird::get(ctx)
        .await
        .expect("Registered at initialization")
        .clone();

    let (handler_lock, conn_result) = manager.join(guild_id, connect_to).await;

    // Add all of the events that should be passed on to the recorder
    if let Ok(_) = conn_result {
        let mut handler = handler_lock.lock().await;

        // I really don't like having to clone the recorder all of these times.
        // However, songbird decided to be a major pain in the ass and its the
        // only way I can think of to get it to work. :)
        handler.add_global_event(
            CoreEvent::ClientConnect.into(),
            receiver::Recording(recorder_handler.0.clone()),
        );

        handler.add_global_event(
            CoreEvent::ClientDisconnect.into(),
            receiver::Recording(recorder_handler.0.clone()),
        );

        handler.add_global_event(
            CoreEvent::VoicePacket.into(),
            receiver::Recording(recorder_handler.0.clone()),
        );

        handler.add_global_event(
            CoreEvent::SpeakingStateUpdate.into(),
            receiver::Recording(recorder_handler.0.clone()),
        );
    }

    info!("Started recording in guild {}", guild_id.0);

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn stop_record(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird loaded at startup");

    let has_handler = manager.get(guild_id).is_some();
    if !has_handler {
        check_msg(msg.reply(ctx, "No recording was started").await);
    }

    let mut state = ctx.data.write().await;
    let recordings = state.get_mut::<RecordState>().unwrap();
    let recorder_handle = match recordings.remove(&guild_id.0) {
        Some(r) => r,
        None => {
            info!("Lost a recorder. That's not good. :(");
            check_msg(msg.reply(ctx, "Recording failed. Sorry").await);
            return Ok(());
        }
    };

    let mut recorder = recorder_handle.0.lock().await;
    let bytes = recorder.recording_size();
    if bytes == 0 {
        check_msg(msg.reply(ctx, "Nothing was recorded").await);
        return Ok(());
    }

    let file = recorder.save();

    match file {
        Ok((url, pass)) => check_msg(
            msg.reply(ctx, format!("File at {} with pw {}", url, pass))
                .await,
        ),
        Err(e) => {
            warn!("Encountered error while writing file: {}", e);
        }
    };

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    sleep(Duration::from_secs(10));
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .with_module_level("mic", LevelFilter::Info)
        .init()
        .unwrap();
    info!("Starting recording microservice");

    let config_path = std::env::args().nth(1).expect(
        "no config path given \
        \nUsage: \
        \nrecord-service [config-path]",
    );

    let config = Configuration::try_load(&config_path)?);

    let framework = StandardFramework::new().configure(|c| c.prefix("!"));

    let songbird = Songbird::serenity();
    songbird.set_config(DriverConfig::default().decode_mode(DecodeMode::Decode));

    let mut client = Client::builder(&config.secrets.discord_token)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird_with(songbird.into())
        .await
        .expect("Err creating client");

    let _ = client
        .start()
        .await
        .map_err(|why| println!("Client ended: {:?}", why));
}

// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        warn!("Error sending message: {:?}", why);
    }
}
