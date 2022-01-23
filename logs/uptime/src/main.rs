#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)]

mod config;
mod rpc;

use crate::config::Configuration;
use crate::rpc::logs::uptime::uptime_service_server::{UptimeService, UptimeServiceServer};
use crate::rpc::logs::uptime::{GatewaySubmitRequest, GatewaySubmitResponse};
use anyhow::Context;
use slog::Logger;
use sloggers::Config;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tonic::transport::Server;
use tonic::{Request, Response};

/// Loads the config and bootstraps the service
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse the config
    let config_path = std::env::args().nth(1).expect(
        "no config path given \
        \nUsage: \
        \nlogs-uptime [config-path]",
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

/// Attempts to initialize the bot and start the gRPC server
async fn run(config: Arc<Configuration>, logger: Logger) -> anyhow::Result<()> {
    // Start the server on the specified port
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.port);
    let uptime_service = UptimeServiceImpl::new(logger.clone());
    serve_grpc(uptime_service, addr, logger.clone()).await?;

    Ok(())
}

/// Serves the main gRPC server
async fn serve_grpc(
    uptime_service: UptimeServiceImpl,
    addr: SocketAddr,
    logger: Logger,
) -> anyhow::Result<()> {
    let uptime_server = UptimeServiceServer::new(uptime_service);
    let server = Server::builder().add_service(uptime_server);

    slog::info!(logger, "serving gRPC server"; "address" => addr);
    server
        .serve(addr)
        .await
        .context("an error occurred while running the gRPC server")?;

    Ok(())
}

struct UptimeServiceImpl {
    logger: Logger,
}

impl UptimeServiceImpl {
    fn new(logger: Logger) -> Self {
        Self { logger }
    }
}

#[tonic::async_trait]
impl UptimeService for UptimeServiceImpl {
    async fn gateway_submit(
        &self,
        request: Request<GatewaySubmitRequest>,
    ) -> anyhow::Result<Response<GatewaySubmitResponse>, tonic::Status> {
        let request = request.into_inner();

        // TODO consume request
        // This service will eventually store them into a db
        // and provide query access.
        // This was omitted from the backend MVP, however

        slog::info!(self.logger, "received gateway submit request"; "request" => ?request);
        Ok(Response::new(GatewaySubmitResponse {}))
    }
}
