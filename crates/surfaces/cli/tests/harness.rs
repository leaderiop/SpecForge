//! `specforge harness gc` (ADR 0287).

use crate::support::{Cli, TestProject};

#[specforge_test_macros::test(
    behavior = "collect_harness_ledger",
    verify = "harness gc leaves a ledger alone when no loaded extension declares a harness field"
)]
fn harness_gc_without_a_harness_field_keeps_every_line() {
    let dir = TestProject::new()
        .without_config()
        .files(&[(
            "specforge.json",
            r#"{"extensions": ["@specforge/software"]}"#,
        )])
        .file("a.spec", "invariant a \"A\" {}\n");
    let digest = "0".repeat(64);
    let messy = format!(
        "# specforge-harness 1\na result {digest}\na result {digest}\ngone result {digest}\n"
    );
    dir.write("specforge-harness", &messy);
    Cli::on(&dir).run(&["harness", "gc", "--dry-run"]).exits(0);
    Cli::on(&dir).run(&["harness", "gc"]).exits(0);
    assert_eq!(dir.read("specforge-harness"), messy);
}
