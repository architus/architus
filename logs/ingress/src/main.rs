mod env;
mod event;

use env::Environment;
use log::{error, info};
use serenity::async_trait;
use serenity::framework::standard::StandardFramework;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

/// Bootstraps the bot and begins listening for gateway events
#[tokio::main]
async fn main() {
    env_logger::init();
    let env = Environment::load();

    let framework = StandardFramework::new();
    let mut client = Client::new(&env.token)
        .event_handler(StatusHandler)
        .raw_event_handler(event::Handler::new())
        .framework(framework)
        .intents(*event::INTENTS)
        .await
        .expect("error creating client");

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

struct StatusHandler;

#[async_trait]
impl EventHandler for StatusHandler {
    /// Called once bot connects and is ready
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.tag());
    }
}
