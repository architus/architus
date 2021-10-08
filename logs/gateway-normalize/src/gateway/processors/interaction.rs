//! Defines processors for the following events:
//! - `InteractionCreate` (from `GatewayEventType::InteractionCreate`)

use crate::event::NormalizedEvent;
use crate::gateway::{Processor, ProcessorContext, ProcessorError, ProcessorFleet};
use twilight_model::gateway::event::EventType as GatewayEventType;

pub fn register_all(fleet: &mut ProcessorFleet) {
    fleet.register(
        GatewayEventType::InteractionCreate,
        Processor::sync(interaction_create),
    );
}

/// Handles `GatewayEventType::InteractionCreate`
fn interaction_create(_ctx: ProcessorContext<'_>) -> Result<NormalizedEvent, ProcessorError> {
    // TODO implement
    Err(ProcessorError::Drop)
}
