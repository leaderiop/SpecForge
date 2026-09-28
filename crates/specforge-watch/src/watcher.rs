use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use crate::debounce::Debouncer;

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
    pub fn new(
        root: &Path,
        sender: mpsc::Sender<Vec<WatchEvent>>,
        debounce_window: Duration,
    ) -> Result<Self, String> {
        Self::new_filtered(
            root,
            sender,
            &[
                WatchEventKind::Spec,
                WatchEventKind::Config,
                WatchEventKind::Plugin,
            ],
            debounce_window,
        )
    }

    /// Like [`Self::new`], but restricted to the given event kinds. Lets a
    /// host watch the project root for config/plugin artifacts while keeping
    /// spec-relative paths flowing through a spec-root watcher.
    pub fn new_filtered(
        root: &Path,
        sender: mpsc::Sender<Vec<WatchEvent>>,
        allowed: &[WatchEventKind],
        debounce_window: Duration,
    ) -> Result<Self, String> {
        // Canonicalize so notify's reported paths strip_prefix cleanly even
        // when the caller passes a symlinked path (macOS /var → /private/var,
        // temp dirs). Without this, every event would look out-of-root and
        // be dropped by the C14-08 boundary.
        let root_path = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        let allowed: std::vec::Vec<WatchEventKind> = allowed.to_vec();

        let (notify_tx, notify_rx) = mpsc::channel::<notify::Result<Event>>();

        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = notify_tx.send(res);
            },
            Config::default(),
        )
        .map_err(|e| format!("failed to create file watcher: {}", e))?;

        // Map notify events to classified, root-relative WatchEvents. The
        // mapping thread owns `root` so out-of-root paths are dropped at the
        // boundary (C14-08) and batches stay uniform.
        let (event_tx, event_rx) = mpsc::channel::<WatchEvent>();
        let map_root = root_path.clone();
        std::thread::spawn(move || {
            for res in notify_rx {
                let Ok(event) = res else { continue };
                for ev in Self::classify_events(&event, &map_root) {
                    if allowed.contains(&ev.kind) {
                        let _ = event_tx.send(ev);
                    }
                }
            }
        });

        // Coalescing delegates to the tested Debouncer (C14-09) instead of a
        // hand-rolled duplicate; the window is a constructor argument.
        std::thread::spawn(move || {
            let debouncer = Debouncer::new(debounce_window);
            while let Some(batch) = debouncer.coalesce(&event_rx) {
                if sender.send(batch).is_err() {
                    return; // receiver dropped
                }
            }
        });

        let mut w = watcher;
        w.watch(&root_path, RecursiveMode::Recursive)
            .map_err(|e| format!("failed to watch directory: {}", e))?;

        Ok(Self { _watcher: w })
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

    /// Root-relative path for a notify event path. Out-of-root paths
    /// (rename coalescing from another watched dir, editor atomic-save temp
    /// files) cannot be expressed root-relative and would poison the
    /// downstream invalidation keying — they are dropped instead of leaking
    /// absolute paths into a batch (C14-08).
    fn relative_path(path: &Path, root: &Path) -> Option<String> {
        path.strip_prefix(root)
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod watcher_unit_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn out_of_root_paths_are_dropped_not_absolutized() {
        let root = Path::new("/tmp/project/spec");
        assert_eq!(
            SpecWatcher::relative_path(&PathBuf::from("/tmp/project/spec/a.spec"), root),
            Some("a.spec".to_string())
        );
        assert_eq!(
            SpecWatcher::relative_path(&PathBuf::from("/elsewhere/b.spec"), root),
            None,
            "out-of-root events must be dropped, not absolutized (C14-08)"
        );
    }

    #[test]
    fn classify_maps_extensions_and_drops_unrelated_files() {
        // Config and plugin artifacts live in the project root; the
        // spec-root watcher (used in this classify call) drops them as
        // out-of-root, while the project-root watcher resolves them.
        let root = Path::new("/tmp/project");
        let event = Event::new(EventKind::Modify(notify::event::ModifyKind::Data(
            notify::event::DataChange::Any,
        )))
        .add_path(PathBuf::from("/tmp/project/spec/x.spec"))
        .add_path(PathBuf::from("/tmp/project/specforge.json"))
        .add_path(PathBuf::from("/tmp/project/plugin.wasm"))
        .add_path(PathBuf::from("/tmp/project/notes.txt"))
        .add_path(PathBuf::from("/outside/y.spec"));
        let events = SpecWatcher::classify_events(&event, root);
        let kinds: Vec<(String, WatchEventKind)> =
            events.into_iter().map(|e| (e.path, e.kind)).collect();
        assert_eq!(
            kinds,
            vec![
                ("spec/x.spec".to_string(), WatchEventKind::Spec),
                ("specforge.json".to_string(), WatchEventKind::Config),
                ("plugin.wasm".to_string(), WatchEventKind::Plugin),
            ],
            "unrelated files and out-of-root paths must not surface"
        );

        // A spec-root watcher drops the project-root artifacts entirely.
        let spec_root = Path::new("/tmp/project/spec");
        let events = SpecWatcher::classify_events(&event, spec_root);
        let kinds: Vec<(String, WatchEventKind)> =
            events.into_iter().map(|e| (e.path, e.kind)).collect();
        assert_eq!(
            kinds,
            vec![("x.spec".to_string(), WatchEventKind::Spec)],
            "out-of-root config/plugin artifacts must not leak absolute paths"
        );
    }
}
