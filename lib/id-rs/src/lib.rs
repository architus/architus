#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::convert::TryInto;
use std::time::{SystemTime, UNIX_EPOCH};

/// Operations related to processing Discord-generated snowflake IDs.
///
/// See <https://discord.com/developers/docs/reference#snowflakes>
pub mod snowflake {
    /// Difference between Unix epoch and Discord epoch
    /// (milliseconds since the first second of 2015)
    pub const DISCORD_EPOCH_OFFSET: u64 = 1_420_070_400_000;

    /// Start position of the timestamp portion of the snowflake binary encoding.
    const TIMESTAMP_BIT_OFFSET: isize = 22;

    /// Naively converts a timestamp into an snowflake boundary.
    /// This should not be used as an actual snowflake,
    /// rather; it can be used as a range boundary for filtering/querying
    #[must_use]
    pub const fn bound_from_ts(timestamp: u64) -> u64 {
        (timestamp - DISCORD_EPOCH_OFFSET) << TIMESTAMP_BIT_OFFSET
    }

    /// Extracts the creation timestamp of the given snowflake
    #[must_use]
    pub const fn extract_timestamp(snowflake: u64) -> u64 {
        (snowflake >> TIMESTAMP_BIT_OFFSET) + DISCORD_EPOCH_OFFSET
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Type {
    LogEvent,
}

impl From<Type> for &'static str {
    fn from(id_type: Type) -> Self {
        match id_type {
            Type::LogEvent => "lgev",
        }
    }
}

/// Generates a new ID from the given ID type
#[must_use]
pub fn new(id_type: Type) -> String {
    format_id(id_type, ksuid::Ksuid::generate())
}

#[derive(thiserror::Error, Debug)]
pub enum WithTsError {
    #[error("unix timestamp is too low; underflowed when converting")]
    TimestampTooLow {
        original: u64,
        sec_timestamp: u64,
        offset: u64,
    },
    #[error("unix timestamp is too high; exceeded u32 max after converting")]
    TimestampTooHigh {
        original: u64,
        converted_timestamp: u64,
    },
}

#[allow(clippy::cast_sign_loss)]
const KSUID_TIMESTAMP_OFFSET: u64 = ksuid::EPOCH.sec as u64;

/// Tries to generates a new ID from the given ID type and Unix MS timestamp.
/// Returns `None` if the Unix millisecond timestamp can't be converted into a valid KSUID timestamp,
/// which is an unsigned 32 bit integer
/// representing the seconds since the KSUID epoch (March 5th, 2014).
/// # Errors
/// - Returns `WithTsError::TimestampTooLow` if the given timestamp
/// underflowed when converting to a second timestamp based on the KSUID epoch
/// - Returns `WithTsError::TimestampTooHigh` if the given timestamp
/// couldn't safely fit into a `u32`
/// even after being converted to a second timestamp based on the KSUID epoch
pub fn with_ts(id_type: Type, ms_timestamp: u64) -> Result<String, WithTsError> {
    let ksuid_timestamp: u32 = ms_timestamp_to_ksuid_timestamp(ms_timestamp)?;
    let mut id = ksuid::Ksuid::generate();
    id.set_timestamp(ksuid_timestamp);
    Ok(format_id(id_type, id))
}

fn ms_timestamp_to_ksuid_timestamp(ms_timestamp: u64) -> Result<u32, WithTsError> {
    // Calculate the second timestamp from the KSUID epoch
    let sec_timestamp: u64 = ms_timestamp / 1_000;
    let (ksuid_timestamp_u64, overflowed) = sec_timestamp.overflowing_sub(KSUID_TIMESTAMP_OFFSET);
    if overflowed {
        return Err(WithTsError::TimestampTooLow {
            original: ms_timestamp,
            sec_timestamp,
            offset: KSUID_TIMESTAMP_OFFSET,
        });
    }
    let ksuid_timestamp: u32 =
        ksuid_timestamp_u64
            .try_into()
            .map_err(|_| WithTsError::TimestampTooHigh {
                original: ms_timestamp,
                converted_timestamp: ksuid_timestamp_u64,
            })?;

    Ok(ksuid_timestamp)
}

fn format_id(id_type: Type, id: ksuid::Ksuid) -> String {
    let id_type_str: &str = id_type.into();
    format!("{}_{}", id_type_str, id.to_base62())
}

#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("ID did not have underscore-separated prefix & ID components")]
    NoPrefixOrID,
    #[error("ID had too many underscores")]
    TooManyUnderscores,
    #[error("ID had unexpected prefix")]
    WrongPrefix { actual: String, expected: String },
    #[error("The ID couldn't be parsed")]
    MalformedID(#[source] std::io::Error),
}

/// Validates the given ID, returning Ok(()) if it is valid
/// # Errors
/// The error result indicates that the ID wasn't valid,
/// and the error enum inside gives more information
/// on what specifically made it invalid.
pub fn validate(id_type: Type, id: impl AsRef<str>) -> Result<(), ValidationError> {
    // Parse the components
    let components = id.as_ref().split('_').collect::<Vec<_>>();
    if components.len() < 2 {
        return Err(ValidationError::NoPrefixOrID);
    }
    if components.len() > 2 {
        return Err(ValidationError::TooManyUnderscores);
    }

    // Validate the prefix
    let expected_id_type_str: &str = id_type.into();
    if components[0] != expected_id_type_str {
        return Err(ValidationError::WrongPrefix {
            actual: String::from(components[0]),
            expected: String::from(expected_id_type_str),
        });
    }

    match ksuid::Ksuid::from_base62(components[1]) {
        Ok(_) => Ok(()),
        Err(err) => Err(ValidationError::MalformedID(err)),
    }
}


/// Naively converts a timestamp into an ID upper boundary.
/// This should not be used as an actual ID,
/// rather; it can be used as a range boundary for filtering/querying
#[must_use]
pub fn upper_bound_from_ts(id_type: Type, ms_timestamp: u64) -> String {
    let ksuid_timestamp: u32 = match ms_timestamp_to_ksuid_timestamp(ms_timestamp) {
        Ok(ts) => ts,
        Err(WithTsError::TimestampTooLow {.. }) => u32::MIN,
        Err(WithTsError::TimestampTooHigh {.. }) => u32::MAX,
    };
    format_id(id_type, ksuid::Ksuid::new(ksuid_timestamp, [0xFF; 16]))
}
