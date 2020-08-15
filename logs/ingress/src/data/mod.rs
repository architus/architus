mod processor;

use std::collections::HashMap;
use logs_lib::{ActionType, AuditLogEntryType};
use crate::data::processor::{GatewayProcessor, Path, TimestampSource};

fn make_definitions() -> HashMap<AuditLogEntryType, GatewayProcessor> {
    let mut map: HashMap<AuditLogEntryType, GatewayProcessor> = HashMap::new();

    map.insert(AuditLogEntryType::ChannelCreate, GatewayProcessor{
        action_type: ActionType::ChannelCreate,
        timestamp_src: TimestampSource::Snowflake(Path::gateway("id")),
        subject_id_src: Some(Path::gateway("id")),
        agent_id_src: Some(Path::audit_log("user_id")),
        audit_log_src: None,
    });

    map
}

