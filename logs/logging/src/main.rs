#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

mod config;
mod stored_event;
mod logging {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    tonic::include_proto!("logging");
}

use crate::config::Configuration;
use crate::stored_event::StoredEvent;
use anyhow::{Context, Result};
use elasticsearch::http::response::Response as ElasticResponse;
use elasticsearch::http::transport::Transport;
use elasticsearch::http::StatusCode;
use elasticsearch::params::OpType;
use elasticsearch::{Elasticsearch, IndexParts};
use log::{debug, info, warn};
use logging::logging_server::{Logging, LoggingServer};
use logging::{SubmitReply, SubmitRequest};
use logs_lib::id::IdProvisioner;
use logs_lib::time;
use std::convert::TryInto;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::result::Result as StdResult;
use std::time::Duration;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

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
    let elasticsearch = config.services.elasticsearch;
    info!("Connecting to Elasticsearch at {}", elasticsearch);
    let elasticsearch_transport =
        Transport::single_node(&elasticsearch).context("Could not connect to Elasticsearch")?;
    let elasticsearch_client = Elasticsearch::new(elasticsearch_transport);

    // Start the server on the specified port
    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), config.port);
    let logging = LoggingService::new(elasticsearch_client);
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
    elasticsearch: Elasticsearch,
}

impl LoggingService {
    fn new(elasticsearch: Elasticsearch) -> Self {
        Self {
            id_provisioner: IdProvisioner::new(),
            elasticsearch,
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
        if let Some(event) = event {
            // Convert the event formats
            // This deserializes the inner source JSON from the protobuf struct
            // into the serde_json::Value so that the source JSON is properly nested
            // Note that the protobuf definitions do not use the generic Struct message
            // (from the Google well-known types) because it only supports f64 numbers,
            // which may cause problems in the future.
            // Instead, the internal JSON is sent as a string
            let mut stored_event: StoredEvent = event.try_into().map_err(|err| {
                let message = format!("could not deserialize inner source JSON: {:?}", err);
                Status::internal(message)
            })?;

            // Add in a timestamp if needed
            if stored_event.timestamp == 0 {
                stored_event.timestamp = timestamp;
            }

            // Provision an Id if needed
            if stored_event.id == 0 {
                stored_event.timestamp = self.id_provisioner.with_ts(timestamp).0;
            }

            // Insert into the ES database,
            // regenerating the ID if needed (by incrementing by 1)
            debug!("Received event from RPC call: {:?}", stored_event);
            loop {
                let event_json = serde_json::to_value(&stored_event).map_err(|err| {
                    let message = format!("could not serialize Event into JSON: {:?}", err);
                    Status::internal(message)
                })?;
                let response = self
                    .elasticsearch
                    .index(IndexParts::IndexId("events", &stored_event.id.to_string()))
                    .body(&event_json)
                    .op_type(OpType::Create)
                    .send()
                    .await
                    .map_err(|err| {
                        let message = format!("sending log event to data store failed: {:?}", err);
                        Status::unavailable(message)
                    })?;

                return match ElasticResponse::error_for_status_code(response) {
                    Ok(_) => Ok(Response::new(SubmitReply {
                        actual_id: stored_event.id,
                        actual_timestamp: stored_event.timestamp,
                    })),
                    Err(err) => {
                        if err.status_code() == Some(StatusCode::CONFLICT) {
                            // Try again with an incremented ID
                            stored_event.id.checked_add(1).ok_or_else(|| {
                                let message =
                                    "cannot attempt to remove ID conflict: reached max ID";
                                Status::data_loss(message)
                            })?;
                            tokio::time::delay_for(Duration::from_millis(100)).await;
                            continue;
                        }

                        warn!("Inserting into Elasticsearch failed: {:?}", err);
                        let message = format!("sending log event to data store failed: {:?}", err);
                        Err(Status::internal(message))
                    }
                };
            }
        }

        Err(Status::invalid_argument("no event given to import"))
    }
}
