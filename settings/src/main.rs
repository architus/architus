use bb8_postgres::{bb8, PostgresConnectionManager, tokio_postgres::NoTls};
use tonic::{
    transport::Server,
    Request, Response, Status,
};
use json;

mod rpc;

use rpc::settings::{GetResponse, GetResponseInner, SettingsUpdate, SettingsRequest, UpdateResponse, UpdateResponseInner,
    settings_server::Settings,
};

mod error_codes {
    pub const SUCCESS: u64 = 0;
    pub const DATABASE_CONNECTION_ERROR: u64 = 1;
    pub const GUILD_ID_NOT_IN_DB: u64 = 2;
    pub const SETTING_NOT_FOUND: u64 = 3;
    pub const JSON_PARSE_ERROR: u64 = 4;
    pub const JSON_NOT_OBJECT: u64 = 5;
    pub const VALUE_NOT_JSON: u64 = 6;
    pub const DB_UPDATE_FAILED: u64 = 7;
    pub const JSON_INSERT_FAILED: u64 = 8;
}

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
        let request = request.into_inner();
        let guild_id = request.guild_id;
        let conn = match self.pool.get().await {
            Ok(c) => c,
            Err(_) => return Err(Status::unavailable("Could not connect to database")),
        };

        let mut results: Vec<UpdateResponseInner> = Vec::with_capacity(request.updates.len());

        let curr_settings = conn.query_one("select * from tb_settings where server_id = $1", &[&(guild_id as i64)]).await;

        let curr_settings: String = match curr_settings {
            Ok(r) => r.get("json_blob"),
            Err(_) => {
                results.push(UpdateResponseInner {
                    value: "".into(),
                    return_code: error_codes::GUILD_ID_NOT_IN_DB,
                    dev_error_msg: "Could not parse json from database".into(),
                    user_error_msg: "Johny doesn't like your server".into(),
                });
                return Ok(Response::new(UpdateResponse {
                    updates: results,
                }));
            },
        };

        let parsed_settings = json::parse(&curr_settings);

        let mut parsed_settings = match parsed_settings {
            Ok(j) => j,
            Err(_) => {
                results.push(UpdateResponseInner {
                    value: "".into(),
                    return_code: error_codes::JSON_PARSE_ERROR,
                    dev_error_msg: "Could not parse json from database".into(),
                    user_error_msg: "Johny screwed it up".into(),
                });
                return Ok(Response::new(UpdateResponse {
                    updates: results,
                }));
            },
        };

        if !parsed_settings.is_object() {
            results.push(UpdateResponseInner {
                value: "".into(),
                return_code: error_codes::JSON_NOT_OBJECT,
                dev_error_msg: "Json didn't parse the base object as an object".into(),
                user_error_msg: "Johny screwed it up".into(),
            });
            return Ok(Response::new(UpdateResponse {
                updates: results,
            }));
        }

        for update in request.updates {
            match json::parse(&update.value) {
                Ok(v) => {
                    match parsed_settings.insert(&update.name, v) {
                        Ok(_) => {
                            results.push(UpdateResponseInner {
                                value: update.value,
                                return_code: error_codes::SUCCESS,
                                dev_error_msg: "".into(),
                                user_error_msg: "".into(),
                            });
                        },
                        Err(_) => {
                            results.push(UpdateResponseInner {
                                value: update.value,
                                return_code: error_codes::JSON_INSERT_FAILED,
                                dev_error_msg: "Tried to add invalid json to the db".into(),
                                user_error_msg: "Johny is speaking gibberish".into(),
                            });
                        }
                    }
                },
                Err(_) => {
                    results.push(UpdateResponseInner {
                        value: update.value,
                        return_code: error_codes::VALUE_NOT_JSON,
                        dev_error_msg: "Tried to add invalid json to the db".into(),
                        user_error_msg: "Johny is speaking gibberish".into(),
                    });
                },
            };
        }

        let json_string = json::stringify(parsed_settings);
        match conn.execute("insert into tb_settings json_blob values $1", &[&json_string]).await {
            Ok(_) => {},
            Err(_) => {
                results.clear();
                results.push(UpdateResponseInner {
                    value: "".into(),
                    return_code: error_codes::DB_UPDATE_FAILED,
                    dev_error_msg: "Could not insert update into database".into(),
                    user_error_msg: "Johny broke the database".into(),
                });
            },
        };

        Ok(Response::new(UpdateResponse {
            updates: results,
        }))
    }

    async fn get_setting(&self, request: Request<SettingsRequest>) -> Result<Response<GetResponse>, Status> {
        let request = request.into_inner();
        let guild_id = request.guild_id;
        let mut results: Vec<GetResponseInner> = Vec::with_capacity(request.settings.len());
        let conn = match self.pool.get().await {
            Ok(c) => c,
            Err(_) => {
                results.push(GetResponseInner {
                    value: "".into(),
                    return_code: error_codes::DATABASE_CONNECTION_ERROR,
                    dev_error_msg: "Failed to get connection to the database".into(),
                    user_error_msg: "Johny borked the database".into(),
                });
                return Ok(Response::new(GetResponse {
                    settings: results,
                }));
            },
        };

        let curr_settings = conn.query_one("select * from tb_settings where server_id = $1", &[&(guild_id as i64)]).await;

        let curr_settings: String = match curr_settings {
            Ok(r) => r.get("json_blob"),
            Err(_) => {
                results.push(GetResponseInner {
                    value: "".into(),
                    return_code: error_codes::GUILD_ID_NOT_IN_DB,
                    dev_error_msg: "Could not parse json from database".into(),
                    user_error_msg: "Johny doesn't like your server".into(),
                });
                return Ok(Response::new(GetResponse {
                    settings: results,
                }));
            },
        };

        let parsed_settings = json::parse(&curr_settings);

        let parsed_settings = match parsed_settings {
            Ok(j) => j,
            Err(_) => {
                results.push(GetResponseInner {
                    value: "".into(),
                    return_code: error_codes::JSON_PARSE_ERROR,
                    dev_error_msg: "Could not parse json from database".into(),
                    user_error_msg: "Johny screwed it up".into(),
                });
                return Ok(Response::new(GetResponse {
                    settings: results,
                }));
            },
        };

        if !parsed_settings.is_object() {
            results.push(GetResponseInner {
                value: "".into(),
                return_code: error_codes::JSON_NOT_OBJECT,
                dev_error_msg: "Json didn't parse the base object as an object".into(),
                user_error_msg: "Johny screwed it up".into(),
            });
            return Ok(Response::new(GetResponse {
                settings: results,
            }));
        }

        for val in request.settings {
            if parsed_settings.has_key(&val.name) {
                results.push(GetResponseInner {
                    value: json::stringify(parsed_settings[&val.name].clone()),
                    return_code: error_codes::SUCCESS,
                    dev_error_msg: "".into(),
                    user_error_msg: "Johny did something right".into(),
                });
            } else {
                results.push(GetResponseInner {
                    value: "".into(),
                    return_code: error_codes::SETTING_NOT_FOUND,
                    dev_error_msg: "You requested a setting that does not exist".into(),
                    user_error_msg: "Johny couldn't figure out how to use his bot properly".into(),
                });
            }
        }

        Ok(Response::new(GetResponse {
            settings: results,
        }))
    }
}
