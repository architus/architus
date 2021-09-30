#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

mod config;
mod connect;
mod elasticsearch_api;
mod fairings;
mod graphql;
mod rpc;
mod event;

use crate::config::Configuration;
use crate::graphql::SearchProvider;
use anyhow::Context;
use rocket::response::content::Html;
use rocket::response::status::BadRequest;
use rocket::serde::json::Json;
use rocket::State;
use slog::Logger;
use sloggers::Config;
use std::sync::Arc;

/// Loads the config and bootstraps the service
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse the config
    let config_path = std::env::args().nth(1).expect(
        "no config path given \
        \nUsage: \
        \nlogs-search [config-path]",
    );
    let config = Arc::new(Configuration::try_load(&config_path)?);

    // Set up the logger from the config
    let logger = config
        .logging
        .build_logger()
        .context("could not build logger from config values")?;

    slog::info!(logger, "configuration loaded"; "path" => config_path);
    slog::debug!(logger, "configuration dump"; "config" => ?config);

    match run(config, logger.clone()).await {
        Ok(_) => slog::info!(logger, "service exited";),
        Err(err) => {
            slog::error!(logger, "an error ocurred during service running"; "error" => ?err)
        }
    }
    Ok(())
}

/// Attempts to initialize the service and listen GraphQL requests
async fn run(config: Arc<Configuration>, logger: Logger) -> anyhow::Result<()> {
    // Connect to Elasticsearch
    let elasticsearch =
        Arc::new(connect::to_elasticsearch(Arc::clone(&config), logger.clone()).await?);

    let search = SearchProvider::new(&elasticsearch, Arc::clone(&config), logger.clone());
    rocket::custom(config.rocket.clone())
        .manage(search)
        .mount("/", rocket::routes![playground, post_graphql, get_graphql])
        .attach(fairings::request_id::Fairing::new())
        .attach(fairings::attach_logger::Fairing::new(logger.clone()))
        .attach(fairings::request_logging::Fairing::new(logger.clone()))
        .launch()
        .await
        .expect("server to launch");

    Ok(())
}

#[derive(serde::Serialize, Debug, Clone)]
struct ApiError {
    message: String,
}

#[rocket::get("/playground")]
fn playground() -> Html<String> {
    juniper_rocket::playground_source("/graphql/000000000000000000", None)
}

#[rocket::post("/graphql/<guild_id>", data = "<request>")]
async fn post_graphql(
    guild_id: u64,
    search: &State<SearchProvider>,
    request: Option<juniper_rocket::GraphQLRequest>,
) -> Result<juniper_rocket::GraphQLResponse, BadRequest<Json<ApiError>>> {
    match request {
        Some(request) => Ok(request
            .execute(search.schema(), &search.context(guild_id, None))
            .await),
        None => Err(BadRequest(Some(Json(ApiError {
            message: String::from("route requires JSON GraphQL request as body"),
        })))),
    }
}

#[rocket::get("/graphql/<guild_id>?<request..>")]
async fn get_graphql(
    guild_id: u64,
    search: &State<SearchProvider>,
    request: Option<juniper_rocket::GraphQLRequest>,
) -> Result<juniper_rocket::GraphQLResponse, BadRequest<Json<ApiError>>> {
    match request {
        Some(request) => Ok(request
            .execute(search.schema(), &search.context(guild_id, None))
            .await),
        None => Err(BadRequest(Some(Json(ApiError {
            message: String::from(
                "route requires JSON GraphQL request as 'request?' query parameter",
            ),
        })))),
    }
}
