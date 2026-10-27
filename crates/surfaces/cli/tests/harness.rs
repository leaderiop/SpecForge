//! `specforge harness gc` (ADR 0287).

use crate::support::{Cli, TestProject};

#[specforge_test_macros::test(
    behavior = "collect_harness_ledger",
    verify = "harness gc rewrites the ledger canonically and --dry-run writes nothing"
)]
#[ignore = "K4 T14"]
fn harness_gc_rewrites_the_ledger_canonically() {
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

    let dry = Cli::on(&dir).run(&["harness", "gc", "--dry-run"]).exits(0);
    assert!(dry.stdout.contains("specforge-harness"), "{}", dry.stdout);
    assert_eq!(
        dir.read("specforge-harness"),
        messy,
        "--dry-run writes nothing"
    );

    Cli::on(&dir).run(&["harness", "gc"]).exits(0);
    // `result` is no harness field of `invariant`, so every line goes: the canonical ledger is its header.
    assert_eq!(dir.read("specforge-harness"), "# specforge-harness 1\n");
}
