//! Defines processors for the following events:
//! - `MessageSend`
//! - `MessageReply`
//! - `MessageEdit`
//! - `MessageDelete` (hybrid)
//! - `MessageBulkDelete` (hybrid)

use crate::gateway::ProcessorFleet;

pub fn register_all(_fleet: &mut ProcessorFleet) {
    // TODO implement MessageSend processor
    // TODO implement MessageReply processor
    // TODO implement MessageEdit processor
    // TODO implement MessageDelete processor
    // TODO implement MessageBulkDelete processor
}
