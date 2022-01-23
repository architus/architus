use futures::{Stream, StreamExt};
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::{Format, Severity};
use sloggers::Build;
use std::future::Future;
use tokio::sync::mpsc;

lazy_static::lazy_static! {
    static ref LOGGER: slog::Logger = {
        let mut builder = TerminalLoggerBuilder::new();
        builder.level(Severity::Info);
        builder.destination(Destination::Stderr);
        builder.format(Format::Full);
        builder.build().unwrap()
    };
}

pub fn logger(test_name: &'static str) -> slog::Logger {
    LOGGER.new(slog::o!("test_name" => test_name))
}

pub fn adapt_stream<T, S>(
    mut in_stream: S,
) -> (impl Future<Output = ()>, mpsc::UnboundedReceiver<T>)
where
    T: std::fmt::Debug,
    S: Unpin + Stream<Item = T>,
{
    let (items_tx, items_rx) = mpsc::unbounded_channel::<T>();
    let adapt_future = async move {
        while let Some(item) = in_stream.next().await {
            if items_tx.send(item).is_err() {
                return;
            }
        }
    };
    (adapt_future, items_rx)
}

pub fn assert_empty<T>(rx: &mut mpsc::UnboundedReceiver<T>) -> bool {
    let mut fake_context = std::task::Context::from_waker(futures::task::noop_waker_ref());
    match rx.poll_recv(&mut fake_context) {
        std::task::Poll::Ready(Some(_)) => false,
        std::task::Poll::Pending | std::task::Poll::Ready(None) => true,
    }
}
