use std::time::{UNIX_EPOCH, Duration, SystemTime};
use std::collections::VecDeque;
use std::cmp;
use std::cmp::Ordering;

use twilight_model::id::{GuildId, ChannelId};
use rocket::serde::{Deserialize, Serialize};

const SECONDS_IN_DAY: u64 = 86_400;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Timespan(pub u64, pub u64);

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Work(pub GuildId, pub ChannelId, pub Timespan);

pub const NULL_WORK: Work = Work(GuildId(0), ChannelId(0), Timespan(0, 0));

impl Timespan {
    /// Attempts to merge two timespans together. Will only merge them
    /// if there is overlap. Returns `None` if there is no overlap.
    pub fn merge(&self, other: &Self) -> Option<Self> {
        if self.1 > other.0 || self.0 < other.1 {
            Some(Timespan(self.0.min(other.0), self.1.max(other.1)))
        } else {
            None
        }
    }
}

impl Work {
    /// Attempts to merge two units of work together. Will only do so
    /// if the timespans overlap else, it returns `None`.
    pub fn merge(&self, other: &Self) -> Option<Self> {
        if self.0 != other.0 || self.1 != other.1 {
            return None;
        }

        match self.2.merge(&other.2) {
            Some(t) => Some(Work(self.0, self.1, t)),
            None => None,
        }
    }
}

// Implement ord and eq functions for use by the vecdequeue to binary
// search for the same guild
impl cmp::PartialOrd for Work {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        let guild = self.0.0.cmp(&other.0.0);
        match guild {
            Ordering::Equal => Some(self.1.0.cmp(&other.1.0)),
            _ => Some(guild),
        }
    }
}

impl cmp::PartialEq for Work {
    fn eq(&self, other: &Self) -> bool {
        self.0.0 == other.0.0 && self.1.0 == other.1.0
    }
}

impl cmp::Ord for Work {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.0.0.cmp(&other.0.0)
    }
}

impl cmp::Eq for Work {}

/// Represents work that must be done to scrape the audit logs.
pub struct WorkQueue {
    // The last time at which the work queue was updated. This is how
    // the manager will know when 24 hours has passed and to update
    // the queue again.
    last_update: SystemTime,

    // This is the actual work that must be done. It is a queue of
    // guilds and the timespand for which to get events.
    // Uses a VecDeque as that will have better performance for
    // pushing and popping from both ends.
    queue: VecDeque<Work>,
}

impl WorkQueue {
    pub fn new(num_guilds: usize) -> Self {
        Self {
           last_update: UNIX_EPOCH,
           queue: VecDeque::with_capacity(num_guilds),
        }
    }

    /// Tests whether or not the work queue needs to be updated.
    pub fn requires_update(&self) -> bool {
        let day = Duration::new(SECONDS_IN_DAY, 0);
        SystemTime::now() - day < self.last_update
    }

    /// Updates the last timestamp at which the work queue was updated.
    pub fn update_timestamp(&mut self) {
        self.last_update = SystemTime::now();
    }

    /// Add a unit of work to the front of the queue.
    pub fn add_work(&mut self, w: Work) {
        self.queue.push_back(w);
    }

    /// Pops a unit of work from the beginning of the queue.
    pub fn get_work(&mut self) -> Option<Work> {
        self.queue.pop_front()
    }

    /// Pushes a unit or work to the end of the queue.
    pub fn queue_empty(&self) -> bool {
        self.queue.len() == 0
    }

    /// Moves a guilds work to the front of the queue. If there is other work
    /// to be done for that guild in the queue already, this method will try
    /// to merge the work together and move the merged work to the front of the
    /// queue. Otherwise, just the new work will be moved to the front.
    pub fn move_to_front(&mut self, w: Work) {
        match self.queue.binary_search(&w) {
            Ok(i) => {
                let o = self.queue.get(i).expect("Got index via binary search");
                match o.merge(&w) {
                    Some(t) => {
                        self.queue[i] = Work(w.0, w.1, t.2);
                        self.queue.swap(0, i);
                    },
                    None => self.queue.push_front(w),
                };
            },
            Err(_) => self.queue.push_front(w),
        };
    }
}
