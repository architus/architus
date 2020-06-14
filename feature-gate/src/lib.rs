//! Api for interacting with the postgresql database through `diesel`.
//!
//! Abstracts away all of the feature ids and relational database stuff
//! so that the feature server only has to deal with guild ids and
//! feature names.

pub mod models;
pub mod schema;

#[macro_use]
extern crate diesel;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use thiserror::Error;

/// Simple custom error type for letting the library user know what went
/// wrong with a call to any of the functions.
#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Failed to insert data into database")]
    Insertion,
    #[error("Failed to retrieve data from database")]
    Query,
    #[error("Requested feature does not exist in database")]
    UnknownFeature,
    #[error("Failed to update db entry")]
    Update,
    #[error("Failed to remove guild feature from database")]
    Delete,
}

type DbResult<T> = Result<T, DatabaseError>;

/// Returns a connection to the postresql database.
pub fn establish_connection(database_url: &str) -> PgConnection {
    PgConnection::establish(database_url).expect("Could not connect to database.")
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
        Err(_) => Err(DatabaseError::Insertion),
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
pub fn insert_guild_feature(conn: &PgConnection, guild_id: i64, feature_id: i32) -> DbResult<()> {
    let relation = models::Guild {
        guild_id,
        feature_id,
    };

    let result = diesel::insert_into(schema::tb_guild_features::table)
        .values(&relation)
        .execute(conn);

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(DatabaseError::Insertion),
    }
}

/// Gets all of the features associated with a guild id.
pub fn get_guild_features(conn: &PgConnection, guild_id: i64) -> DbResult<Vec<(String, bool)>> {
    let join = schema::tb_guild_features::table
        .inner_join(schema::tb_feature_flags::table)
        .filter(schema::tb_guild_features::guild_id.eq(guild_id))
        .select((
            schema::tb_feature_flags::name,
            schema::tb_feature_flags::open,
        ))
        .load::<(String, bool)>(conn);

    join.map_err(|_| DatabaseError::Query)
}

/// Checks to see if a guild has the associated feature.
///
/// First gets the feature id and then checks to see if the guild id and feature id
/// pair can be found in the guild feature database.
pub fn check_guild_feature(conn: &PgConnection, guild_id: i64, feature: &str) -> DbResult<bool> {
    let feature_id = get_feature_id(conn, feature)?;

    let result = schema::tb_guild_features::table
        .filter(schema::tb_guild_features::guild_id.eq(guild_id))
        .filter(schema::tb_guild_features::feature_id.eq(feature_id))
        .limit(1)
        .load::<(i64, i32)>(conn);

    match result {
        Ok(v) => {
            if v.len() > 0 {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(_) => Err(DatabaseError::Query),
    }
}

/// Removes a guild <-> feature association from the database.
#[must_use]
pub fn remove_guild_feature(conn: &PgConnection, guild_id: i64, feature: &str) -> DbResult<()> {
    let feature_id = get_feature_id(conn, feature)?;

    let result = diesel::delete(schema::tb_guild_features::table)
        .filter(schema::tb_guild_features::guild_id.eq(guild_id))
        .filter(schema::tb_guild_features::feature_id.eq(feature_id))
        .execute(conn);

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(DatabaseError::Delete),
    }
}

/// Sets the open flag of a feature.
///
/// Will check to make sure that the feature has already been added to the database.
/// If called with a feature id that is not in the db it will just return an
/// unknown feature error.
#[must_use]
pub fn set_feature_openness(conn: &PgConnection, feature: &str, openness: bool) -> DbResult<()> {
    let feature_id = get_feature_id(conn, feature)?;

    let result = diesel::update(schema::tb_feature_flags::table)
        .filter(schema::tb_feature_flags::id.eq(feature_id))
        .set(schema::tb_feature_flags::open.eq(openness))
        .execute(conn);

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(DatabaseError::Update),
    }
}

// Searches database for the given feature.
fn get_feature_id(conn: &PgConnection, feature: &str) -> DbResult<i32> {
    let result = schema::tb_feature_flags::table
        .filter(schema::tb_feature_flags::name.eq(feature))
        .limit(1)
        .select(schema::tb_feature_flags::id)
        .load::<i32>(conn);

    match result {
        Ok(mut v) => {
            if v.len() > 0 {
                Ok(v.remove(0))
            } else {
                Err(DatabaseError::UnknownFeature)
            }
        }
        Err(_) => Err(DatabaseError::Query),
    }
}
