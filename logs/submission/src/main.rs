#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)]

mod config;
mod rpc;

use crate::config::Configuration;
use crate::rpc::submission::submission_service_server::{
    SubmissionService, SubmissionServiceServer,
};
use crate::rpc::submission::{SubmitIdempotentRequest, SubmitIdempotentResponse};
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
    let submission_service = Submission::new();
    let submission_server = SubmissionServiceServer::new(submission_service);
    let server = Server::builder().add_service(submission_server);

    log::info!("Serving gRPC server at {}", addr);
    server
        .serve(addr)
        .await
        .context("An error occurred while running the gRPC server")?;

    Ok(())
}

struct Submission {}

impl Submission {
    fn new() -> Self {
        Self {}
    }
}

#[tonic::async_trait]
impl SubmissionService for Submission {
    async fn submit_idempotent(
        &self,
        request: Request<SubmitIdempotentRequest>,
    ) -> Result<Response<SubmitIdempotentResponse>, tonic::Status> {
        let request = request.into_inner();

        // TODO consume request
        log::info!("Received idempotent submission: {:?}", request);
        Ok(Response::new(SubmitIdempotentResponse {}))
    }
}
