//! Collection of small fairings to implement
//! request logging, request id pinning, and request timing.

/// Attaches an ID to each incoming request
/// that can be used to correlate log messages
pub mod request_id {
    use rocket::fairing::{Info, Kind};
    use rocket::http::Status;
    use rocket::request::{FromRequest, Outcome};
    use rocket::{Data, Request};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Value stored in request-local state.
    pub(super) struct AttachedId(pub(super) Option<usize>);

    /// Fairing for attaching request ids
    pub struct Fairing {
        current: AtomicUsize,
    }

    impl Fairing {
        pub fn new() -> Self {
            Self {
                current: AtomicUsize::new(0),
            }
        }
    }

    #[rocket::async_trait]
    impl rocket::fairing::Fairing for Fairing {
        fn info(&self) -> Info {
            Info {
                name: "Request IDs",
                kind: Kind::Request,
            }
        }

        /// Stores the request id in request-local state.
        async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
            request.local_cache(|| AttachedId(Some(self.current.fetch_add(1, Ordering::SeqCst))));
        }
    }

    /// Request guard used to retrieve a request's ID
    #[derive(Clone)]
    pub struct RequestId(pub usize);

    // Allows a route to access its ID
    #[rocket::async_trait]
    impl<'r> FromRequest<'r> for RequestId {
        type Error = ();

        async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ()> {
            match *request.local_cache(|| AttachedId(None)) {
                AttachedId(Some(id)) => Outcome::Success(RequestId(id)),
                AttachedId(None) => Outcome::Failure((Status::InternalServerError, ())),
            }
        }
    }
}

/// Attaches a request-scoped logger to each incoming request,
/// using the request ID fairing if available to attach a structured log field.
pub mod attach_logger {
    use rocket::fairing::{Info, Kind};
    use rocket::http::Status;
    use rocket::request::{FromRequest, Outcome};
    use rocket::{Data, Request};
    use slog::Logger;

    /// Value stored in request-local state.
    pub(super) struct AttachedLogger(pub(super) Option<Logger>);

    /// Fairing for attaching loggers
    pub struct Fairing {
        logger: Logger,
    }

    impl Fairing {
        pub fn new(logger: Logger) -> Self {
            Self { logger }
        }
    }

    #[rocket::async_trait]
    impl rocket::fairing::Fairing for Fairing {
        fn info(&self) -> Info {
            Info {
                name: "Attach Logger",
                kind: Kind::Request,
            }
        }

        /// Stores the logger in request-local state.
        /// Depends on `request_id` if it is available.
        async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
            let request_id = request
                .local_cache(|| super::request_id::AttachedId(None))
                .0;
            request.local_cache(|| {
                let logger = match request_id {
                    Some(id) => self.logger.new(slog::o!("request_id" => id)),
                    None => self.logger.clone(),
                };
                AttachedLogger(Some(logger))
            });
        }
    }

    /// Request guard used to retrieve a request-scoped logger
    #[derive(Clone)]
    pub struct RequestLogger(pub Logger);

    // Allows a route to access its logger
    #[rocket::async_trait]
    impl<'r> FromRequest<'r> for RequestLogger {
        type Error = ();

        async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ()> {
            match &*request.local_cache(|| AttachedLogger(None)) {
                AttachedLogger(Some(logger)) => Outcome::Success(RequestLogger(logger.clone())),
                AttachedLogger(None) => Outcome::Failure((Status::InternalServerError, ())),
            }
        }
    }
}

/// Logs each request and its timing.
pub mod request_logging {
    use rocket::fairing::{Info, Kind};
    use rocket::http::Status;
    use rocket::request::{FromRequest, Outcome};
    use rocket::{Data, Request, Response};
    use slog::Logger;
    use std::time::SystemTime;

    /// Value stored in request-local state.
    struct TimerStart(Option<SystemTime>);

    /// Fairing for logging requests.
    /// Logging is sent to the request's scoped logger
    /// (attached with `fairings::attach_logger`) if available,
    /// otherwise `fallback_logger` is used as a fallback.
    pub struct Fairing {
        fallback_logger: Logger,
    }

    impl Fairing {
        pub fn new(fallback_logger: Logger) -> Self {
            Self { fallback_logger }
        }
    }

    #[rocket::async_trait]
    impl rocket::fairing::Fairing for Fairing {
        fn info(&self) -> Info {
            Info {
                name: "Request Logging",
                kind: Kind::Request | Kind::Response,
            }
        }

        /// Stores the start time in request-local state.
        async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
            request.local_cache(|| TimerStart(Some(SystemTime::now())));
        }

        /// Notes how long the request took to process, and log the response
        async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
            let start_time = req.local_cache(|| TimerStart(None));
            let response_timing = if let Some(Ok(duration)) = start_time.0.map(|st| st.elapsed()) {
                Some(duration.as_secs() * 1000 + duration.subsec_millis() as u64)
            } else {
                None
            };

            // Use the request logger if available
            let logger = req
                .local_cache(|| super::attach_logger::AttachedLogger(None))
                .0
                .as_ref()
                .unwrap_or_else(|| &self.fallback_logger);

            // Log using the response timing if available
            match response_timing {
                None => log_request(logger, req, res),
                Some(timing) => {
                    let logger_with_timing = logger.new(slog::o!("latency_ms" => timing));
                    log_request(&logger_with_timing, req, res);
                }
            };
        }
    }

    fn log_request<'r>(logger: &Logger, req: &'r Request<'_>, res: &mut Response<'r>) {
        slog::info!(
            logger,
            "handled request";
            "method" => %req.method(),
            "uri" => %req.uri(),
            "status" => %res.status(),
        );
    }

    /// Request guard used to retrieve the start time of a request.
    #[derive(Copy, Clone)]
    pub struct StartTime(pub SystemTime);

    // Allows a route to access the time a request was initiated.
    #[rocket::async_trait]
    impl<'r> FromRequest<'r> for StartTime {
        type Error = ();

        async fn from_request(request: &'r Request<'_>) -> Outcome<Self, ()> {
            match *request.local_cache(|| TimerStart(None)) {
                TimerStart(Some(time)) => Outcome::Success(StartTime(time)),
                TimerStart(None) => Outcome::Failure((Status::InternalServerError, ())),
            }
        }
    }
}
