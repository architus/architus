use crate::gateway::{EventProcessor, Path, Processor, TimestampSource};
use logs_lib::ActionType;
use twilight_model::gateway::event::EventType;

/// Registers all pre-configured processors
/// to handle as many gateway events as possible
pub fn register_all(processor: Processor) -> Processor {
    processor.register(
        EventType::MessageCreate,
        EventProcessor::Static {
            action_type: ActionType::MessageSend,
            timestamp_src: TimestampSource::Snowflake(Path::gateway("id")),
            subject_id_src: Some(Path::gateway("id")),
            agent_id_src: Some(Path::gateway("author.id")),
            audit_log_src: None,
            guild_id_src: Some(Path::gateway("guild_id")),
            reason_src: None,
        },
    )
}
