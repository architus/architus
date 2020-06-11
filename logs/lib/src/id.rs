use crate::time;
use mac_address;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

/// Handles atomic provisioning of HoarFrost Ids
///
/// See https://discord.com/developers/docs/reference#snowflakes
pub struct IdProvisioner {
    shifted_worker_id: u64,
    shifted_process_id: u64,
    internal_counter: AtomicU64,
}

fn get_mac_address() -> Option<[u8; 6]> {
    mac_address::get_mac_address()
        .ok()
        .flatten()
        .map(|mac| mac.bytes())
}

const DISCORD_EPOCH_OFFSET: u64 = 1_420_070_400_000;

impl Default for IdProvisioner {
    fn default() -> Self {
        let mac_addr_significant = get_mac_address().map(|bytes| bytes[0] as u64).unwrap_or(0);
        return Self {
            shifted_worker_id: (mac_addr_significant & 0b11111) << 17,
            shifted_process_id: (process::id() as u64 & 0b11111) << 12,
            internal_counter: AtomicU64::new(0),
        };
    }
}

impl IdProvisioner {
    pub fn new() -> Self {
        Default::default()
    }

    /// Atomically provisions a new Id
    #[must_use]
    pub fn provision(&self) -> u64 {
        let increment = self.internal_counter.fetch_add(1, Ordering::Relaxed) & 0b111111111111;
        let timestamp = (time::millisecond_ts() - DISCORD_EPOCH_OFFSET) << 22;
        increment | timestamp | self.shifted_worker_id | self.shifted_process_id
    }
}

/// Extracts the creation timestamp of the given snowflake-format Id
///
/// See https://discord.com/developers/docs/reference#snowflakes
#[must_use]
pub fn extract_timestamp(snowflake: u64) -> u64 {
    (snowflake >> 22) + DISCORD_EPOCH_OFFSET
}
