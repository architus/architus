//! gRPC server for interfacing with the feature flags database.
//!
//! Due to the diesel connection struct not being sync or send,
//! the server will establish a new db connection for each request.
//! If the server is not able to establish a connection to the
//! database it will return some kind of error message.
//! The specifics of what the error message are can be found
//! in the specific documentation for each function.

use db::*;
use diesel::connection::Connection;
use diesel::PgConnection;
use std::env;
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::mpsc;
use tonic::{transport::Server, Request, Response, Status};

use feature_gate::feature_gate_server::{FeatureGate, FeatureGateServer};
use feature_gate::*;

type RpcResponse<T> = Result<Response<T>, Status>;

pub mod feature_gate {
    include!("../../grpc/featuregate.rs");
}

#[derive(Debug)]
pub struct Gate {
    pub db_addr: String,
}

impl Gate {
    fn get_connection(&self) -> Option<PgConnection> {
        PgConnection::establish(&self.db_addr).ok()
    }
}

#[tonic::async_trait]
impl FeatureGate for Gate {
    /// Adds a new feature to the feature database.
    ///
    /// If the feature is successfully added to the database a success is returned.
    /// If the database is not able to be reached or any other error occurrs while
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
            Ok(_) => success,
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

        let result = insert_guild_feature(&conn, addition.guild_id, &addition.feature_name);
        match result {
            Ok(()) => success,
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
        let result = remove_guild_feature(&conn, removal.guild_id, &removal.feature_name);
        match result {
            Ok(()) => success,
            Err(_) => failure,
        }
    }

    /// Checks to see if a guild has a certain feature.
    ///
    /// Will return a true/false value on successfully querying the database.
    /// If the feature asked about does not exist, then it will return a
    /// default value of true.
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

        let removal = request.into_inner();
        let result = remove_guild_feature(&conn, removal.guild_id, &removal.feature_name);
        match result {
            Ok(()) => Ok(Response::new(FeatureResult { success: true })),
            Err(DatabaseError::UnknownFeature) => {
                Ok(Response::new(FeatureResult { success: true }))
            }
            Err(_) => Err(Status::internal("Database connection failed")),
        }
    }

    // A type association for doing a streaming response.
    type GetFeaturesStream = mpsc::Receiver<Result<Feature, Status>>;

    /// Returns a stream of all of the features on the database.
    async fn get_features(&self, _: Request<FeatureList>) -> RpcResponse<Self::GetFeaturesStream> {
        let conn = match self.get_connection() {
            Some(c) => c,
            None => return Err(Status::internal("Database connection failed")),
        };

        let features = match get_all_features(&conn) {
            Ok(v) => v,
            Err(_) => return Err(Status::internal("Database failure")),
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    sleep(Duration::from_secs(20));
    let database_url =
        env::var("DATABASE_URL").expect("Database url environment variable not set.");

    let addr = "[::1]:50051".parse()?;
    let gate = Gate {
        db_addr: database_url,
    };

    Server::builder()
        .add_service(FeatureGateServer::new(gate))
        .serve(addr)
        .await?;

    println!("Server ended main loop");
    Ok(())
}
