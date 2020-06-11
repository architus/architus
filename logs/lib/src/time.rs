/// Gets the second unix timestamp for the stat filename
#[must_use]
pub fn millisecond_ts() -> u64 {
    time::millisecond_ts()
}

#[cfg(target_os = "linux")]
mod time {
    use libc::{clock_gettime, timespec, CLOCK_REALTIME};
    use std::mem;

    /// Invokes `clock_gettime` from time.h in libc to get a `timespec` struct
    fn get_time() -> timespec {
        let mut tp: timespec = unsafe { mem::zeroed() };
        unsafe {
            clock_gettime(CLOCK_REALTIME, &mut tp);
        }
        tp
    }

    pub fn millisecond_ts() -> u64 {
        let tp = get_time();
        (tp.tv_nsec / 1_000_000) as u64 + (tp.tv_sec as u64) * 1_000
    }
}
