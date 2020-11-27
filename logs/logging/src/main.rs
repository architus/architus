mod config;
mod logging {
    tonic::include_proto!("logging");
}

use crate::config::Configuration;
use anyhow::{Context, Result};
use log::info;
use logging::logging_server::{Logging, LoggingServer};
use logging::{SubmitReply, SubmitRequest};
use logs_lib::id::IdProvisioner;
use logs_lib::time;
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
    let logging = LoggingService::new();
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
struct LoggingService {
    id_provisioner: IdProvisioner,
}

impl Default for LoggingService {
    fn default() -> Self {
        Self::new()
    }
}

impl LoggingService {
    fn new() -> Self {
        Self {
            id_provisioner: IdProvisioner::new(),
        }
    }
}

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
        let timestamp = time::millisecond_ts();
        let event = request.into_inner().event;
        if let Some(mut event) = event {
            // Add in a timestamp if needed
            if event.timestamp == 0 {
                event.timestamp = timestamp;
            }

            // Provision an Id if needed
            if event.id == 0 {
                event.timestamp = self.id_provisioner.with_ts(timestamp).0;
            }

            // Insert into the ES database,
            // regenerating the ID if needed (by incrementing by 1)
            info!("Received event from RPC call: {:?}", event);
            return Ok(Response::new(SubmitReply {
                actual_id: event.id,
                actual_timestamp: event.timestamp,
            }));
            // TODO actually implement
            // loop {
            //     match self.database.insert(&event) {
            //         Ok(_) => {
            //             return Ok(Response::new(SubmitReply{
            //                 actual_id: event.id,
            //                 actual_timestamp: event.timestamp,
            //             }))
            //         },
            //         Err(err) => {
            //             warn!("Insertion of log event into Elasticsearch data store failed: {:?}", err);
            //             // Re-create the id and try again
            //             event.id += 1;
            //         }
            //     }
            // }
        }

        Err(Status::invalid_argument("No event given to import"))
    }
}
