pub mod models;
pub mod schema;

#[macro_use]
extern crate diesel;
extern crate dotenv;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to insert feature data into database")]
    FeatureInsertion,
    #[error("Failed to retrieve data from database")]
    Query,
    #[error("Failed to insert guild data into database")]
    GuildInsertion,
}

type DbResult<T> = Result<T, DatabaseError>;

/// Returns a connection to the postresql database.
pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("Need to put database url in .env file");
    PgConnection::establish(&database_url).expect("Could not connect to database.")
}

/// Inserts a new feature into the database. Will return a result that's either an
/// insertion error or the id that was assigned to the new feature.
#[must_use]
pub fn insert_feature(conn: &PgConnection, name: &str, open: bool) -> DbResult<i32> {
    let new_feature = models::NewFeature { name, open };

    let res: Result<models::Feature, _> = diesel::insert_into(schema::tb_feature_flags::table)
        .values(&new_feature)
        .get_result(conn);

    match res {
        Ok(f) => Ok(f.id),
        Err(_) => Err(DatabaseError::FeatureInsertion),
    }
}

/// Queries the database for all features.
pub fn get_all_features(conn: &PgConnection) -> DbResult<Vec<models::Feature>> {
    let features = schema::tb_feature_flags::table.load::<models::Feature>(conn);
    features.map_err(|_| DatabaseError::Query)
}

/// Queries the database to find a feature with the matching id.
pub fn get_feature_by_id(
    conn: &PgConnection,
    feature_id: i32,
) -> DbResult<Option<models::Feature>> {
    let result = schema::tb_feature_flags::table
        .filter(schema::tb_feature_flags::id.eq(feature_id))
        .limit(1)
        .load::<models::Feature>(conn);

    match result {
        Ok(mut res) => {
            if res.len() > 0 {
                Ok(Some(res.remove(0)))
            } else {
                Ok(None)
            }
        }
        Err(_) => Err(DatabaseError::Query),
    }
}

/// Queries the database to find a feature with the matching name.
pub fn get_feature_by_name(
    conn: &PgConnection,
    feature_name: &str,
) -> DbResult<Option<models::Feature>> {
    let result = schema::tb_feature_flags::table
        .filter(schema::tb_feature_flags::name.eq(feature_name))
        .limit(1)
        .load::<models::Feature>(conn);

    match result {
        Ok(mut res) => {
            if res.len() > 0 {
                Ok(Some(res.remove(0)))
            } else {
                Ok(None)
            }
        }
        Err(_) => Err(DatabaseError::Query),
    }
}

/// Inserts a new guild <-> feature relation into the database.
///
/// This function will not check that the guild_id and feature_id are valid ids.
/// Only pass feature_ids that are known to be in the database. These can be found
/// by using the `get_feature_by_name` function to get the feature's id.
#[must_use]
pub fn insert_guild_feature(conn: &PgConnection, guild_id: i64, feature_id: i64) -> DbResult<()> {
    let relation = models::Guild {
        guild_id,
        feature_id,
    };

    let result = diesel::insert_into(schema::tb_guild_features::table)
        .values(&relation)
        .get_result(conn);

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(DatabaseError::GuildInsertion),
    }
}
