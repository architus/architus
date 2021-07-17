#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

mod config;
mod connect;
mod elasticsearch_api;
mod graphql;
mod proto;
mod stored_event;

use crate::config::Configuration;
use crate::graphql::SearchProvider;
use anyhow::Context;
use hyper::http::{Method, Request, Response, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Error, Server};
use slog::Logger;
use sloggers::Config;
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
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
    serve_http(search.clone(), Arc::clone(&config), logger.clone()).await?;

    Ok(())
}

/// Starts the embedded HTTP server if configured,
/// used to serve the GraphQL playground utility in development
/// in addition to the normal graphql route needed to make it function
async fn serve_http(
    search: SearchProvider,
    config: Arc<Configuration>,
    logger: Logger,
) -> anyhow::Result<()> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.graphql.http_port);
    let new_service = make_service_fn(move |_| {
        let search = search.clone();
        async {
            Ok::<_, Infallible>(service_fn(move |req| {
                let search = search.clone();
                async { Ok::<_, Error>(handle_http(req, search).await) }
            }))
        }
    });

    let server = Server::bind(&addr).serve(new_service);
    slog::info!(logger, "serving GraphQL HTTP"; "address" => ?addr);
    server
        .await
        .context("an error occurred while running the HTTP server")?;

    Ok(())
}

/// Hyper HTTP handler function used to serve the GraphQL playground & GraphQL query API
async fn handle_http(req: Request<Body>, search: SearchProvider) -> Response<Body> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/playground") => juniper_hyper::playground("/graphql", None).await,
        (&Method::GET, "/graphql") | (&Method::POST, "/graphql") => {
            let context = Arc::new(search.context(None, None));
            juniper_hyper::graphql(search.schema(), context, req).await
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
    }
}
