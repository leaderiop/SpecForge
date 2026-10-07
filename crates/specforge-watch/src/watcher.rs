use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use crate::debounce::Debouncer;

/// Watches one directory, recursively ([`Self::new`]) or only its own
/// entries ([`Self::shallow`]), and sends debounced batches of the
/// paths created, modified or removed under it: absolute, sorted, each
/// once per batch. What a path means to the project (a source, an
/// environment input, nothing) is the project session's to say
/// (`specforge_project::ProjectSession::changes`), so every surface shares
/// one meaning of "a change".
pub struct SpecWatcher {
    _watcher: RecommendedWatcher,
}

impl SpecWatcher {
    /// Watch `root` and send changed paths through `sender` as batches,
    /// each after `debounce_window` of quiet.
    pub fn new(
        root: &Path,
        sender: mpsc::Sender<Vec<PathBuf>>,
        debounce_window: Duration,
    ) -> Result<Self, String> {
        Self::watching(root, sender, debounce_window, RecursiveMode::Recursive)
    }

    /// [`Self::new`] for `dir`'s own entries only, not what is below them:
    /// the nearest existing ancestor of a directory that does not exist
    /// yet, which reports the creation of the next directory on the way.
    pub fn shallow(
        dir: &Path,
        sender: mpsc::Sender<Vec<PathBuf>>,
        debounce_window: Duration,
    ) -> Result<Self, String> {
        Self::watching(dir, sender, debounce_window, RecursiveMode::NonRecursive)
    }

    fn watching(
        root: &Path,
        sender: mpsc::Sender<Vec<PathBuf>>,
        debounce_window: Duration,
        mode: RecursiveMode,
    ) -> Result<Self, String> {
        // Canonicalize so notify's reported paths are compared with the
        // root as notify spells it, even when the caller passes a symlinked
        // path (macOS /var → /private/var, temp dirs). Without this, every
        // event would look out-of-root and be dropped by the C14-08
        // boundary.
        let root_path = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());

        let (notify_tx, notify_rx) = mpsc::channel::<notify::Result<Event>>();

        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = notify_tx.send(res);
            },
            Config::default(),
        )
        .map_err(|e| format!("failed to create file watcher: {}", e))?;

        // Map notify events to changed paths. The mapping thread owns
        // `root` so out-of-root paths are dropped at the boundary (C14-08).
        let (event_tx, event_rx) = mpsc::channel::<PathBuf>();
        let map_root = root_path.clone();
        std::thread::spawn(move || {
            for res in notify_rx {
                let Ok(event) = res else { continue };
                for path in Self::changed_paths(&event, &map_root) {
                    let _ = event_tx.send(path);
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
        w.watch(&root_path, mode)
            .map_err(|e| format!("failed to watch directory: {}", e))?;

        Ok(Self { _watcher: w })
    }

    /// The paths a creation, modification or removal touched under `root`.
    /// A path outside `root` (rename coalescing from another watched dir,
    /// an editor's atomic-save temp file elsewhere) is dropped (C14-08).
    fn changed_paths(event: &Event, root: &Path) -> Vec<PathBuf> {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => event
                .paths
                .iter()
                .filter(|path| path.starts_with(root))
                .cloned()
                .collect(),
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod watcher_unit_tests {
    use super::*;

    #[test]
    fn changed_paths_are_whole_and_out_of_root_paths_dropped() {
        let root = Path::new("/tmp/project");
        let event = Event::new(EventKind::Modify(notify::event::ModifyKind::Data(
            notify::event::DataChange::Any,
        )))
        .add_path(PathBuf::from("/tmp/project/spec/x.spec"))
        .add_path(PathBuf::from("/tmp/project/specforge.json"))
        .add_path(PathBuf::from("/tmp/project/target/x.wasm"))
        .add_path(PathBuf::from("/outside/y.spec"));
        assert_eq!(
            SpecWatcher::changed_paths(&event, root),
            vec![
                PathBuf::from("/tmp/project/spec/x.spec"),
                PathBuf::from("/tmp/project/specforge.json"),
                PathBuf::from("/tmp/project/target/x.wasm"),
            ],
            "out-of-root events must be dropped, not reported (C14-08)"
        );
    }

    #[test]
    fn access_events_change_nothing() {
        let event = Event::new(EventKind::Access(notify::event::AccessKind::Any))
            .add_path(PathBuf::from("/tmp/project/a.spec"));
        assert!(SpecWatcher::changed_paths(&event, Path::new("/tmp/project")).is_empty());
    }
}
