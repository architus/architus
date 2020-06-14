use backoff;
use backoff::backoff::Backoff;
use logs_lib::id;
use logs_lib::{time, AuditLogEntryType};
use reqwest::StatusCode;
use serenity;
use serenity::http::Http;
use serenity::model::guild::{AuditLogEntry, Guild};
use serenity::model::id::AuditLogEntryId;
use std::sync::Arc;
use std::time::Duration;

/// Configures the backoff struct used
fn make_backoff() -> backoff::ExponentialBackoff {
    backoff::ExponentialBackoff {
        max_elapsed_time: Some(Duration::from_secs(15)),
        current_interval: Duration::from_millis(400),
        initial_interval: Duration::from_millis(400),
        max_interval: Duration::from_secs(4),
        ..Default::default()
    }
}

/// Determines if the given serenity error is an HTTP unauthorized
fn is_http_unauthorized(err: &serenity::Error) -> bool {
    if let serenity::Error::Http(boxed_err) = err {
        return match &**boxed_err {
            serenity::http::HttpError::UnsuccessfulRequest(response) => {
                response.status_code == StatusCode::UNAUTHORIZED
            }
            _ => false,
        };
    }

    false
}

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
    let mut backoff = make_backoff();
    let target_timestamp = target_timestamp.unwrap_or_else(|| time::millisecond_ts());
    loop {
        use backoff::Error;

        let future = try_get_entry(
            Arc::clone(&http),
            &guild,
            entry_type,
            target_timestamp,
            &matches,
        );

        let err = match future.await {
            Ok(v) => return Ok(v),
            Err(err) => err,
        };

        let err = match err {
            Error::Permanent(err) => return Err(Error::Permanent(err)),
            Error::Transient(err) => {
                if is_http_unauthorized(&err) {
                    return Err(Error::Permanent(err));
                }
                err
            }
        };

        let next = match backoff.next_backoff() {
            Some(next) => next,
            None => return Err(Error::Transient(err)),
        };

        tokio::time::delay_for(next).await;
    }
}

/// Attempts to get a single audit log entry during a backoff loop
async fn try_get_entry<P>(
    http: Arc<Http>,
    guild: &Guild,
    entry_type: AuditLogEntryType,
    target_timestamp: u64,
    matches: &P,
) -> Result<AuditLogEntry, backoff::Error<serenity::Error>>
where
    P: Fn(&AuditLogEntry) -> bool,
{
    // limit is a tradeoff of time in situations where a single event of the target type
    // occurs in a small interval to situations where many events of the target type
    // occur in a small interval
    let limit = 5;
    // determines the max number of seconds to search back in the
    // audit log for an entry since the target timestamp
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

        // attempt to find the desired matching entry
        let mut oldest: Option<AuditLogEntryId> = None;
        for (key, value) in entries {
            if matches(&value) {
                return Ok(value);
            }
            // update oldest entry if older then oldest
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
    // TODO get better errors
    Err(backoff::Error::Transient(serenity::Error::Other(
        "exhausted audit log search",
    )))
}
