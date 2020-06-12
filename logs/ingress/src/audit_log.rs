use backoff;
use backoff_futures::BackoffExt;
use logs_lib::id;
use logs_lib::{time, AuditLogEntryType};
use serenity;
use serenity::http::Http;
use serenity::model::guild::{AuditLogEntry, Guild};
use serenity::model::id::AuditLogEntryId;
use std::sync::Arc;

/// Attempts to get an audit log entry corresponding to some other target action,
/// repeatedly searching the most recent audit log entries to find a matching
/// entry.
///
/// Times out after 15 minutes
pub async fn get_entry<P>(
    http: Arc<Http>,
    guild: Guild,
    entry_type: AuditLogEntryType,
    target_timestamp: Option<u64>,
    matches: P,
) -> Result<AuditLogEntry, backoff::Error<serenity::Error>>
where
    P: Fn(&AuditLogEntry) -> bool,
{
    let target_timestamp = target_timestamp.unwrap_or_else(|| time::millisecond_ts());
    type BackoffError = backoff::Error<serenity::Error>;
    let get_log_entry = || async {
        // limit is a tradeoff of time in situations where a single channel is made in a small interval
        // to situations where many channels are made in a small interval
        let limit = 5;
        // determines the max number of seconds to search back in the
        // audit log for an entry since the creation of the channel
        let time_threshold = 60_000;
        let mut before: Option<AuditLogEntryId> = None;
        // traverse the audit log history
        loop {
            let entries = guild
                .audit_logs(
                    Arc::clone(&http),
                    Some(entry_type as u8),
                    None,
                    before,
                    Some(limit),
                )
                .await?
                .entries;
            if entries.len() == 0 {
                break;
            }

            // attempt to find the entry corresponding to the channel
            let mut oldest: Option<AuditLogEntryId> = None;
            for (key, value) in entries {
                // try to match the current entry
                if matches(&value) {
                    return Ok::<AuditLogEntry, BackoffError>(value);
                }
                // update oldest entry
                if oldest.map(|id| id > key).unwrap_or(true) {
                    oldest = Some(key);
                }
            }

            // determine whether to continue
            let oldest_timestamp = oldest.map(|i| id::extract_timestamp(i.0)).unwrap_or(0);
            if (target_timestamp - oldest_timestamp) > time_threshold {
                break;
            }

            before = oldest;
        }

        // Search was either exhausted or stopped early due to passing the threshold;
        // mark as transient error and backoff
        Err::<AuditLogEntry, BackoffError>(backoff::Error::Transient(serenity::Error::Other(
            "exhausted audit log search",
        )))
    };

    let mut backoff = backoff::ExponentialBackoff::default();
    get_log_entry.with_backoff(&mut backoff).await
}
