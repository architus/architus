#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)]

mod config;
mod rpc;

use crate::config::Configuration;
use crate::rpc::uptime::uptime_service_server::{UptimeService, UptimeServiceServer};
use crate::rpc::uptime::{GatewaySubmitRequest, GatewaySubmitResponse};
use anyhow::{Context, Result};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tonic::transport::Server;
use tonic::{Request, Response};

/// Loads the config and bootstraps the service
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Parse the config
    let config_path = std::env::args().nth(1).expect(
        "no config path given \
        \nUsage: \
        \nlogs-uptime [config-path]",
    );
    let config = Arc::new(Configuration::try_load(config_path)?);
    run(config).await
}

/// Attempts to initialize the bot and start the gRPC server
async fn run(config: Arc<Configuration>) -> Result<()> {
    // Start the server on the specified port
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.port);
    let uptime_service = Uptime::new();
    let uptime_server = UptimeServiceServer::new(uptime_service);
    let server = Server::builder().add_service(uptime_server);

    log::info!("Serving gRPC server at {}", addr);
    server
        .serve(addr)
        .await
        .context("An error occurred while running the gRPC server")?;

    Ok(())
}

struct Uptime {}

impl Uptime {
    fn new() -> Self {
        Self {}
    }
}

#[tonic::async_trait]
impl UptimeService for Uptime {
    async fn gateway_submit(
        &self,
        request: Request<GatewaySubmitRequest>,
    ) -> Result<Response<GatewaySubmitResponse>, tonic::Status> {
        let request = request.into_inner();

        // TODO consume request
        log::info!("Received gateway submit request: {:?}", request);
        Ok(Response::new(GatewaySubmitResponse {}))
    }
}
