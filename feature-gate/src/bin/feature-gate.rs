//! gRPC server for interfacing with the feature flags database.
//!
//! Due to the diesel connection struct not being sync or send,
//! the server will establish a new db connection for each request.
//! If the server is not able to establish a connection to the
//! database it will return some kind of error message.
//! The specifics of what the error message are can be found
//! in the specific documentation for each function.

#![deny(clippy::all)]

use db::*;
use diesel::r2d2::ConnectionManager;
use diesel::{Connection, PgConnection};
use log::{info, warn};
use r2d2::PooledConnection;
use std::env;
use tokio::sync::mpsc;
use tonic::{transport::Server, Request, Response, Status};

use crate::rpc::feature_gate::feature_gate_server::{FeatureGate, FeatureGateServer};
use crate::rpc::feature_gate::*;
use backoff::future::FutureOperation;
use backoff::ExponentialBackoff;

type RpcResponse<T> = Result<Response<T>, Status>;

/// Structure for handling all of the gRPC requests.
/// Holds a connection pool that can issue RAII connection handles
pub struct Gate {
    connection_pool: r2d2::Pool<ConnectionManager<PgConnection>>,
}

impl Gate {
    fn get_connection(&self) -> Option<PooledConnection<ConnectionManager<PgConnection>>> {
        self.connection_pool.get().ok()
    }
}

#[tonic::async_trait]
impl FeatureGate for Gate {
    /// Adds a new feature to the feature database.
    ///
    /// If the feature is successfully added to the database a success is returned.
    /// If the database is not able to be reached or any other error occurs while
    /// trying to add the feature, false will be returned to indicate the feature
    /// was not added.
    async fn create_feature(&self, request: Request<Feature>) -> RpcResponse<CreationResult> {
        let success = Ok(Response::new(CreationResult { success: true }));
        let failure = Ok(Response::new(CreationResult { success: false }));
        let conn = match self.get_connection() {
            Some(c) => c,
            None => return failure,
        };

        let feature = request.into_inner();
        let result = insert_feature(&conn, &feature.name, feature.open);

        match result {
            Ok(_) => {
                info!(
                    "Added feature {} which is {}",
                    feature.name,
                    if feature.open { "open" } else { "closed" }
                );
                success
            }
            Err(_) => failure,
        }
    }

    /// Checks to see if a feature is open or closed.
    ///
    /// If the client asks about a feature that is not in the database, the default value of
    /// false is returned to the client. If the client is not able to reach the database,
    /// a status of internal error is returned to indicate that things went wrong.
    async fn check_openness(&self, request: Request<FeatureName>) -> RpcResponse<OpennessResult> {
        let conn = match self.get_connection() {
            Some(c) => c,
            None => return Err(Status::internal("DB connection failed")),
        };

        let feature_name = request.into_inner().name;
        let result = get_feature_openness(&conn, &feature_name);

        match result {
            Ok(open) => Ok(Response::new(OpennessResult { open })),
            Err(DatabaseError::UnknownFeature) => Ok(Response::new(OpennessResult { open: false })),
            Err(_) => Err(Status::internal("DB Connection failed")),
        }
    }

    /// Creates a new guild <-> feature association in the database.
    ///
    /// On any type of error, the `AddResult` response will containe a false value to
    /// indicate that the addition of the feature to the guild was not successful.
    async fn add_guild_feature(&self, request: Request<FeatureAddition>) -> RpcResponse<AddResult> {
        let success = Ok(Response::new(AddResult { success: true }));
        let failure = Ok(Response::new(AddResult { success: false }));
        let addition = request.into_inner();
        let conn = match self.get_connection() {
            Some(c) => c,
            None => return failure,
        };

        let result = insert_guild_feature(&conn, addition.guild_id as i64, &addition.feature_name);
        match result {
            Ok(()) => {
                info!(
                    "Added feature {} to guild {}",
                    addition.feature_name, addition.guild_id
                );
                success
            }
            Err(_) => failure,
        }
    }

    /// Removes a guild <-> feature association from the database.
    ///
    /// Has the same type of error response as `add_guild_feature`.
    async fn remove_guild_feature(
        &self,
        request: Request<FeatureRemoval>,
    ) -> RpcResponse<RemoveResult> {
        let success = Ok(Response::new(RemoveResult { success: true }));
        let failure = Ok(Response::new(RemoveResult { success: false }));
        let conn = match self.get_connection() {
            Some(c) => c,
            None => return failure,
        };

        let removal = request.into_inner();
        let result = remove_guild_feature(&conn, removal.guild_id as i64, &removal.feature_name);
        match result {
            Ok(()) => {
                info!(
                    "Removed feature {} from guild {}",
                    removal.feature_name, removal.guild_id
                );
                success
            }
            Err(_) => failure,
        }
    }

    /// Checks to see if a guild has a certain feature.
    ///
    /// Will return a true/false value on successfully querying the database.
    /// If the feature asked about does not exist, then it will return a
    /// default value of false.
    /// If the database is not able to be contacted, then it will return an
    /// internal server error status code.
    async fn check_guild_feature(
        &self,
        request: Request<GuildFeature>,
    ) -> RpcResponse<FeatureResult> {
        let conn = match self.get_connection() {
            Some(c) => c,
            None => return Err(Status::internal("Database connection failed")),
        };

        let check = request.into_inner();
        let result = check_guild_feature(&conn, check.guild_id as i64, &check.feature_name);
        match result {
            Ok(b) => Ok(Response::new(FeatureResult { has_feature: b })),
            Err(DatabaseError::UnknownFeature) => {
                Ok(Response::new(FeatureResult { has_feature: false }))
            }
            Err(_) => Err(Status::internal("Database connection failed")),
        }
    }

