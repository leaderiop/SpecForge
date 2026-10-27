//! `format` writes under the project's write lock (ADR 0289), so a concurrent write is never overwritten.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use specforge_ops::authoring::format::{self, Forms, Mode, Request};
use specforge_test_macros::test as specforge_test;

use crate::writes::project;

#[ignore = "K5 T14"]
#[specforge_test(
    behavior = "apply_edit_plan",
    verify = "format does not overwrite a concurrent write"
)]
fn format_does_not_overwrite_a_concurrent_write() {
    let messy = "behavior messy \"Messy\" {\ncontract \"The system MUST work\"\n}\n";
    let dir = project(&[], &[("a.spec", messy)]);
    let root = specforge_common::ProjectRoot::of(dir.path()).unwrap();
    // Another writer holds the lock and changes the file; format waits for it instead of replacing the file.
    let lock = specforge_project::harness::lock(dir.path()).unwrap();
    let done = AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            format::run(&Request {
                root: &root,
                paths: &[],
                mode: Mode::Write,
                caller: specforge_ops::testing::caller(),
                forms: Forms::none(),
            })
            .unwrap();
            done.store(true, Ordering::SeqCst);
        });
        std::thread::sleep(Duration::from_millis(400));
        assert!(
            !done.load(Ordering::SeqCst),
            "format wrote while another writer held the lock"
        );
        std::fs::write(
            dir.path().join("a.spec"),
            format!("{messy}behavior added \"Added\" {{\n}}\n"),
        )
        .unwrap();
        drop(lock);
    });
    let text = std::fs::read_to_string(dir.path().join("a.spec")).unwrap();
    assert!(text.contains("behavior added"), "{text}");
}
