//! Defines a generalized data structure that is a thread-safe set of items
//! that can be added to or removed from,
//! and re-emits batch add and delete events after a short debouncing delay.

use log::warn;
use static_assertions::assert_impl_all;
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::FromIterator;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

/// Represents a single released update from a debounced pool,
/// including all items added to or removed from the pool since the last release.
#[derive(Clone, Debug, PartialEq)]
pub struct Update<T> {
    pub added: Option<Vec<T>>,
    pub removed: Option<Vec<T>>,
}

/// Encapsulates a pool of items that can be added to or removed from,
/// and will emit debounced update events after a short delay
/// that include all items added to or removed from the pool in that interval.
/// Useful for getting bulk addition/removal events from a pool
/// that receives many updates in quick succession
#[derive(Clone, Debug)]
pub struct DebouncedPool<T: Clone + Eq + Hash> {
    delay: Arc<Duration>,
    inner: Arc<Mutex<PoolInner<T>>>,
    release_publish: mpsc::UnboundedSender<Update<T>>,
}

// Make sure a pool of guild ids is sync+send
assert_impl_all!(DebouncedPool<u64>: Sync, Send);

impl<T: Clone + Debug + Eq + Hash + Send + 'static> DebouncedPool<T> {
    /// Creates a new shared debounced pool
    /// and returns a consumer for the published update messages
    pub fn new(delay: Duration) -> (Self, mpsc::UnboundedReceiver<Update<T>>) {
        let (release_publish, release_consume) = mpsc::unbounded_channel::<Update<T>>();
        let new_pool = Self {
            inner: Arc::new(Mutex::new(PoolInner::new())),
            delay: Arc::new(delay),
            release_publish,
        };
        (new_pool, release_consume)
    }

    /// Adds a single item to the pool,
    /// potentially starting a timer for an update
    #[allow(clippy::if_same_then_else)]
    pub fn add(&self, value: T) {
        let mut inner_state = self.inner.lock().expect("debounced pool poisoned");
        if inner_state.removed.contains(&value) {
            // Reverse the removal before it gets released
            // (note: we don't need to check to potentially release
            // since a non-empty removed set means a release is pending)
            inner_state.removed.remove(&value);
        } else if inner_state.pool.contains(&value) {
            // Pool already contains item; no-op
        } else if inner_state.added.contains(&value) {
            // Addition set already contains item; no-op
        } else {
            // Prepare to add the item, and start a release task if not already started
            // (make sure to atomically mark a release as being prepared)
            inner_state.added.insert(value);
            if inner_state.preparing_release_cancel.is_none() {
                inner_state.preparing_release_cancel = Some(self.start_release_timer());
            }
        }
    }

    /// Removes a single item from the pool,
    /// potentially starting a timer for an update
    #[allow(clippy::if_same_then_else)]
    pub fn remove(&self, value: T) {
        let mut inner_state = self.inner.lock().expect("debounced pool poisoned");
        if inner_state.removed.contains(&value) {
            // Reverse the addition before it gets released
            // (note: we don't need to check to potentially release
            // since a non-empty addition set means a release is pending)
            inner_state.added.remove(&value);
        } else if !inner_state.pool.contains(&value) {
            // Pool doesn't contain item; no-op
        } else if inner_state.added.contains(&value) {
            // Removal set already contains item; no-op
        } else {
            // Prepare to remove the item, and start a release task if not already started
            inner_state.removed.insert(value);
            if inner_state.preparing_release_cancel.is_none() {
                inner_state.preparing_release_cancel = Some(self.start_release_timer());
            }
        }
    }

    /// Attempts to release the currently debounced updates immediately, if they exist.
    /// If a debounced release timer is currently running, then cancels it.
    pub fn release(&self) -> Option<Update<T>> {
        let mut inner_state = self.inner.lock().expect("debounced pool poisoned");
        let cancel = std::mem::replace(&mut inner_state.preparing_release_cancel, None);
        if let Some(cancel) = cancel {
            if let Err(err) = cancel.send(()) {
                warn!(
                    "debounced pool had non-None cancellation but channel was closed: {:?}",
                    err
                );
            };
        }

        // See if there are any items to release
        if !inner_state.added.is_empty() || !inner_state.removed.is_empty() {
            inner_state.release()
        } else {
            None
        }
    }

    /// Retrieves all items in the current state of the pool.
    /// Note that this does not include unreleased updates
    pub fn items<U: FromIterator<T>>(&self) -> U {
        let inner_state = self.inner.lock().expect("debounced pool poisoned");
        inner_state.pool.iter().cloned().collect::<U>()
    }

    /// Starts a Tokio task for releasing the current pool status,
    /// returning a oneshot channel that can be used to cancel the release
    /// (for example, if a manual release was performed early)
    fn start_release_timer(&self) -> oneshot::Sender<()> {
        let (cancel_sender, mut cancel_receiver) = oneshot::channel::<()>();
        let channel = self.release_publish.clone();
        let inner_state_mutex = Arc::clone(&self.inner);
        let delay = Arc::clone(&self.delay);
        tokio::spawn(async move {
            tokio::time::delay_for(*delay).await;
            let mut inner_state = inner_state_mutex.lock().expect("debounced pool poisoned");

            // Make sure the release wasn't cancelled
            if cancel_receiver.try_recv().is_ok() {
                return;
            }

            // Get the update and add all updates to the inner pool
            let update = inner_state.release();

            // Clear the timer and drop the lock
            inner_state.preparing_release_cancel = None;
            drop(inner_state);

            // Publish the release
            if let Some(update) = update {
                channel
                    .send(update)
                    .expect("DebouncedPool send failed; listeners have died");
            }
        });
        cancel_sender
    }
}

/// Contains the inner mutable state of a pool
/// (requires synchronization)
#[derive(Debug)]
struct PoolInner<T: Clone + Eq + Hash> {
    pool: HashSet<T>,
    added: HashSet<T>,
    removed: HashSet<T>,
    preparing_release_cancel: Option<oneshot::Sender<()>>,
}

impl<T: Clone + Eq + Hash> PoolInner<T> {
    fn new() -> Self {
        Self {
            pool: HashSet::new(),
            added: HashSet::new(),
            removed: HashSet::new(),
            preparing_release_cancel: None,
        }
    }

    fn release(&mut self) -> Option<Update<T>> {
        // Process added items if there are any
        let added = if self.added.is_empty() {
            None
        } else {
            let mut added_vec = Vec::with_capacity(self.added.len());
            for item in self.added.drain() {
                self.pool.insert(item.clone());
                added_vec.push(item);
            }
            Some(added_vec)
        };

        // Process removed items if there are any
        let removed = if self.removed.is_empty() {
            None
        } else {
            let mut removed_vec = Vec::with_capacity(self.removed.len());
            for item in self.removed.drain() {
                self.pool.remove(&item);
                removed_vec.push(item);
            }
            Some(removed_vec)
        };

        // Only return an update if there were any items
        if added.is_some() || removed.is_some() {
            Some(Update { added, removed })
        } else {
            warn!("a release on a DebouncedPool was performed but no items were updated");
            None
        }
    }
}
