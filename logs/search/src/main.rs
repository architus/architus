#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

mod config;
mod elasticsearch_api;
mod graphql;
mod stored_event;
mod logging {
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    tonic::include_proto!("logging");
}

use crate::config::Configuration;
use crate::graphql::SearchProvider;
use crate::stored_event::StoredEvent;
use anyhow::{Context, Result};
use architus_id::{time, IdProvisioner};
use elasticsearch::http::response::Response as ElasticResponse;
use elasticsearch::http::transport::Transport;
use elasticsearch::http::StatusCode;
use elasticsearch::params::OpType;
use elasticsearch::{Elasticsearch, IndexParts};
use futures::try_join;
use juniper::http::GraphQLRequest;
use juniper::InputValue;
use log::{debug, info, warn};
use logging::logging_server::{Logging, LoggingServer};
use logging::{SearchRequest, SearchResponse, SubmitRequest, SubmitResponse};
use std::convert::TryInto;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::result::Result as StdResult;
use std::sync::Arc;
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
    let es_path = &config.services.elasticsearch;
    info!("Connecting to Elasticsearch at {}", es_path);
    let es_transport =
        Transport::single_node(es_path).context("Could not connect to Elasticsearch")?;
    let elasticsearch = Arc::new(Elasticsearch::new(es_transport));

    // Create the search provider and pass it into both servers
    // (cloning it is cheat since it uses Arc<> internally)
    let search = SearchProvider::new(&elasticsearch, &config);
    let grpc_future = serve_grpc(&elasticsearch, search.clone(), &config);
    let http_future = serve_http(search.clone(), &config);

    try_join!(grpc_future, http_future)?;
    Ok(())
}

/// Starts the main `gRPC` server using tonic that is used to import and search the logs
async fn serve_grpc(
    elasticsearch: &Arc<Elasticsearch>,
    search: SearchProvider,
    config: &Configuration,
) -> Result<()> {
    // Start the server on the specified port
    let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), config.port);
    let logging = LoggingService::new(elasticsearch, search, &config.log_index);
    let service = LoggingServer::new(logging);
    let server = Server::builder().add_service(service);

    info!("Serving gRPC server at {}", addr);
    server
        .serve(addr)
        .await
        .context("An error occurred while running the gRPC server")?;

    Ok(())
}

/// Starts the embedded HTTP server if configured,
/// used to serve the `GraphiQL` utility in development
/// in addition to the normal graphql route needed to make it function
async fn serve_http(search: SearchProvider, config: &Configuration) -> Result<()> {
    use hyper::service::{make_service_fn, service_fn};
    use hyper::{
        Body, Method, Response as HttpResponse, Server as HttpServer, StatusCode as HttpStatusCode,
    };

    if let Some(port) = config.graphql.http_port {
        let addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port);
        let new_service = make_service_fn(move |_| {
            let search = search.clone();
            async {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    let search = search.clone();
                    async move {
                        match (req.method(), req.uri().path()) {
                            (&Method::GET, "/") => juniper_hyper::graphiql("/graphql", None).await,
                            (&Method::GET, "/playground") => {
                                juniper_hyper::playground("/graphql", None).await
                            }
                            (&Method::GET, "/graphql") | (&Method::POST, "/graphql") => {
                                let context = Arc::new(search.context(None, None));
                                juniper_hyper::graphql(search.schema(), context, req).await
                            }
                            _ => {
                                let mut response = HttpResponse::new(Body::empty());
                                *response.status_mut() = HttpStatusCode::NOT_FOUND;
                                Ok(response)
                            }
                        }
                    }
                }))
            }
        });

        let server = HttpServer::bind(&addr).serve(new_service);
        info!("Serving GraphQL HTTP at http://{}", addr);
        server
            .await
            .context("An error occurred while running the HTTP server")?;
    }

    Ok(())
}

struct LoggingService {
    id_provisioner: IdProvisioner,
    elasticsearch: Arc<Elasticsearch>,
    search: SearchProvider,
    index: String,
}

