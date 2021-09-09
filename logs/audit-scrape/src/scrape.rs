//! Module for scraping audit log events from the discord http api.

use std::error::Error;
use std::fmt;

use twilight_http::Client;
use twilight_model::id::GuildId;
use twilight_model::guild::audit_log::AuditLog;

/// Limit of how many guilds discord will send in response to asking which guilds we're in.
const GUILD_REQUEST_LIMIT: u64 = 200;

/// This is the max number of audit logs that can be returned from a request.
const AUDIT_LOG_REQUEST_LIMIT: u64 = 100;

/// Error type for scrape module.
pub enum ScrapeError {
    /// An error occurred getting the audit logs from the given guild
    AuditLogGet(GuildId),
    /// Unable to get a list of current guilds from architus
    ListGuilds,
}

impl fmt::Display for ScrapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ScrapeError::AuditLogGet(_) => write!(f, "failed to get audit logs for guild"),
            ScrapeError::ListGuilds => write!(f, "failed to get architus guild from discord"),
        }
    }
}

impl fmt::Debug for ScrapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ScrapeError::AuditLogGet(g) => write!(f, "failed to get audit logs for guild {}", g.0),
            ScrapeError::ListGuilds => write!(f, "failed to get architus guild from discord"),
        }
    }
}

impl Error for ScrapeError {}

pub type ScrapeResult<T> = Result<T, ScrapeError>;

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

/// Asks discord for a list of all of the guilds that architus is in.
pub async fn get_guilds(client: &Client) -> ScrapeResult<Vec<GuildId>> {
    // Architus is currently only in ~450 guilds. 1000 is a good number that
    // allows room to grow. Also, a GuildId is just a newtype for a u64 so
    // this shouldn't actually take up that much space (like 2 pages).
    let mut architus_guilds: Vec<GuildId> = Vec::with_capacity(1000);

    let request = client.current_user_guilds()
        .limit(GUILD_REQUEST_LIMIT).expect("Will succeed as long as `GUILD_REQUEST_LIMIT` is kept up to date")
        .exec().await;

    let response = match request {
        Ok(r) => r,
        Err(_) => return Err(ScrapeError::ListGuilds),
    };

    let data = response.models().await;
    let guilds = match data {
        Ok(d) => d,
        Err(_) => return Err(ScrapeError::ListGuilds),
    };

    for g in &guilds {
        architus_guilds.push(g.id);
    }

    while guilds.len() >= (GUILD_REQUEST_LIMIT as usize) {
        let request = client.current_user_guilds()
            .limit(GUILD_REQUEST_LIMIT).expect("Will succeed as long as `GUILD_REQUEST_LIMIT` is kept up to date")
            .exec().await;

        let response = match request {
            Ok(r) => r,
            Err(_) => return Err(ScrapeError::ListGuilds),
        };

        let data = response.models().await;
        let guilds = match data {
            Ok(d) => d,
            Err(_) => return Err(ScrapeError::ListGuilds),
        };

        for g in &guilds {
            architus_guilds.push(g.id);
        }
    }

    Ok(architus_guilds)
}
