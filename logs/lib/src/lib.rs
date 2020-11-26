pub mod id;
pub mod time;

use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Clone, Copy, Debug, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ActionOrigin {
    // Action originated from the gateway and was caught as it originated
    Gateway = 1,
    // Action originated from the audit log
    AuditLog = 2,
    // Gateway events that also incorporate a corresponding audit log entry
    Hybrid = 3,
    // Action originated from a scheduled recovery job where the bot knew it had
    // ingestion downtime and ran a recovery job to collect all relevant origin/update events
    ScheduledRecovery = 4,
    // Action originated from an unscheduled recovery job where the bot was
    // scanning history and verifying that the logs have the up-to-date state
    UnscheduledRecovery = 5,
    // Action comes from the internal logs endpoint
    Logs = 6,
    // Action comes from some other internal process
    Internal = 7,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum AuditLogEntryType {
    GuildUpdate = 1,
    ChannelCreate = 10,
    ChannelUpdate = 11,
    ChannelDelete = 12,
    ChannelOverwriteCreate = 13,
    ChannelOverwriteUpdate = 14,
    ChannelOverwriteDelete = 15,
    MemberKick = 20,
    MemberPrune = 21,
    MemberBanAdd = 22,
    MemberBanRemove = 23,
    MemberUpdate = 24,
    MemberRoleUpdate = 25,
    MemberMove = 26,
    MemberDisconnect = 27,
    BotAdd = 28,
    RoleCreate = 30,
    RoleUpdate = 31,
    RoleDelete = 32,
    InviteCreate = 40,
    InviteUpdate = 41,
    InviteDelete = 42,
    WebhookCreate = 50,
    WebhookUpdate = 51,
    WebhookDelete = 52,
    EmojiCreate = 60,
    EmojiUpdate = 61,
    EmojiDelete = 62,
    MessageDelete = 72,
    MessageBulkDelete = 73,
    MessagePin = 74,
    MessageUnpin = 75,
    IntegrationCreate = 80,
    IntegrationUpdate = 81,
    IntegrationDelete = 82,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize_repr, Deserialize_repr)]
#[repr(u16)]
pub enum ActionType {
    // Discord audit log events
    GuildUpdate = 1,
    ChannelCreate = 10,
    ChannelUpdate = 11,
    ChannelDelete = 12,
    ChannelOverwriteCreate = 13,
    ChannelOverwriteUpdate = 14,
    ChannelOverwriteDelete = 15,
    MemberKick = 20,
    MemberPrune = 21,
    MemberBanAdd = 22,
    MemberBanRemove = 23,
    MemberUpdate = 24,
    MemberRoleUpdate = 25,
    MemberMove = 26,
    MemberDisconnect = 27,
    BotAdd = 28,
    RoleCreate = 30,
    RoleUpdate = 31,
    RoleDelete = 32,
    InviteCreate = 40,
    InviteUpdate = 41,
    InviteDelete = 42,
    WebhookCreate = 50,
    WebhookUpdate = 51,
    WebhookDelete = 52,
    EmojiCreate = 60,
    EmojiUpdate = 61,
    EmojiDelete = 62,
    MessageDelete = 72,
    MessageBulkDelete = 73,
    MessagePin = 74,
    MessageUnpin = 75,
    IntegrationCreate = 80,
    IntegrationUpdate = 81,
    IntegrationDelete = 82,
    // Discord-related custom events
    MessageSend = 3001,
    MessageEdit = 3002,
    ReactionAdd = 3003,
    ReactionRemove = 3004,
    ReactionRemoveAll = 3005,
    MemberJoin = 3006,
    MemberLeave = 3007,
    GuildUnavailable = 3008,
    VoiceStateUpdate = 3009,
    VoiceServerUpdate = 3010,
    // Auto response events
    AutoResponseCreate = 3100,
    AutoResponseUpdate = 3101,
    AutoResponseDelete = 3102,
    AutoResponseTrigger = 3103,
    // Log events
    LogRevert = 3200,
    LogRollback = 3201,
    LogRecoveryRun = 3202,
    // Custom emoji events
    CustomEmojiCreate = 3300,
    CustomEmojiUpdate = 3301,
    CustomEmojiDelete = 3302,
    CustomEmojiUse = 3303,
    CustomEmojiCache = 3304,
    CustomEmojiLoad = 3305,
    // Settings events
    SettingsUpdate = 3400,
    // General bot events
    ArchitusJoin = 4000,
    ArchitusLeave = 4001,
    UserPrivacyUpdate = 4002,
    // Special action types
    Unknown = 9000,
    InternalDebug = 9100,
    InternalInfo = 9101,
    InternalWarn = 9102,
    InternalError = 9103,
    InternalCritical = 9104,
}
