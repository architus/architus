use backoff;
use backoff::backoff::Backoff;
use logs_lib::id;
use logs_lib::{time, AuditLogEntryType};
use reqwest::StatusCode;
use serenity;
use serenity::http::error::ErrorResponse;
use serenity::http::Http;
use serenity::model::guild::AuditLogEntry;
use serenity::model::id::{AuditLogEntryId, GuildId, UserId};
use std::sync::Arc;
use std::time::Duration;

/// Aggregate options struct for performing a resilient search
/// on the audit logs of a guild
pub struct SearchQuery<P: Fn(&AuditLogEntry) -> bool> {
    pub guild_id: GuildId,
    pub entry_type: Option<AuditLogEntryType>,
    pub target_timestamp: Option<u64>,
    pub strategy: Strategy,
    pub matches: P,
    pub initial_retry_interval: Option<Duration>,
    pub max_retry_interval: Option<Duration>,
    /// Max length to spend attempting to find the audit log entry
    pub max_search_duration: Option<Duration>,
    pub chunk_size: Option<u8>,
    pub timestamp_ignore_threshold: Option<Duration>,
    pub user_id: Option<UserId>,
}

impl<P: Fn(&AuditLogEntry) -> bool> SearchQuery<P> {
    #[must_use]
    pub fn new(guild_id: GuildId, matches: P) -> Self {
        Self {
            guild_id,
            matches,
            entry_type: None,
            target_timestamp: None,
            strategy: Strategy::First,
            initial_retry_interval: None,
            max_retry_interval: None,
            max_search_duration: None,
            chunk_size: None,
            timestamp_ignore_threshold: None,
            user_id: None,
        }
    }

    #[must_use]
    fn max_duration(&self) -> Duration {
        self.max_search_duration.unwrap_or(Duration::from_secs(15))
    }

    #[must_use]
    fn make_backoff(&self) -> backoff::ExponentialBackoff {
        let initial_interval = self
            .initial_retry_interval
            .unwrap_or(Duration::from_millis(400));
        backoff::ExponentialBackoff {
            max_elapsed_time: Some(self.max_duration()),
            current_interval: initial_interval,
            initial_interval: initial_interval,
            max_interval: self.max_retry_interval.unwrap_or(Duration::from_secs(4)),
            ..Default::default()
        }
    }
}

/// Possible errors while attempting to search for audit log entries
pub enum Error {
    SerenityError(serenity::Error),
    Unauthorized(ErrorResponse),
    SearchExhausted,
    TimedOut,
}

type BackoffError<T> = backoff::Error<T>;
type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq)]
struct SearchTiming {
    start: u64,
    target: u64,
}

#[derive(Clone)]
pub enum Strategy {
    /// Uses the first audit log entry that is within the absolute bounds
    /// and matches the supplied match function
    First,
    /// Creates a growing acceptance interval
    GrowingInterval { max: Duration },
}

impl Strategy {
    #[must_use]
    fn matches<P>(
        &self,
        timing: SearchTiming,
        entry: &AuditLogEntry,
        search: &SearchQuery<P>,
    ) -> bool
    where
        P: Fn(&AuditLogEntry) -> bool,
    {
        match self {
            Self::First => true,
            Self::GrowingInterval { max } => {
                let timestamp = time::millisecond_ts();
                // Construct the interval based on how much time has elapsed
                // since the start
                let ratio: f64 = max.as_secs_f64() / search.max_duration().as_secs_f64();
                let ms_passed = timestamp - timing.start;
                let interval_width = ((ms_passed as f64) * ratio) as u64;
                let lower = timing.target - interval_width;
                let upper = timing.target + interval_width;

                // Only match if the entry's timestamp is inside the interval
                let entry_ts = id::extract_timestamp(entry.id.0);
                entry_ts > lower && entry_ts < upper
            }
        }
    }
}

/// Determines if the given serenity error is an HTTP unauthorized
fn unauthorized_response(err: &serenity::Error) -> Option<ErrorResponse> {
    if let serenity::Error::Http(boxed_err) = err {
        return match &**boxed_err {
            serenity::http::HttpError::UnsuccessfulRequest(response) => {
                if response.status_code == StatusCode::UNAUTHORIZED {
                    return Some(response.clone());
                } else {
                    return None;
                }
            }
            _ => None,
        };
    };

    None
}

/// Attempts to get an audit log entry corresponding to some other target action,
/// repeatedly searching the most recent audit log entries to find a matching
/// entry.
///
/// Times out after 15 minutes
pub async fn get_entry<P>(http: Arc<Http>, search: SearchQuery<P>) -> Result<AuditLogEntry>
where
    P: Fn(&AuditLogEntry) -> bool,
{
    let mut backoff = search.make_backoff();
    let start = time::millisecond_ts();
    let timing = SearchTiming {
        start,
        target: search.target_timestamp.unwrap_or(start),
    };

    loop {
        let result = try_get_entry(Arc::clone(&http), &search, timing).await;
        let err = match result {
            Ok(v) => return Ok(v),
            Err(err) => err,
        };

        match err {
            BackoffError::Permanent(err) => return Err(err),
            BackoffError::Transient(err) => {
                if let Error::SerenityError(serenity_err) = &err {
                    if let Some(response) = unauthorized_response(&serenity_err) {
                        return Err(Error::Unauthorized(response));
                    }
                }
            }
        };

        let next = match backoff.next_backoff() {
            Some(next) => next,
            None => return Err(Error::TimedOut),
        };

        tokio::time::delay_for(next).await;
    }
}

/// Attempts to get a single audit log entry during a backoff loop
async fn try_get_entry<P>(
    http: Arc<Http>,
    search: &SearchQuery<P>,
    timing: SearchTiming,
) -> std::result::Result<AuditLogEntry, BackoffError<Error>>
where
    P: Fn(&AuditLogEntry) -> bool,
{
    // limit is a tradeoff of time in situations where a single event of the target type
    // occurs in a small interval to situations where many events of the target type
    // occur in a small interval
    let limit = search.chunk_size.unwrap_or(5);
    // determines the max number of seconds to search back in the
    // audit log for an entry since the target timestamp
    let time_threshold = search
        .timestamp_ignore_threshold
        .map(|dur| dur.as_millis() as u64)
        .unwrap_or(60_000);

    let mut before: Option<AuditLogEntryId> = None;
    // traverse the audit log history
    loop {
        let entries = search
            .guild_id
            .audit_logs(
                Arc::clone(&http),
                search.entry_type.map(|t| t as u8),
                search.user_id,
                before,
                Some(limit),
            )
            .await
            .map_err(|err| Error::SerenityError(err))?
            .entries;

        if entries.len() == 0 {
            break;
        }

        // attempt to find the desired matching entry
        let mut oldest: Option<AuditLogEntryId> = None;
        for (key, value) in entries {
            if (search.matches)(&value) && search.strategy.matches(timing, &value, search) {
                return Ok(value);
            }
            // update oldest entry if older then oldest
            if oldest.map(|id| id > key).unwrap_or(true) {
                oldest = Some(key);
            }
        }

        // determine whether to continue
        let oldest_timestamp = oldest.map(|i| id::extract_timestamp(i.0)).unwrap_or(0);
        if (timing.target - oldest_timestamp) > time_threshold {
            break;
        }

        before = oldest;
    }

    // Search was either exhausted or stopped early due to passing the threshold;
    // mark as transient error and backoff
    Err(backoff::Error::Transient(Error::SearchExhausted))
}
