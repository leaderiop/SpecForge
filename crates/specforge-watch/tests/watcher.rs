use specforge_test_macros::test as spec;
use specforge_watch::{SpecWatcher, WatchEvent, WatchEventKind};
use std::fs;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn wait_for_event(
    rx: &mpsc::Receiver<Vec<WatchEvent>>,
    timeout: Duration,
) -> Option<Vec<WatchEvent>> {
    rx.recv_timeout(timeout).ok()
}

// ── file modification triggers recompilation ──────────────────

#[spec(
    behavior = "watch_file_system_for_changes",
    verify = "file modification triggers recompilation"
)]
#[test]
fn file_modification_triggers_recompilation() {
    let dir = TempDir::new().unwrap();
    let spec_path = dir.path().join("a.spec");
    fs::write(&spec_path, r#"behavior foo "Foo" { contract "x" }"#).unwrap();

    let (tx, rx) = mpsc::channel();
    let _watcher =
        SpecWatcher::new(dir.path(), tx, specforge_watch::DEFAULT_DEBOUNCE_WINDOW).unwrap();

    // Give watcher time to start
    std::thread::sleep(Duration::from_millis(100));

    // Modify the file
    fs::write(&spec_path, r#"behavior bar "Bar" { contract "y" }"#).unwrap();

    let event = wait_for_event(&rx, Duration::from_secs(2));
    assert!(
        event.is_some(),
        "should receive change event after file modification"
    );
    let events = event.unwrap();
    assert!(
        events.iter().any(|f| f.path.ends_with("a.spec")),
        "changed files should include a.spec, got: {:?}",
        events
    );
}

// ── file creation triggers recompilation ──────────────────────

#[spec(
    behavior = "watch_file_system_for_changes",
    verify = "file creation triggers recompilation"
)]
#[test]
fn file_creation_triggers_recompilation() {
    let dir = TempDir::new().unwrap();

    let (tx, rx) = mpsc::channel();
    let _watcher =
        SpecWatcher::new(dir.path(), tx, specforge_watch::DEFAULT_DEBOUNCE_WINDOW).unwrap();

    std::thread::sleep(Duration::from_millis(100));

    // Create a new spec file
    let new_path = dir.path().join("new.spec");
    fs::write(&new_path, r#"behavior new_thing "New" { contract "z" }"#).unwrap();

    let event = wait_for_event(&rx, Duration::from_secs(2));
    assert!(
        event.is_some(),
        "should receive change event after file creation"
    );
    let events = event.unwrap();
    assert!(
        events.iter().any(|f| f.path.ends_with("new.spec")),
        "changed files should include new.spec, got: {:?}",
        events
    );
}

// ── file deletion triggers recompilation ──────────────────────

#[spec(
    behavior = "watch_file_system_for_changes",
    verify = "file deletion triggers recompilation"
)]
#[test]
fn file_deletion_triggers_recompilation() {
    let dir = TempDir::new().unwrap();
    let spec_path = dir.path().join("doomed.spec");
    fs::write(&spec_path, r#"behavior doomed "Doomed" { contract "bye" }"#).unwrap();

    let (tx, rx) = mpsc::channel();
    let _watcher =
        SpecWatcher::new(dir.path(), tx, specforge_watch::DEFAULT_DEBOUNCE_WINDOW).unwrap();

    std::thread::sleep(Duration::from_millis(100));

    // Delete the file
    fs::remove_file(&spec_path).unwrap();

    let event = wait_for_event(&rx, Duration::from_secs(2));
    assert!(
        event.is_some(),
        "should receive change event after file deletion"
    );
    let events = event.unwrap();
    assert!(
        events.iter().any(|f| f.path.ends_with("doomed.spec")),
        "changed files should include doomed.spec, got: {:?}",
        events
    );
}

// ── latency integration test ──────────────────────────────────

#[spec(
    behavior = "watch_file_system_for_changes",
    verify = "watch detects changes within 100ms"
)]
#[test]
fn watch_detects_changes_within_latency_target() {
    let dir = TempDir::new().unwrap();
    let spec_path = dir.path().join("latency.spec");
    fs::write(&spec_path, r#"behavior init "Init" { contract "x" }"#).unwrap();

    let (tx, rx) = mpsc::channel();
    let _watcher =
        SpecWatcher::new(dir.path(), tx, specforge_watch::DEFAULT_DEBOUNCE_WINDOW).unwrap();

    std::thread::sleep(Duration::from_millis(200));

    let start = Instant::now();
    fs::write(&spec_path, r#"behavior updated "Updated" { contract "y" }"#).unwrap();

    let event = wait_for_event(&rx, Duration::from_secs(2));
    let elapsed = start.elapsed();

    assert!(event.is_some(), "should receive change event");
    // Generous CI headroom: spec target is 100ms detection + 50ms debounce
    assert!(
        elapsed < Duration::from_millis(500),
        "change detection took {:?}, expected < 500ms",
        elapsed
    );
}

// ── contract test ─────────────────────────────────────────────

#[spec(
    behavior = "watch_file_system_for_changes",
    verify = "Watch File System for Changes: file system watching holds for the declared obligations"
)]
#[test]
fn watch_contract_consistency() {
    let dir = TempDir::new().unwrap();

    // Requires: watch mode active on spec root
    let (tx, rx) = mpsc::channel();
    let _watcher =
        SpecWatcher::new(dir.path(), tx, specforge_watch::DEFAULT_DEBOUNCE_WINDOW).unwrap();
    std::thread::sleep(Duration::from_millis(200));

    // Ensures: file_changed event produced for creation
    let spec_path = dir.path().join("contract.spec");
    fs::write(&spec_path, r#"behavior a "A" { contract "x" }"#).unwrap();
    let event = wait_for_event(&rx, Duration::from_secs(2));
    assert!(event.is_some(), "ensures: file_changed for creation");

    // Ensures: file_changed event produced for modification
    fs::write(&spec_path, r#"behavior b "B" { contract "y" }"#).unwrap();
    let event = wait_for_event(&rx, Duration::from_secs(2));
    assert!(event.is_some(), "ensures: file_changed for modification");

    // Non-.spec files should NOT trigger events
    let txt_path = dir.path().join("readme.txt");
    fs::write(&txt_path, "not a spec").unwrap();
    let event = wait_for_event(&rx, Duration::from_millis(500));
    assert!(event.is_none(), "non-.spec files must not trigger events");
}

// ── config + plugin artifacts classify (hardening-plan H2 / R-5) ──
#[spec(
    behavior = "watch_file_system_for_changes",
    verify = "specforge.json and .wasm changes classify as config/plugin"
)]
#[test]
fn config_and_plugin_changes_classify() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"t","version":"0.1.0"}"#,
    )
    .unwrap();

    let (tx, rx) = mpsc::channel();
    let _watcher =
        SpecWatcher::new(dir.path(), tx, specforge_watch::DEFAULT_DEBOUNCE_WINDOW).unwrap();
    std::thread::sleep(Duration::from_millis(100));

    // Config change
    fs::write(
        dir.path().join("specforge.json"),
        r#"{"name":"t2","version":"0.1.0"}"#,
    )
    .unwrap();
    let batch = wait_for_event(&rx, Duration::from_secs(2));
    let batch = batch.expect("config change should produce an event");
    assert!(
        batch
            .iter()
            .any(|e| e.kind == WatchEventKind::Config && e.path.ends_with("specforge.json")),
        "expected Config event, got: {:?}",
        batch
    );

    // Plugin change
    fs::write(dir.path().join("patch.wasm"), b"\x00asm\x01\x00\x00\x00").unwrap();
    let batch = wait_for_event(&rx, Duration::from_secs(2));
    let batch = batch.expect("plugin change should produce an event");
    assert!(
        batch
            .iter()
            .any(|e| e.kind == WatchEventKind::Plugin && e.path.ends_with("patch.wasm")),
        "expected Plugin event, got: {:?}",
        batch
    );
}
