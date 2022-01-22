use crate::normalize;

use twilight_model::guild::audit_log;

pub fn parse_audit_entry(entry: AuditLog) -> NormalizedEvent {
    match entry.action_type {
        
    }
}
