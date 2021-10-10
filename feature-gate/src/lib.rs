//! Api for interacting with the postgresql database through `diesel`.
//!
//! Abstracts away all of the feature ids and relational database stuff
//! so that the feature server only has to deal with guild ids and
//! feature names.

#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

pub mod config;
pub mod models;
pub mod rpc;
pub mod schema;

#[macro_use]
extern crate diesel;

use diesel::pg::expression::dsl::any;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::collections::HashSet;
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

/// Inserts a new feature into the database.
///
/// # Arguments
/// * `conn` - Database connection
/// * `name` - Name of feature to insert
/// * `open` - Whether or not the feature is open or closed
///
/// # Errors
/// * `DatabaseError::Insertion` - Failed to insert new feature into database
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
///
/// # Arguments
/// * `conn` - Database connection
///
/// # Errors
/// * `DatabaseError::Query` - Failed to query the database
pub fn get_all_features(conn: &PgConnection) -> DbResult<Vec<models::Feature>> {
    let features = schema::tb_feature_flags::table.load::<models::Feature>(conn);
    features.map_err(|_| DatabaseError::Query)
}

/// Inserts a new guild <-> feature relation into the database.
///
/// Will return an `UnknownFeature` error if a feature that has not been previously
/// added is passed as an argument.
///
/// # Arguments
/// * `conn` - Connection to the database
/// * `guild_id` - ID of the guild to add the feature to
/// * `feature_name` - Name of the feature being added to the guild
///
/// # Errors
/// * `DatabaseError::UnknownFeature` - The feature passed in is not in the database
/// * `DatabaseError::Insertion` - Association could not be inserted into the database
pub fn insert_guild_feature(
    conn: &PgConnection,
    guild_id: i64,
    feature_name: &str,
) -> DbResult<()> {
    let feature_id = get_feature_id(conn, feature_name)?;

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
///
/// # Arguments
/// * `conn` - Connection to the database
/// * `guild_id` - Which guild to get the features of
///
/// # Errors
/// * `DatabaseError::Query` - The database query failed
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
///
/// # Arguments
/// * `conn` - Database connection
/// * `guild_id` - ID of guild that is being checked
/// * `feature` - Name of feature to check on guild
///
/// # Errors
/// * `DatabaseError::UnknownFeature` - Feature is not found in the database
/// * `DatabaseError::Query` - The query to the database failed
pub fn check_guild_feature(conn: &PgConnection, guild_id: i64, feature: &str) -> DbResult<bool> {
    let feature_id = get_feature_id(conn, feature)?;

    let result = schema::tb_guild_features::table
        .filter(schema::tb_guild_features::guild_id.eq(guild_id))
        .filter(schema::tb_guild_features::feature_id.eq(feature_id))
        .limit(1)
        .load::<(i64, i32)>(conn);

    match result {
        Ok(v) => {
            if v.is_empty() {
                Ok(false)
            } else {
                Ok(true)
            }
        }
        Err(_) => Err(DatabaseError::Query),
    }
}

/// Checks to see if a list of guilds has the associated feature.
///
/// First gets the feature id and then checks to see if each of the guild id-feature id
/// pairs can be found in the guild feature database.
/// Returns a list of booleans in the same order as the input `guild_ids` list
/// where each element corresponds to whether the corresponding guild has the feature.
///
/// # Arguments
/// * `conn` - Database connection
/// * `guild_ids` - List of Guild IDs to check
/// * `feature` - Name of feature to check on guild
///
/// # Errors
/// * `DatabaseError::UnknownFeature` - Feature is not found in the database
/// * `DatabaseError::Query` - The query to the database failed
pub fn batch_check_guild_feature(
    conn: &PgConnection,
    guild_ids: &[i64],
    feature: &str,
) -> DbResult<Vec<bool>> {
    let feature_id = get_feature_id(conn, feature)?;

    let result = schema::tb_guild_features::table
        .filter(schema::tb_guild_features::feature_id.eq(feature_id))
        // Note: uses PG-specific feature
        .filter(schema::tb_guild_features::guild_id.eq(any(guild_ids)))
        .load::<(i64, i32)>(conn);

    match result {
        Ok(v) => {
            // Create set of guilds with the feature enabled
            let mut with_feature_set = HashSet::<i64>::with_capacity(v.len());
            for (guild_id, _) in v {
                with_feature_set.insert(guild_id);
            }

            // Create an ordered result
            let mut results = Vec::with_capacity(guild_ids.len());
            for guild_id in guild_ids {
                results.push(with_feature_set.contains(guild_id));
            }

            Ok(results)
        }
        Err(_) => Err(DatabaseError::Query),
    }
}

/// Removes a guild <-> feature association from the database.
///
/// # Arguments
/// * `conn` - Database connection
/// * `guild_id` - ID of guild that the feature is being removed from
/// * `feature` - Name of feature being removed from guild
///
/// # Errors
/// * `DatabaseError::UnknownFeature` - Feature name has no associated feature in database
/// * `DatabaseError::Delete` - Failed to delete association from the database
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
///
/// # Arguments
/// * `conn` - Database connection
/// * `feature` - Name of feature to update
/// * `openness` - Value to update feature to
///
/// # Errors
/// * `DatabaseError::UnknownFeature` - Feature name not found in database
/// * `DatabaseError::Update` - Failed to update the feature in the database
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

/// Check to see if a feature is open or closed.
///
/// If the feature does not exist in the database, an unknown feature errror will
/// be returned. Any other error should be interpreted as the database having failed
/// in some way.
///
/// # Arguments
/// * `conn` - Database connection
/// * `feature` - Name of feature to check if it's open or not
///
/// # Errors
/// * `DatabaseError::UnknownFeature` - Feature name not found in database
/// * `DatabaseError::Queyr` - Database qeury failed
pub fn get_feature_openness(conn: &PgConnection, feature: &str) -> DbResult<bool> {
    let feature_id = get_feature_id(conn, feature)?;

    let result = schema::tb_feature_flags::table
        .filter(schema::tb_feature_flags::id.eq(feature_id))
        .limit(1)
        .select(schema::tb_feature_flags::open)
        .load::<bool>(conn);

    match result {
        // Can just blindly remove the first element because we know that it will
        // be there if no error occurrs. The id fetch returned true so the feature
        // is there. Therefore it must have an open value that can be returned.
        Ok(mut v) => Ok(v.remove(0)),
        Err(_) => Err(DatabaseError::Query),
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
            if v.is_empty() {
                Err(DatabaseError::UnknownFeature)
            } else {
                Ok(v.remove(0))
            }
        }
        Err(_) => Err(DatabaseError::Query),
    }
}
