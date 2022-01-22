mod config;
mod scrape;
mod utils;
mod convert;

use anyhow::Context;
use twilight_http::Client;
use scrape_manager::work::Work;
use slog::Logger;
use sloggers::Config;
use tokio::time::sleep;

use std::time::Duration;

#[tokio::main(flavor="multi_thread", worker_threads=4)]
async fn main() {
    let config_path = std::env::args().nth(1).expect("no config path given");
    let config = config::Configuration::try_load(&config_path).expect("Couldn't parse config file");

    let logger = config
        .logging
        .build_logger()
        .context("could not build logger from config vbalues")
        .unwrap();

    slog::info!(
        logger,
        "starting service";
        "config_path" => config_path,
        "arguments" => ?std::env::args().collect::<Vec<_>>(),
    );
    slog::info!(logger, "manager uri"; "uri" => config.manager_uri);
    slog::info!(logger, "num worker threads"; "workers" => config.worker_threads);

    let work_uri = config.manager_uri + "/work";
}

// Main loop for consuming work from the manager
async fn worker(token: String, uri: String, success_time: Duration, fail_time: Duration, logger: Logger) -> ! {
    let client = Client::new(token.clone());

    'main: loop {
        let work: Work;
        let body = reqwest::get(&uri).await;
        if let Ok(resp) = body {
            work = if let Ok(w) = resp.json::<Work>().await {
                w
            } else {
                slog::warn!(logger, "failed to parse work from manager");
                sleep(fail_time).await;
                continue 'main;
            }
        } else {
            slog::warn!(logger, "failed to connect to work manager");
            sleep(fail_time).await;
            continue 'main;
        }

        if work.0.0 == 0 {
            slog::info!(logger, "no work to be done");
            sleep(success_time).await;
            continue 'main;
        }

        if work.1.0 == 0 {
            slog::info!(logger, "scraping audit log"; "guild id" => work.0.0; "start" => work.2.0; "end" => work.2.1);
            if let Ok(events) = scrape::scrape_timespan(&client, work.0, work.2).await {
            } else {
                slog::err!(logger, "failed audit query"; "guild id" => work.0.0);
                sleep(fail_time).await;
                continue 'main;
            }
        }


        sleep(success_time).await;
    }
}
