use static_assertions::assert_impl_all;
use anyhow::{Context, Result};
use crate::event::NormalizedEvent;

/// Represents a single gateway event that is ready to be normalized
pub struct OriginalEvent {
    seq: Option<u64>,
    event_type: String,
    json: serde_json::Value,
    rx_timestamp: u64,
}

pub struct Processor {

}

impl Default for Processor {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor {
    pub fn new() -> Self {
        Self{}
    }

    pub async fn normalize(&self, event: OriginalEvent) -> Result<NormalizedEvent> {
        todo!()
    }

    pub fn can_process(&self, event_type: &str) -> bool {
        todo!()
    }
}


// Processor should be safe to share,
// and we'll wrap it in an Arc when using
assert_impl_all!(Processor: Sync);
