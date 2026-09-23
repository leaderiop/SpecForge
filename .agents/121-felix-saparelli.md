# 121 — Félix Saparelli

**Cluster:** C14 — Direct-dependency maintainers (surface infrastructure)
**Roster role:** notify maintainer; cross-platform file-watching
**SpecForge anchors:** crates/specforge-watch/src/watcher.rs (notify event source), debounce.rs, dispatch.rs (incremental DAG invalidation)

## Why this engineer
Saparelli (passcod) maintained notify for years through its defining rework — the modern watcher API with `Config`, recursive modes, and typed event kinds — and his cargo-watch/watchexec tooling is the longest-running consumer of that API in the wild. specforge-watch's watcher.rs consumes exactly the event stream he shaped; his crates document the platform quirks (inotify vs FSEvents vs ReadDirectoryChangesW, poll fallbacks, duplicate/rename events) that debounce.rs must survive for correct incremental invalidation.

## References for SpecForge
**Key works**
- [notify-rs/notify](https://github.com/notify-rs/notify) — GitHub, 2016. The file-event crate under specforge-watch (workspace pins 8.2); platform-backend matrix documented here.
- [watchexec/cargo-watch](https://github.com/watchexec/cargo-watch) — GitHub, 2016. His own consumer of notify: debounce-then-run UX and event-duplication workarounds precedent for watch's dispatch loop.
- [passcod.name](https://passcod.name) — personal site. Notes across the watchexec/notify family and file-watching on each OS.

## Study first
1. Raw vs debounced event semantics per backend (what debounce.rs can assume)
2. Rename/move event pairing — the failure mode for import_dag.rs invalidation
3. Poll-fallback behavior on network drives and editor atomic saves
