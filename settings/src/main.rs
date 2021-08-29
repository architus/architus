use bb8_postgres::{bb8, PostgresConnectionManager, tokio_postgres::NoTls};
use tonic::{
    transport::{Endpoint, Server, Uri},
    Request, Response, Status,
};

use std::net::SocketAddr;

mod rpc;

use rpc::settings::{GetResponse, GetResponseInner, SettingsUpdate, SettingsRequest, UpdateResponse,
    settings_server::Settings,
};

struct SettingsServer {
    pool: bb8::Pool<PostgresConnectionManager<NoTls>>,
}

#[tokio::main]
async fn main() {
    let manager = PostgresConnectionManager::new_from_stringlike(
        "host=postgres:54321 user=autbot pass=autbot",
        NoTls
        ).expect("Failed to create connection manager");

    let pool = bb8::Pool::builder().build(manager).await.unwrap();

    let server = SettingsServer { pool };
    Server::builder()
        .add_service(rpc::settings::settings_server::SettingsServer::new(server))
        .serve("[::1]:8080".parse().expect("Failed to parse address"))
        .await.expect("Failed to start settings server");
}

#[tonic::async_trait]
impl Settings for SettingsServer {
    async fn set_setting(&self, request: Request<SettingsUpdate>) -> Result<Response<UpdateResponse>, Status> {
        unimplemented!();
    }

    async fn get_setting(&self, request: Request<SettingsRequest>) -> Result<Response<GetResponse>, Status> {
        let request = request.into_inner();
        let conn = match self.pool.get().await {
            Ok(c) => c,
            Err(_) => return Err(Status::unavailable("Could not connect to database")),
        };

        let results: Vec<GetResponseInner> = Vec::with_capacity(request.settings.len());

        Ok(Response::new(GetResponse {
            settings: results,
        }))
    }
}