impl LoggingService {
    fn new(
        elasticsearch: &Arc<Elasticsearch>,
        search: SearchProvider,
        index: impl AsRef<str>,
    ) -> Self {
        Self {
            id_provisioner: IdProvisioner::new(),
            elasticsearch: Arc::clone(elasticsearch),
            search,
            index: String::from(index.as_ref()),
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
    ) -> StdResult<Response<SubmitResponse>, Status> {
        let timestamp = time::millisecond_ts();
        let mut event = request
            .into_inner()
            .event
            .ok_or_else(|| Status::invalid_argument("no event given"))?;

        // Add in a timestamp if needed
        if event.timestamp == 0 {
            event.timestamp = timestamp;
        }

        // Provision an Id if needed
        if event.id == 0 {
            event.id = self.id_provisioner.with_ts(timestamp).0;
        }

        // Convert the event format into the stored version.
        // This deserializes the inner source JSON from the protobuf struct
        // into the serde_json::Value so that the source JSON is properly nested
        // Note that the protobuf definitions do not use the generic Struct message
        // (from the Google well-known types) because it only supports f64 numbers,
        // which may cause problems in the future.
        // Instead, the internal JSON is sent as a string
        let stored_event: StoredEvent = event.try_into().map_err(|err| {
            let message = format!("could not parse event: {:?}", err);
            Status::internal(message)
        })?;

        // Insert into the ES database,
        // regenerating the ID if needed (by incrementing by 1)
        // TODO perform batched inserts
        debug!("Received event from RPC call: {:?}", stored_event);
        loop {
            let event_json = serde_json::to_value(&stored_event).map_err(|err| {
                let message = format!("could not serialize Event into JSON: {:?}", err);
                Status::internal(message)
            })?;
            let response = self
                .elasticsearch
                .index(IndexParts::IndexId(
                    &self.index,
                    &stored_event.id.to_string(),
                ))
                .body(&event_json)
                .op_type(OpType::Create)
                .send()
                .await
                .map_err(|err| {
                    let message = format!("sending log event to data store failed: {:?}", err);
                    Status::unavailable(message)
                })?;

            return match ElasticResponse::error_for_status_code(response) {
                Ok(_) => Ok(Response::new(SubmitResponse {
                    actual_id: stored_event.id.into(),
                    actual_timestamp: stored_event.timestamp,
                })),
                Err(err) => {
                    if err.status_code() == Some(StatusCode::CONFLICT) {
                        // Try again with an incremented ID
                        stored_event.id.0.checked_add(1).ok_or_else(|| {
                            let message = "cannot attempt to remove ID conflict: reached max ID";
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

    /// Submits a GraphQL search request to the service
    /// and retrieves the results.
    /// Note that this does not support mutations
    async fn search(
        &self,
        request: Request<SearchRequest>,
    ) -> StdResult<Response<SearchResponse>, Status> {
        // Build the context from the params
        let request = request.into_inner();
        let channel_whitelist = if request.enable_channel_id_whitelist {
            Some(request.channel_id_whitelist)
        } else {
            None
        };
        let context = self
            .search
            .context(Some(request.guild_id), channel_whitelist);

        // Build the GraphQL request
        let query = request.query;
        let operation_name = Some(request.operation_name).filter(String::is_empty);
        let variables_json = request.variables_json;
        let variables = if variables_json.is_empty() {
            None
        } else {
            Some(
                serde_json::from_str::<InputValue>(&variables_json).map_err(|err| {
                    let message = format!("cannot decode variables JSON: {:?}", err);
                    Status::invalid_argument(message)
                })?,
            )
        };
        let graphql_request = GraphQLRequest::new(query, operation_name, variables);

        // Execute the request
        let schema = self.search.schema();
        let graphql_response = graphql_request.execute(&schema, &context).await;
        let response = SearchResponse {
            is_ok: graphql_response.is_ok(),
            result_json: serde_json::to_string(&graphql_response).map_err(|err| {
                let message = format!("cannot encode result JSON: {:?}", err);
                Status::data_loss(message)
            })?,
        };

        Ok(Response::new(response))
    }
}
