use crate::time;
use serde::{Deserialize, Serialize};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

/// Difference between Unix epoch and Discord epoch
/// (milliseconds since the first second of 2015)
const DISCORD_EPOCH_OFFSET: u64 = 1_420_070_400_000;

/// Handles atomic provisioning of HoarFrost Ids
///
/// See https://discord.com/developers/docs/reference#snowflakes
#[derive(Debug)]
pub struct IdProvisioner {
    combined_process_id: u64,
    internal_counter: AtomicU64,
}

impl Default for IdProvisioner {
    #[must_use]
    fn default() -> Self {
        Self::new()
    }
}

impl IdProvisioner {
    /// Creates a new IdProvisioner and examines the system configuration
    /// for the PID and some MAC address
    #[must_use]
    pub fn new() -> Self {
        let mac_addr_significant = get_mac_address().map(|bytes| bytes[0] as u64).unwrap_or(0);
        let worker_id = mac_addr_significant & 0b11111;
        let process_id = process::id() as u64 & 0b11111;
        Self {
            combined_process_id: (worker_id << 17) | (process_id << 12),
            internal_counter: AtomicU64::new(0),
        }
    }

    /// Atomically provisions a new Id
    #[must_use]
    pub fn provision(&self) -> HoarFrost {
        self.with_ts(time::millisecond_ts())
    }

    /// Atomically provisions a new Id using the given timestamp
    #[must_use]
    pub fn with_ts(&self, timestamp: u64) -> HoarFrost {
        // Note: we can use Ordering::Relaxed here because the overall ordering
        // doesn't really matter; all that matters is that the operation is atomic
        // (since the timestamp is at a more significant bit than the counter)
        let increment = self.internal_counter.fetch_add(1, Ordering::Relaxed) & 0b111111111111;
        let shifted_timestamp = (timestamp - DISCORD_EPOCH_OFFSET) << 22;
        HoarFrost(increment | shifted_timestamp | self.combined_process_id)
    }
}

/// Attempts to get the first non-local MAC address of the current system
#[must_use]
fn get_mac_address() -> Option<[u8; 6]> {
    mac_address::get_mac_address()
        .ok()
        .flatten()
        .map(|mac| mac.bytes())
}

/// Extracts the creation timestamp of the given snowflake-format Id
///
/// See https://discord.com/developers/docs/reference#snowflakes
#[must_use]
pub fn extract_timestamp(snowflake: u64) -> u64 {
    (snowflake >> 22) + DISCORD_EPOCH_OFFSET
}

// Architus-style ID
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct HoarFrost(pub u64);

impl HoarFrost {
    /// Extracts the creation timestamp of the given hoar frost Id
    ///
    /// See https://discord.com/developers/docs/reference#snowflakes
    #[must_use]
    pub fn extract_timestamp(&self) -> u64 {
        extract_timestamp(self.0)
    }
}
