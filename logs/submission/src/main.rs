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
use architus_id::IdProvisioner;
use bytes::Bytes;
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
    id_provisioner: IdProvisioner,
}

impl Submission {
    fn new(config: Arc<Configuration>) -> Self {
        Self {
            config,
            http_client: Client::new(),
            id_provisioner: IdProvisioner::new(),
        }
    }

    async fn send_to_logstash(
        &self,
        body: Bytes,
        mime_type: &str,
    ) -> Result<(), LogstashSendError> {
        let response = self
            .http_client
            .post(&self.config.services.logs_submission_logstash)
            .header("Content-Type", mime_type)
            .body(body)
            .send()
            .await
            .map_err(LogstashSendError::Network)?;
        // https://www.elastic.co/guide/en/logstash/current/plugins-inputs-http.html#plugins-inputs-http-response_code
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
        let timestamp = architus_id::time::millisecond_ts();
        let event = request
            .into_inner()
            .event
            .ok_or_else(|| Status::invalid_argument("no event given"))?;

        // TODO consume additional metadata in SubmittedEvent message
        //      and send to revision service

        let mut inner = event
            .inner
            .ok_or_else(|| Status::invalid_argument("no inner event given"))?;

        // guild_id is the only required field
        if inner.guild_id == 0 {
            return Err(Status::invalid_argument("missing guild_id on inner event"));
        }

        // Add in a timestamp if needed
        if inner.timestamp == 0 {
            inner.timestamp = timestamp;
        }

        // Provision an Id if needed
        if inner.id == 0 {
            inner.id = self.id_provisioner.with_ts(timestamp).0;
        }

        // Serialize the inner event to JSON
        let json = serde_json::to_vec(&inner).map_err(|err| {
            let message = format!("could not encode event to JSON {:?}", err);
            log::warn!(
                "Could not serialize JSON due to {:?}. Event: {:?} ",
                err,
                inner
            );
            Status::internal(message)
        })?;
        let json_body = Bytes::from(json);

        // Send the event to logstash
        let send = || async {
            self.send_to_logstash(json_body.clone(), "application/json")
                .await
                .map_err(|err| match err {
                    LogstashSendError::InvalidCredentials | LogstashSendError::InternalError(_) => {
                        backoff::Error::Permanent(err)
                    }
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

        log::info!(
            "Successfully forwarded event with id '{}' to logstash",
            inner.id
        );
        Ok(Response::new(SubmitIdempotentResponse {}))
    }
}
