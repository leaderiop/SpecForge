use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

/// Classification of a watched-file change (hardening-plan H2 / R-5).
///
/// Spec changes drive incremental rebuilds; config and plugin changes drive
/// a full extension re-describe + graph re-seed. Both kinds coalesce through
/// the same debounce window.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum WatchEventKind {
    Spec,
    Config,
    Plugin,
}

/// One classified change: relative path plus what kind of artifact changed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct WatchEvent {
    pub path: String,
    pub kind: WatchEventKind,
}

/// Watches a spec root directory for .spec, specforge.json, and .wasm
/// changes. Sends debounced batches of classified events through the sender.
pub struct SpecWatcher {
    _watcher: RecommendedWatcher,
}

impl SpecWatcher {
    /// Create a new watcher on the given directory.
    /// Classified change events are sent through `sender` as debounced batches.
    pub fn new(root: &Path, sender: mpsc::Sender<Vec<WatchEvent>>) -> Result<Self, String> {
        Self::new_filtered(
            root,
            sender,
            &[
                WatchEventKind::Spec,
                WatchEventKind::Config,
                WatchEventKind::Plugin,
            ],
        )
    }

    /// Like [`Self::new`], but restricted to the given event kinds. Lets a
    /// host watch the project root for config/plugin artifacts while keeping
    /// spec-relative paths flowing through a spec-root watcher.
    pub fn new_filtered(
        root: &Path,
        sender: mpsc::Sender<Vec<WatchEvent>>,
        allowed: &[WatchEventKind],
    ) -> Result<Self, String> {
        let root_path = root.to_path_buf();
        let allowed: std::vec::Vec<WatchEventKind> = allowed.to_vec();

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
            Self::debounce_loop(notify_rx, sender, &root_path, allowed);
        });

        let mut w = watcher;
        w.watch(root, RecursiveMode::Recursive)
            .map_err(|e| format!("failed to watch directory: {}", e))?;

        Ok(Self { _watcher: w })
    }

    fn debounce_loop(
        rx: mpsc::Receiver<notify::Result<Event>>,
        sender: mpsc::Sender<Vec<WatchEvent>>,
        root: &Path,
        allowed: std::vec::Vec<WatchEventKind>,
    ) {
        let debounce_window = Duration::from_millis(50);

        loop {
            // Wait for first event
            let first = match rx.recv() {
                Ok(Ok(event)) => event,
                Ok(Err(_)) => continue,
                Err(_) => return, // channel closed
            };

            let mut changed: std::vec::Vec<WatchEvent> = Self::classify_events(&first, root)
                .into_iter()
                .filter(|e| allowed.contains(&e.kind))
                .collect();

            // Drain additional events within the debounce window
            loop {
                match rx.recv_timeout(debounce_window) {
                    Ok(Ok(event)) => {
                        changed.extend(
                            Self::classify_events(&event, root)
                                .into_iter()
                                .filter(|e| allowed.contains(&e.kind)),
                        );
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

    fn classify_events(event: &Event, root: &Path) -> Vec<WatchEvent> {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => event
                .paths
                .iter()
                .filter_map(|p| {
                    let kind = if p.extension().is_some_and(|ext| ext == "spec") {
                        WatchEventKind::Spec
                    } else if p.file_name().is_some_and(|n| n == "specforge.json") {
                        WatchEventKind::Config
                    } else if p.extension().is_some_and(|ext| ext == "wasm") {
                        WatchEventKind::Plugin
                    } else {
                        return None;
                    };
                    Some(WatchEvent {
                        path: Self::relative_path(p, root)?,
                        kind,
                    })
                })
                .collect(),
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
