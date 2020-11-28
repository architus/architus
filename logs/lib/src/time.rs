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
