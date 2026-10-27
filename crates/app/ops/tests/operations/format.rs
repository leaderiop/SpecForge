//! `format` writes under the project's write lock when the project's writes are serialized (ADR 0289), so a concurrent
//! write is never overwritten; any other project formats exactly as before, with no `.specforge/` needed.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use specforge_ops::authoring::format::{self, Forms, Mode, Request};
use specforge_project::testing::GsdWrites;
use specforge_project::{RuntimeSource, SharedRuntime};
use specforge_test_macros::test as specforge_test;

use crate::plans::project as gsd_project;
use crate::writes::project;

const MESSY: &str = "behavior messy \"Messy\" {\ncontract \"The system MUST work\"\n}\n";

/// Write-format the sources of the project at `dir`; `served` is the fixture's extensions, loaded as the CLI does.
fn format_all(dir: &std::path::Path, served: Option<SharedRuntime>) {
    let root = specforge_common::ProjectRoot::of(dir).unwrap();
    format::run(&Request {
        root: &root,
        paths: &[],
        mode: Mode::Write,
        caller: specforge_ops::testing::caller(),
        forms: match served {
            Some(runtime) => Forms::loading(RuntimeSource::Fixed(Some(runtime))),
            None => Forms::none(),
        },
    })
    .unwrap();
}

/// [`format_all`] over a project of the writing fixture.
fn format_served(project: &specforge_project::testing::TestProject) {
    let runtime: SharedRuntime = Arc::new(GsdWrites::new().runtime());
    // The lock file the host would have written when the extensions were installed.
    project.environment_in(Arc::clone(&runtime));
    format_all(project.root(), Some(runtime));
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "format does not overwrite a concurrent write"
)]
fn format_does_not_overwrite_a_concurrent_write() {
    let dir = gsd_project().file("a.spec", MESSY);
    // Another writer holds the lock and changes the file; format waits for it instead of replacing the file.
    let lock = specforge_project::harness::lock(dir.root()).unwrap();
    let done = AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            format_served(&dir);
            done.store(true, Ordering::SeqCst);
        });
        std::thread::sleep(Duration::from_millis(400));
        assert!(
            !done.load(Ordering::SeqCst),
            "format wrote while another writer held the lock"
        );
        std::fs::write(
            dir.root().join("a.spec"),
            format!("{MESSY}behavior added \"Added\" {{\n}}\n"),
        )
        .unwrap();
        drop(lock);
    });
    let text = dir.read("a.spec");
    assert!(text.contains("behavior added"), "{text}");
}

#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "format locks a project with a writing extension and not one that merely has a .specforge directory"
)]
fn format_locks_only_a_project_whose_writes_are_serialized() {
    // A project with a writing command locks, and makes its `.specforge/` to do it.
    let serialized = gsd_project().file("a.spec", MESSY);
    format_served(&serialized);
    assert!(serialized.root().join(".specforge/write.lock").exists());
    // A plain project with a read-only `.specforge/` (a restored cache) still formats.
    use std::os::unix::fs::PermissionsExt;
    let dir = project(&[], &[("a.spec", MESSY)]);
    let cache = dir.path().join(".specforge");
    std::fs::create_dir(&cache).unwrap();
    std::fs::set_permissions(&cache, std::fs::Permissions::from_mode(0o555)).unwrap();
    format_all(dir.path(), None);
    std::fs::set_permissions(&cache, std::fs::Permissions::from_mode(0o755)).unwrap();
    let text = std::fs::read_to_string(dir.path().join("a.spec")).unwrap();
    assert_ne!(text, MESSY, "formatted");
    assert!(!cache.join("write.lock").exists());
}
