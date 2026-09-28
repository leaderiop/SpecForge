use std::collections::BTreeSet;
use std::sync::mpsc;
use std::time::Duration;

/// Default coalescing window for watcher batches: short enough to feel
/// live, long enough to collapse editor save storms.
pub const DEFAULT_DEBOUNCE_WINDOW: Duration = Duration::from_millis(50);

pub struct Debouncer {
    window: Duration,
}

impl Debouncer {
    pub fn new(window: Duration) -> Self {
        Self { window }
    }

    /// Coalesce file change events from `receiver` into batches.
    /// Waits for `window` of silence before emitting a batch.
    /// Returns the batch when ready, or None if the channel is closed.
    pub fn coalesce<T: Ord>(&self, receiver: &mpsc::Receiver<T>) -> Option<Vec<T>> {
        // Wait for the first event (blocking)
        let first = receiver.recv().ok()?;
        let mut batch = BTreeSet::new();
        batch.insert(first);

        // Drain any additional events within the debounce window
        loop {
            match receiver.recv_timeout(self.window) {
                Ok(path) => {
                    batch.insert(path);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        Some(batch.into_iter().collect())
    }
}
