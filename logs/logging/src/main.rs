mod config;
mod logging {
    tonic::include_proto!("logging");
}

use crate::config::Configuration;
use anyhow::{Context, Result};
use log::info;
use logging::logging_server::{Logging, LoggingServer};
use logging::{SubmitReply, SubmitRequest};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::result::Result as StdResult;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

/// Attempts to initialize the service and listen for RPC requests
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Load the configuration
    let config_path = std::env::args().nth(1).expect(
        "no config path given\
        Usage: \
        logging-service [config-path]",
    );
    let config = Configuration::try_load(config_path)?;

    // Start the server on the specified port
    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), config.port);
    let logging = LoggingService {};
    let service = LoggingServer::new(logging);
    let server = Server::builder().add_service(service);

    info!("Serving gRPC server at {}", addr);
    server
        .serve(addr)
        .await
        .context("An error occurred while running the gRPC server")?;

    Ok(())
}

#[derive(Debug)]
struct LoggingService;

#[tonic::async_trait]
impl Logging for LoggingService {
    /// Submit a single log event to be imported into the Elasticsearch database.
    /// Will automatically provision an ID if not given and fill in a timestamp,
    /// ensuring that the ID of the LogEvent is unique within the data store
    /// (re-generating it if necessary)
    async fn submit(
        &self,
        request: Request<SubmitRequest>,
    ) -> StdResult<Response<SubmitReply>, Status> {
        let event = request.into_inner().event;

        // TODO implement
        info!("Received event from RPC call: {:?}", event);
        Ok(Response::new(SubmitReply {
            actual_id: 0,
            actual_timestamp: 0,
        }))
    }
}
