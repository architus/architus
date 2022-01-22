use crate::util::snowflake;

use architus_logs_lib::event::*;

use serde_json::to_value;
use twilight_model::guild::audit_log;
use twilight_model::id::{GuildId, UserId};
use twilight_model::guild::audit_log::AuditLogEventType;

pub fn parse_audit_logs(audit: AuditLog, guild: GuildId, bot_id: u64) -> Vec<NormalizedEvent> {
    let mut normalized = Vec::with_capacity(audit.entries.len());

    for i in 0..audit.entries.len() {
        let entry_type: EntryType;
        let id_param: IdParams;
        let timestamp: u64;
        let mut channel: Option<Channel> = None;
        let mut agent: Option<Agent> = None;
        let mut subject: Option<Entity> = None;
        match audit.entries[i].action_type {
            AuditLogEventType::GuildUpdate => {
                entry_type = EventType::GuildUpdate;
                id_param = IdParams::Two(guild.0, audit.entries[i].id.0);
                timestamp = snowflake::extract_system_time(audit.entries[i].id.0);
                subject = Some(Entity::Guild(guild.0));
                if let Some(uid) = audit.entries[i].user_id {
                    agent = Some(get_user_agent(&audit, &uid, bot_id));
                }
            }
        }
        let source = Source {
            gateway: None,
            audit_log: to_value(audit.entries[i]).ok(),
            internal: None,
        };
        normalized.push(NormalizedEvent {
            id_params: id_param,
            timestamp: timestamp,
            source: source,
            origin: EventOrigin::AuditLog,
            event_type: entry_type,
            guild_id: guild.0,
            reason: audit.entries[i].reason.clone(),
            audit_log_id: Some(audit.entries[i].id.0),
            channel: channel,
            agent: agent,
            subject: subject,
        });
    }
    normalized
}

fn get_user_agent(log: &AuditLog, user: &UserId, bot_id: u64) -> Agent {
    let user_data = log.users.iter().find(|u| u.id == user);
    if let Some(u) = user_data {
        let user_color = if let Some(v) = u.color {
            u32::try_from(v).ok()
        } else {
            None
        };
        Agent {
            entity: Entity::UserLike(UserLike {
                id: u.id.0,
                name: u.name.clone(),
                nickname: None,
                discriminator: u.discriminator,
                color: user_color,
            }),
            special_type: Agent::type_from_id(user.0, Some(bot_id)),
            webhook_username: None,
        }
    } else {
        Agent {
            entity: Entity::UserLike(UserLike {
                id: user.0,
                name: None,
                nickname: None,
                discriminator: None,
                color: None,
            }),
            special_type: Agent::type_from_id(user.0, Some(bot_id)),
            webhook_username: None,
        }
    }
}
