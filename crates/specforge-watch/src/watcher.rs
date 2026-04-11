use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

/// Watches a spec root directory for .spec file changes.
/// Sends batches of changed file paths through the provided sender.
pub struct SpecWatcher {
    _watcher: RecommendedWatcher,
}

impl SpecWatcher {
    /// Create a new watcher on the given directory.
    /// Changed .spec file paths are sent through `sender` as debounced batches.
    pub fn new(root: &Path, sender: mpsc::Sender<Vec<String>>) -> Result<Self, String> {
        let root_path = root.to_path_buf();

        let (notify_tx, notify_rx) = mpsc::channel::<notify::Result<Event>>();

        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = notify_tx.send(res);
            },
            Config::default(),
        )
        .map_err(|e| format!("failed to create file watcher: {}", e))?;

        // Spawn debounce thread
        std::thread::spawn(move || {
            Self::debounce_loop(notify_rx, sender, &root_path);
        });

        let mut w = watcher;
        w.watch(root, RecursiveMode::Recursive)
            .map_err(|e| format!("failed to watch directory: {}", e))?;

        Ok(Self { _watcher: w })
    }

    fn debounce_loop(
        rx: mpsc::Receiver<notify::Result<Event>>,
        sender: mpsc::Sender<Vec<String>>,
        root: &Path,
    ) {
        let debounce_window = Duration::from_millis(50);

        loop {
            // Wait for first event
            let first = match rx.recv() {
                Ok(Ok(event)) => event,
                Ok(Err(_)) => continue,
                Err(_) => return, // channel closed
            };

            let mut changed = Self::extract_spec_paths(&first, root);

            // Drain additional events within the debounce window
            loop {
                match rx.recv_timeout(debounce_window) {
                    Ok(Ok(event)) => {
                        changed.extend(Self::extract_spec_paths(&event, root));
                    }
                    Ok(Err(_)) => continue,
                    Err(mpsc::RecvTimeoutError::Timeout) => break,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }

            if !changed.is_empty() {
                // Deduplicate and sort
                changed.sort();
                changed.dedup();
                if sender.send(changed).is_err() {
                    return; // receiver dropped
                }
            }
        }
    }

    fn extract_spec_paths(event: &Event, root: &Path) -> Vec<String> {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                event
                    .paths
                    .iter()
                    .filter(|p| {
                        p.extension()
                            .is_some_and(|ext| ext == "spec")
                    })
                    .filter_map(|p| Self::relative_path(p, root))
                    .collect()
            }
            _ => vec![],
        }
    }

    fn relative_path(path: &Path, root: &Path) -> Option<String> {
        path.strip_prefix(root)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
            .or_else(|| Some(path.to_string_lossy().to_string()))
    }
}