    /// Checks to see if a list of guilds have a certain feature.
    ///
    /// Will return a list of true/false values on successfully querying the database
    /// in the same order as the provided guild id list.
    /// If the feature asked about does not exist, then it will return a
    /// default value of false for each guild.
    /// If the database is not able to be contacted, then it will return an
    /// internal server error status code.
    ///
    /// The number of supported guilds to check at once is at least 256, but may be more.
    async fn batch_check_guild_features(
        &self,
        request: Request<BatchCheck>,
    ) -> RpcResponse<BatchCheckResult> {
        let conn = match self.get_connection() {
            Some(c) => c,
            None => return Err(Status::internal("Database connection failed")),
        };
        let batch_check = request.into_inner();

        // Limit the number of items supported at once
        if batch_check.guild_ids.len() > 256 {
            return Err(Status::invalid_argument(format!(
                "Batched check operation only supports 256 guild-checks at once (provided {})",
                batch_check.guild_ids.len()
            )));
        }

        let result = batch_check_guild_feature(
            &conn,
            &batch_check
                .guild_ids
                .iter()
                .map(|id| *id as i64)
                .collect::<Vec<_>>(),
            &batch_check.feature_name,
        );
        match result {
            Ok(list) => Ok(Response::new(BatchCheckResult { has_feature: list })),
            Err(DatabaseError::UnknownFeature) => Ok(Response::new(BatchCheckResult {
                has_feature: vec![false; batch_check.guild_ids.len()],
            })),
            Err(_) => Err(Status::internal("Database connection failed")),
        }
    }

    // A type association for doing a streaming response.
    type GetFeaturesStream = mpsc::Receiver<Result<Feature, Status>>;

    /// Returns a stream of all of the features on the database.
    ///
    /// Returns a stream of each feature. The client will then receive an iterator
    /// of features that will go through everything that this function returns.
    /// Should return all features but may stop early if the client drops
    /// the iterator before all features have been read.
    async fn get_features(&self, _: Request<FeatureList>) -> RpcResponse<Self::GetFeaturesStream> {
        let conn = match self.get_connection() {
            Some(c) => c,
            None => return Err(Status::internal("Database connection failed")),
        };

        let features = match get_all_features(&conn) {
            Ok(v) => v,
            Err(_) => return Err(Status::internal("Database connection failed")),
        };

        let (mut tx, rx) = mpsc::channel(8);
        tokio::spawn(async move {
            for feat in features {
                match tx
                    .send(Ok(Feature {
                        name: feat.name,
                        open: feat.open,
                    }))
                    .await
                {
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });

        Ok(Response::new(rx))
    }

    // Stream type for returning a feature stream. Basically the same thing as for
    // the `get_features` stream.
    type GetGuildFeaturesStream = mpsc::Receiver<Result<Feature, Status>>;

    /// Sends all of the features associated with a guild.
    ///
    /// See `get_features` documentation for exactly how a stream works.
    /// If the guild id is not in the database, an empty stream will be
    /// sent to the client.
    async fn get_guild_features(
        &self,
        request: Request<Guild>,
    ) -> RpcResponse<Self::GetGuildFeaturesStream> {
        let conn = match self.get_connection() {
            Some(c) => c,
            None => return Err(Status::internal("Database connection failed")),
        };

        let guild_id = request.into_inner().guild_id;
        let features = match get_guild_features(&conn, guild_id as i64) {
            Ok(v) => v,
            Err(_) => return Err(Status::internal("Database connection failed")),
        };

        let (mut tx, rx) = mpsc::channel(8);
        tokio::spawn(async move {
            for feat in features {
                match tx
                    .send(Ok(Feature {
                        name: feat.0,
                        open: feat.1,
                    }))
                    .await
                {
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });

        Ok(Response::new(rx))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Need to limit to Level or above to prevent tonic from bombarding the logs with
    // hundreds of lines of debug output.
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let db_usr = env::var("db_user").expect("Need to have an architus.env file");
    let db_pass = env::var("db_pass").expect("Need to have an architus.env file");
    let db_host = "postgres";
    let db_port = 5432_u16;
    let db_name = "autbot";
    let database_url = format!(
        "postgresql://{}:{}@{}:{}/{}",
        db_usr, db_pass, db_host, db_port, db_name
    );

    // First, establish a stable connection with the database before creating the connection pool
    let ping = || async {
        if let Err(err) = PgConnection::establish(&database_url) {
            log::warn!(
                "Couldn't connect to the Postgres database; retrying after a backoff: {:?}",
                err
            );
            Err(backoff::Error::Transient(err))
        } else {
            Ok(())
        }
    };
    ping.retry(ExponentialBackoff::default()).await?;
    log::info!(
        "Connected to Postgres database at postgresql://***:***@{}:{}/{}",
        db_host,
        db_port,
        db_name
    );

    let addr = "0.0.0.0:50555".parse()?;
    let manager: ConnectionManager<PgConnection> = ConnectionManager::new(database_url);
    let pool = r2d2::Pool::builder()
        .max_size(16)
        .build(manager)
        .expect("Could not build connection pool");
    let gate = Gate {
        connection_pool: pool,
    };

    Server::builder()
        .add_service(FeatureGateServer::new(gate))
        .serve(addr)
        .await?;

    warn!("Server exited mainloop");
    Ok(())
}
