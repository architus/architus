//! Module for scraping audit log events from the discord http api.

mod utils;

use std::error::Error;
use std::fmt;

use twilight_http::Client;
use twilight_model::id::GuildId;
use twilight_model::guild::audit_log::AuditLog;

/// This is the max number of audit logs that can be returned from a request.
const AUDIT_LOG_REQUEST_LIMIT: u64 = 100;

/// Error type for scrape module.
pub enum ScrapeError {
    /// An error occurred getting the audit logs from the given guild
    AuditLogGet(GuildId),
}

impl fmt::Display for ScrapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ScrapeError::AuditLogGet(_) => write!(f, "failed to get audit logs for guild"),
        }
    }
}

impl fmt::Debug for ScrapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ScrapeError::AuditLogGet(g) => write!(f, "failed to get audit logs for guild {}", g.0),
        }
    }
}

impl Error for ScrapeError {}

pub type ScrapeResult<T> = Result<T, ScrapeError>;

pub async fn scrape_timespan(client: &Client, guild: GuildId, timespan: (u64, u64)) -> ScrapeResult<AuditLog> {
    let mut logs = AuditLog {
        entries: Vec::new(),
        integrations: Vec::new(),
        users: Vec::new(),
        webhooks: Vec::new(),
    };

    let curr_event_batch = scrape_before(client, guild, utils::bound_from_ts(timespan.1)).await;
}

/// Gets up to 100 audit log events from `guild` before the event referenced by `event_id`
/// occurred.
pub async fn scrape_before(client: &Client, guild: GuildId, event_id: Option<u64>) -> ScrapeResult<AuditLog> {
    let request = match event_id {
        Some(e) => {
            client
                .audit_log(guild)
                // I think the event_id is supposed to the an AuditLogEntryId but the docs say it
                // should be a straight u64 so it must be an unwrapped one.
                .before(e)
                .limit(AUDIT_LOG_REQUEST_LIMIT).expect("This will never fail as long as `AUDIT_LOG_REQUEST_LIMIT` is kept up to date")
                .exec().await
        },
        None => {
            client
                .audit_log(guild)
                .limit(AUDIT_LOG_REQUEST_LIMIT).expect("This will never fail as long as `AUDIT_LOG_REQUEST_LIMIT` is kept up to date")
                .exec().await
        },
    };

    let response = match request {
        Ok(r) => r,
        Err(_) => return Err(ScrapeError::AuditLogGet(guild)),
    };

    let data = response.model().await;
    match data {
        Ok(d) => Ok(d),
        Err(_) => Err(ScrapeError::AuditLogGet(guild)),
    }
}
