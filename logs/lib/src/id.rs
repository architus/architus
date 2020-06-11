use crate::time;
use mac_address;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

/// Difference between Unix epoch and Discord epoch
/// (milliseconds since the first second of 2015)
const DISCORD_EPOCH_OFFSET: u64 = 1_420_070_400_000;

/// Handles atomic provisioning of HoarFrost Ids
///
/// See https://discord.com/developers/docs/reference#snowflakes
pub struct IdProvisioner {
    combined_process_id: u64,
    internal_counter: AtomicU64,
}

#[must_use]
fn get_mac_address() -> Option<[u8; 6]> {
    mac_address::get_mac_address()
        .ok()
        .flatten()
        .map(|mac| mac.bytes())
}

impl Default for IdProvisioner {
    #[must_use]
    fn default() -> Self {
        let mac_addr_significant = get_mac_address().map(|bytes| bytes[0] as u64).unwrap_or(0);
        let worker_id = mac_addr_significant & 0b11111;
        let process_id = process::id() as u64 & 0b11111;
        return Self {
            combined_process_id: (worker_id << 17) | (process_id << 12),
            internal_counter: AtomicU64::new(0),
        };
    }
}

impl IdProvisioner {
    #[must_use]
    pub fn new() -> Self {
        Default::default()
    }

    /// Atomically provisions a new Id
    #[must_use]
    pub fn provision(&self) -> u64 {
        let increment = self.internal_counter.fetch_add(1, Ordering::Relaxed) & 0b111111111111;
        let timestamp = (time::millisecond_ts() - DISCORD_EPOCH_OFFSET) << 22;
        increment | timestamp | self.combined_process_id
    }
}

/// Extracts the creation timestamp of the given snowflake-format Id
///
/// See https://discord.com/developers/docs/reference#snowflakes
#[must_use]
pub fn extract_timestamp(snowflake: u64) -> u64 {
    (snowflake >> 22) + DISCORD_EPOCH_OFFSET
}
