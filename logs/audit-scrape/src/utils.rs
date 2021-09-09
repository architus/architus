/// Utility module for dealing with discord snowflakes
pub mod snowflake {
    pub const DISCORD_EPOCH_OFFSET: u64 = 1_420_070_400_000;

    pub const TIMESTAMP_BIT_OFFSET: usize = 22;

    pub const fn extract_timestamp(snowflake: u64) -> u64 {
        (snowflake >> TIMESTAMP_BIT_OFFSET) + DISCORD_EPOCH_OFFSET
    }
}
