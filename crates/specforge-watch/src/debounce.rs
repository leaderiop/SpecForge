use std::collections::BTreeSet;
use std::sync::mpsc;
use std::time::Duration;

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
    pub fn coalesce(&self, receiver: &mpsc::Receiver<String>) -> Option<Vec<String>> {
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
