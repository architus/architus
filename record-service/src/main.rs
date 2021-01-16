use log::info;

mod handler;
mod manager;
mod zipper;

use std::env::args;
use std::thread::{sleep, spawn};
use std::time::Duration;

use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
    framework::{
        StandardFramework,
        standard::{
            macros::{command, group},
            Args, CommandResult,
        },
    },
    model::{
        channel::Message,
        gateway::Ready,
        id::ChannelId,
        misc::Mentionable,
    },
    Result as SerenityResult,
};

use songbird::{
    driver::{Config as DriverConfig, DecodeMode},

fn main() {
    sleep(Duration::from_secs(10));
    simple_logger::init_with_level(log::Level::Info).unwrap();
    info!("Starting recording microservice");

    let port = args().nth(1).expect("No port given");
    let port = port.parse::<u16>().expect("Didn't pass a valid number");
    assert!(port < 65535);

    let server = TcpListener::bind(format!("record:{}", port)).expect("Failed to bind");
    info!("Listening on port {}", port);

    for stream in server.incoming() {
        match stream {
            Ok(s) => {
                let h = handler::WAVReceiver::new();
                info!(
                    "Serving: {}",
                    s.peer_addr().expect("Peer doesn't have address")
                );
                spawn(move || h.handle(s));
            }
            Err(_) => break,
        }
    }
}
