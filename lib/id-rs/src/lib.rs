#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

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
