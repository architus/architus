pub mod logging {
    tonic::include_proto!("logging");
}

use logging::{SubmitRequest, Event, SubmitReply};
use logging::logging_server::Logging;

fn main() {
    println!("Hello, world!");
}

#[derive(Debug)]
struct LoggingService;
