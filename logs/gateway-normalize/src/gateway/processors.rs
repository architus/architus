use crate::gateway::ProcessorFleet;

/// Registers all pre-configured processors
/// to handle as many gateway events as possible
pub fn register_all(fleet: ProcessorFleet) -> ProcessorFleet {
    fleet
    // TODO implement
    // processor
    //     .register(
    //         GatewayEventType::MessageCreate,
    //         EventProcessor::Static {
    //             event_type: EventType::MessageSend,
    //             timestamp_src: TimestampSource::Snowflake(Path::gateway("id")),
    //             subject_id_src: Some(Path::gateway("id")),
    //             agent_id_src: Some(Path::gateway("author.id")),
    //             audit_log_src: None,
    //             guild_id_src: Some(Path::gateway("guild_id")),
    //             channel_id_src: Some(Path::gateway("channel_id")),
    //             reason_src: None,
    //         },
    //     )
    //     .register(
    //         GatewayEventType::ChannelCreate,
    //         EventProcessor::Static {
    //             event_type: EventType::ChannelCreate,
    //             timestamp_src: TimestampSource::Snowflake(Path::gateway("id")),
    //             subject_id_src: Some(Path::gateway("id")),
    //             agent_id_src: Some(Path::gateway("author.id")),
    //             audit_log_src: None,
    //             guild_id_src: Some(Path::gateway("guild_id")),
    //             channel_id_src: Some(Path::gateway("channel_id")),
    //             reason_src: None,
    //         },
    //     )
}
