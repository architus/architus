#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::future_not_send)]

mod config;
mod rpc;

use crate::config::Configuration;
use crate::rpc::logs::submission::submission_service_server::{
    SubmissionService, SubmissionServiceServer,
};
use crate::rpc::logs::submission::{SubmitIdempotentRequest, SubmitIdempotentResponse};
use anyhow::{Context, Result};
use bytes::Bytes;
use prost::Message;
use reqwest::{Client, StatusCode};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

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
    let submission_service = Submission::new(Arc::clone(&config));
    let submission_server = SubmissionServiceServer::new(submission_service);
    let server = Server::builder().add_service(submission_server);

    log::info!("Serving gRPC server at {}", addr);
    server
        .serve(addr)
        .await
        .context("An error occurred while running the gRPC server")?;

    Ok(())
}

/// Possible errors while attempting to send an event to logstash
#[derive(thiserror::Error, Debug)]
pub enum LogstashSendError {
    #[error("the HTTP request to logstash failed")]
    Network(#[source] reqwest::Error),
    #[error("the downstream logstash queue is full")]
    QueueFull,
    #[error("an internal error occurred in logstash")]
    InternalError(String),
    #[error("invalid credentials were supplied to logstash")]
    InvalidCredentials,
}

struct Submission {
    config: Arc<Configuration>,
    http_client: Client,
}

impl Submission {
    fn new(config: Arc<Configuration>) -> Self {
        Self {
            config,
            http_client: Client::new(),
        }
    }

    async fn send_to_logstash(&self, body: Bytes) -> Result<(), LogstashSendError> {
        let response = self
            .http_client
            .post(&self.config.services.logs_submission_logstash)
            .body(body)
            .send()
            .await
            .map_err(LogstashSendError::Network)?;
        match response.status() {
            StatusCode::TOO_MANY_REQUESTS => Err(LogstashSendError::QueueFull),
            StatusCode::UNAUTHORIZED => Err(LogstashSendError::InvalidCredentials),
            StatusCode::INTERNAL_SERVER_ERROR => {
                let body = response.text().await.unwrap_or_else(|err| {
                    log::warn!(
                        "Could not read internal error response body from logstash: {:?}",
                        err
                    );
                    String::from("<error reading response body>")
                });
                Err(LogstashSendError::InternalError(body))
            }
            _ => Ok(()),
        }
    }
}

#[tonic::async_trait]
impl SubmissionService for Submission {
    async fn submit_idempotent(
        &self,
        request: Request<SubmitIdempotentRequest>,
    ) -> Result<Response<SubmitIdempotentResponse>, tonic::Status> {
        let request = request.into_inner();

        // TODO consume additional metadata and send to revision service

        // Serialize the inner event to protobuf
        if let Some(event) = request.event.as_ref().and_then(|e| e.inner.as_ref()) {
            let mut buf = Vec::<u8>::with_capacity(event.encoded_len());
            event.encode(&mut buf).map_err(|err| {
                let message = format!("could not encode event to protobuf {:?}", err);
                log::warn!(
                    "Could not serialize proto due to {:?}. Original request: {:?} ",
                    err,
                    request
                );
                Status::internal(message)
            })?;
            let body_bytes = Bytes::from(buf);

            // Send the event to logstash
            log::info!("Forwarding event with id '{}' to logstash", event.id);
            // let rmq_url = self.config.services.logs_submission_logstash.clone();
            let send = || async {
                self.send_to_logstash(body_bytes.clone())
                    .await
                    .map_err(|err| match err {
                        LogstashSendError::InvalidCredentials
                        | LogstashSendError::InternalError(_) => backoff::Error::Permanent(err),
                        LogstashSendError::Network(_) | LogstashSendError::QueueFull => {
                            backoff::Error::Transient(err)
                        }
                    })
            };
            backoff::future::retry(self.config.submission_backoff.build(), send)
                .await
                .map_err(|err| {
                    let message = format!("could not send request to logstash {:?}", err);
                    log::warn!("Could not send request to logstash: {:?}", err);
                    Status::unavailable(message)
                })?;

            Ok(Response::new(SubmitIdempotentResponse {}))
        } else {
            Err(Status::invalid_argument("no inner event in submission"))
        }
    }
}
