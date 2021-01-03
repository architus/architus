use backoff::backoff::Backoff;
use backoff::ExponentialBackoff;
use std::convert::TryFrom;
use std::time::Duration;
use twilight_http::Client;
use twilight_model::guild::audit_log::{AuditLog, AuditLogEntry, AuditLogEvent};
use twilight_model::id::{AuditLogEntryId, GuildId, UserId};

/// Aggregate options struct for performing a resilient search
/// on the audit logs of a guild
pub struct SearchQuery<P: Fn(&AuditLogEntry) -> bool> {
    pub guild_id: u64,
    pub entry_type: Option<AuditLogEvent>,
    pub target_timestamp: Option<u64>,
    pub strategy: Strategy,
    pub matches: P,
    pub initial_retry_interval: Option<Duration>,
    pub max_retry_interval: Option<Duration>,
    /// Max length to spend attempting to find the audit log entry
    pub max_search_duration: Option<Duration>,
    pub chunk_size: Option<u8>,
    pub timestamp_ignore_threshold: Option<Duration>,
    pub user_id: Option<u64>,
}

impl<P: Fn(&AuditLogEntry) -> bool> SearchQuery<P> {
    #[must_use]
    pub fn new(guild_id: u64, matches: P) -> Self {
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
        self.max_search_duration
            .unwrap_or_else(|| Duration::from_secs(15))
    }

    #[must_use]
    fn make_backoff(&self) -> ExponentialBackoff {
        let initial_interval = self
            .initial_retry_interval
            .unwrap_or_else(|| Duration::from_millis(400));
        ExponentialBackoff {
            max_elapsed_time: Some(self.max_duration()),
            current_interval: initial_interval,
            initial_interval,
            max_interval: self
                .max_retry_interval
                .unwrap_or_else(|| Duration::from_secs(4)),
            ..ExponentialBackoff::default()
        }
    }
}

type TwilightError = twilight_http::Error;

/// Possible errors while attempting to search for audit log entries
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("an error occurred while using the Discord API")]
    Twilight(#[source] TwilightError),
    #[error("the bot does not have access to the audit log")]
    Unauthorized,
    #[error("no audit log entry could be found during the search")]
    SearchExhausted,
    #[error("the search could not be completed in the allotted interval")]
    TimedOut,
    #[error("the limit provided was invalid")]
    LimitInvalid(#[source] twilight_http::request::guild::get_audit_log::GetAuditLogError),
}

type BackoffError<T> = backoff::Error<T>;
type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq)]
struct SearchTiming {
    start: u64,
    target: u64,
}

#[derive(Debug, Clone)]
pub enum Strategy {
    /// Uses the first audit log entry that is within the absolute bounds
    /// and matches the supplied match function
    First,
    /// Creates a growing acceptance interval
    GrowingInterval { max: Duration },
}

impl Strategy {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
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
                let timestamp = architus_id::time::millisecond_ts();
                // Construct the interval based on how much time has elapsed
                // since the start
                let ratio: f64 = max.as_secs_f64() / search.max_duration().as_secs_f64();
                // use saturating overflows to prevent overflows
                let ms_passed = timestamp.saturating_sub(timing.start);
                let interval_width = ((ms_passed as f64) * ratio).round() as u64;
                let lower = timing.target.saturating_sub(interval_width);
                let upper = timing.target.saturating_add(interval_width);

                // Only match if the entry's timestamp is inside the interval
                let entry_ts = architus_id::extract_timestamp(entry.id.0);
                entry_ts > lower && entry_ts < upper
            }
        }
    }
}

/// Attempts to get an audit log entry corresponding to some other target action,
/// repeatedly searching the most recent audit log entries to find a matching
/// entry.
///
/// Times out after 15 minutes
pub async fn get_entry<P>(client: &Client, search: SearchQuery<P>) -> Result<AuditLogEntry>
where
    P: Fn(&AuditLogEntry) -> bool,
{
    let mut backoff = search.make_backoff();
    let start = architus_id::time::millisecond_ts();
    let timing = SearchTiming {
        start,
        target: search.target_timestamp.unwrap_or(start),
    };

    loop {
        let result = try_get_entry(client, &search, timing).await;
        let err = match result {
            Ok(v) => return Ok(v),
            Err(err) => err,
        };

        match err {
            BackoffError::Permanent(err) => return Err(err),
            BackoffError::Transient(Error::Twilight(TwilightError::Unauthorized)) => {
                return Err(Error::Unauthorized);
            }
            _ => {}
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
    client: &Client,
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
        .as_ref()
        .map(Duration::as_millis)
        .and_then(|u| u64::try_from(u).ok())
        .unwrap_or(60_000);

    let mut before: Option<AuditLogEntryId> = None;
    // traverse the audit log history
    loop {
        let mut get_audit_log = client
            .audit_log(GuildId(search.guild_id))
            .limit(u64::from(limit))
            .map_err(Error::LimitInvalid)?;
        if let Some(before) = before {
            get_audit_log = get_audit_log.before(before.0);
        }
        if let Some(action_type) = search.entry_type {
            get_audit_log = get_audit_log.action_type(action_type);
        }
        if let Some(user_id) = search.user_id {
            get_audit_log = get_audit_log.user_id(UserId(user_id));
        }
        let entries_option = get_audit_log.await.map_err(Error::Twilight)?;

        match entries_option {
            None => break,
            Some(AuditLog {
                audit_log_entries, ..
            }) if audit_log_entries.is_empty() => break,
            Some(AuditLog {
                audit_log_entries, ..
            }) => {
                // attempt to find the desired matching entry
                let mut oldest: Option<AuditLogEntryId> = None;
                for entry in audit_log_entries {
                    if (search.matches)(&entry) && search.strategy.matches(timing, &entry, search) {
                        return Ok(entry);
                    }
                    // update oldest entry if older then oldest
                    if oldest.map_or(true, |id| id > entry.id) {
                        oldest = Some(entry.id);
                    }
                }

                // determine whether to continue
                let oldest_timestamp = oldest.map_or(0, |i| architus_id::extract_timestamp(i.0));
                if (timing.target - oldest_timestamp) > time_threshold {
                    break;
                }

                before = oldest;
            }
        }
    }

    // Search was either exhausted or stopped early due to passing the threshold;
    // mark as transient error and backoff
    Err(backoff::Error::Transient(Error::SearchExhausted))
}
