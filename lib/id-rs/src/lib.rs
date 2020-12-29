#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use serde::{Deserialize, Serialize};
use std::fmt;
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

/// Difference between Unix epoch and Discord epoch
/// (milliseconds since the first second of 2015)
const DISCORD_EPOCH_OFFSET: u64 = 1_420_070_400_000;

/// Handles atomic provisioning of `HoarFrost` Ids
///
/// See <https://discord.com/developers/docs/reference#snowflakes>
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
    /// Creates a new `IdProvisioner` and examines the system configuration
    /// for the PID and some MAC address
    #[must_use]
    pub fn new() -> Self {
        let mac_addr_significant = get_mac_address().map_or(0, |bytes| u64::from(bytes[0]));
        let worker_id = mac_addr_significant & 0b1_1111;
        let process_id = u64::from(process::id()) & 0b1_1111;
        Self {
            combined_process_id: (worker_id << 17) | (process_id << 12),
            internal_counter: AtomicU64::new(0),
        }
    }

    /// Atomically provisions a new Id
    /// (mutates inner state but doesn't require &mut self because the mutation is atomic
    /// and requiring mutability would make the API un-ergonomic)
    #[must_use]
    pub fn provision(&self) -> HoarFrost {
        self.with_ts(time::millisecond_ts())
    }

    /// Atomically provisions a new Id using the given timestamp
    /// (mutates inner state but doesn't require &mut self because the mutation is atomic
    /// and requiring mutability would make the API un-ergonomic)
    #[must_use]
    pub fn with_ts(&self, timestamp: u64) -> HoarFrost {
        // Note: we can use Ordering::Relaxed here because the overall ordering
        // doesn't really matter; all that matters is that the operation is atomic
        // (since the timestamp is at a more significant bit than the counter)
        let increment = self.internal_counter.fetch_add(1, Ordering::Relaxed) & 0b1111_1111_1111;
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
        .map(mac_address::MacAddress::bytes)
}

/// Naively converts a timestamp into an ID boundary.
/// This should not be used as an actual ID,
/// rather; it can be used as a range boundary for filtering/querying
#[must_use]
pub const fn id_bound_from_ts(timestamp: u64) -> u64 {
    (timestamp - DISCORD_EPOCH_OFFSET) << 22
}

/// Extracts the creation timestamp of the given snowflake-format Id
///
/// See <https://discord.com/developers/docs/reference#snowflakes>
#[must_use]
pub const fn extract_timestamp(snowflake: u64) -> u64 {
    (snowflake >> 22) + DISCORD_EPOCH_OFFSET
}

// Architus-style ID
#[derive(Copy, Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct HoarFrost(pub u64);

impl HoarFrost {
    /// Extracts the creation timestamp of the given hoar frost Id
    ///
    /// See <https://discord.com/developers/docs/reference#snowflakes>
    #[must_use]
    pub const fn extract_timestamp(self) -> u64 {
        extract_timestamp(self.0)
    }
}

impl Into<u64> for HoarFrost {
    fn into(self) -> u64 {
        self.0
    }
}

impl fmt::Display for HoarFrost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Contains ID-related time operations
pub mod time {
    /// Gets the current millisecond unix timestamp
    #[must_use]
    pub fn millisecond_ts() -> u64 {
        imp::millisecond_ts()
    }

    #[cfg(target_os = "linux")]
    mod imp {
        use libc::{clock_gettime, timespec, CLOCK_REALTIME};
        use std::convert::TryFrom;

        /// Invokes `clock_gettime` from time.h in libc to get a `timespec` struct
        fn get_time() -> timespec {
            let mut tp_out = timespec {
                tv_nsec: 0_i64,
                tv_sec: 0_i64,
            };

            // unsafe needed for FFI call to libc
            // (it's (almost?) impossible for this call to break safety)
            let result = unsafe { clock_gettime(CLOCK_REALTIME, &mut tp_out) };
            if result != 0 {
                // this should be impossible and indicates something is very wrong
                // see https://linux.die.net/man/3/clock_gettime
                panic!("clock_gettime returned non-zero result code")
            }

            tp_out
        }

        pub fn millisecond_ts() -> u64 {
            let tp = get_time();
            u64::try_from(tp.tv_nsec / 1_000_000).unwrap_or(0)
                + u64::try_from(tp.tv_sec).unwrap_or(0) * 1_000
        }
    }
}
