#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

mod config;
mod elasticsearch_api;
mod graphql;
mod proto;
mod stored_event;

use crate::config::Configuration;
use crate::graphql::SearchProvider;
use anyhow::{Context, Result};
use elasticsearch::http::transport::Transport;
use elasticsearch::Elasticsearch;
use hyper::http::{Method, Request, Response, StatusCode};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Error, Server,
};
use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

/// Attempts to initialize the service and listen for RPC requests
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Load the configuration
    let config_path = std::env::args().nth(1).expect(
        "no config path given \
        \nUsage: \
        \nlogging-service [config-path]",
    );
    let config = Configuration::try_load(config_path)?;

    // Connect to Elasticsearch
    let es_path = &config.services.elasticsearch;
    log::info!("Connecting to Elasticsearch at {}", es_path);
    let es_transport =
        Transport::single_node(es_path).context("Could not connect to Elasticsearch")?;
    let elasticsearch = Arc::new(Elasticsearch::new(es_transport));

    let search = SearchProvider::new(&elasticsearch, &config);
    serve_http(search.clone(), &config).await?;

    Ok(())
}

async fn handle_http(req: Request<Body>, search: SearchProvider) -> Response<Body> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => juniper_hyper::graphiql("/graphql", None).await,
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

/// Starts the embedded HTTP server if configured,
/// used to serve the `GraphiQL` utility in development
/// in addition to the normal graphql route needed to make it function
async fn serve_http(search: SearchProvider, config: &Configuration) -> Result<()> {
    if let Some(port) = config.graphql.http_port {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
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
        log::info!("Serving GraphQL HTTP at http://{}", addr);
        server
            .await
            .context("An error occurred while running the HTTP server")?;
    }

    Ok(())
}
