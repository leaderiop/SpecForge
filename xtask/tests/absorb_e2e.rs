//! `absorb` end to end: the old fixture project into a fresh `@gsd/core` project, which checks clean. Needs the built
//! component: `GSD_CORE_WASM=<gsd_core.wasm> cargo nextest run -p xtask --run-ignored all -E 'binary_id(xtask::absorb_e2e)'`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const OLD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/absorb/old");

fn gsd_core() -> PathBuf {
    PathBuf::from(std::env::var_os("GSD_CORE_WASM").expect(
        "GSD_CORE_WASM names gsd_core.wasm: (cd gsd && cargo build --locked -p gsd-core --release --target wasm32-wasip2)",
    ))
}

/// One run of the command: its exit code, its summary (stdout) and its stderr.
struct Run {
    code: i32,
    summary: Value,
    stderr: String,
}

fn absorb(args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_absorb"))
        .args(args)
        .env("GSD_CORE_WASM", gsd_core())
        .output()
        .expect("absorb runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    Run {
        code: out.status.code().unwrap_or(-1),
        summary: serde_json::from_str(stdout.trim()).unwrap_or(Value::Null),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn path(p: &Path) -> &str {
    p.to_str().expect("a UTF-8 path")
}

/// Every file under `root/spec`, by its path from `root`.
fn spec_tree(root: &Path) -> BTreeMap<String, String> {
    walkdir::WalkDir::new(root.join("spec"))
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let key = e.path().strip_prefix(root).unwrap().display().to_string();
            (key, std::fs::read_to_string(e.path()).unwrap())
        })
        .collect()
}

#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn all_thirteen_kinds_convert_with_no_error_and_no_warning() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a"), dir.path().join("b"));
    let run = absorb(&["--from", OLD, "--to", path(&a)]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.summary["target"]["errors"], 0, "{}", run.summary);
    assert_eq!(run.summary["target"]["warnings"], 0, "{}", run.summary);
    let constraints = std::fs::read_to_string(a.join("spec/governance/constraints.spec")).unwrap();
    assert!(
        constraints.contains("threshold 95.0\n"),
        "a threshold keeps its written text: {constraints}"
    );
    assert_eq!(
        run.summary["work"],
        json!({"compiles": 3, "edit_plans": 1}),
        "three compiles and one edit plan, whatever the size"
    );
    assert_eq!(
        (&run.summary["converted"], &run.summary["created"]),
        (&json!(28), &json!(1)),
        "every entity but the module and the release, and one feature's solution: {}",
        run.summary
    );
    let journeys = std::fs::read_to_string(a.join("spec/product/journeys.spec")).unwrap();
    assert!(
        journeys.contains("milestones [store_cli, store_web]"),
        "a journey lists every deliverable that listed it: {journeys}"
    );
    let features = std::fs::read_to_string(a.join("spec/product/features.spec")).unwrap();
    assert!(
        features.contains("decision store_entries_solution \"Store Entries\""),
        "{features}"
    );
    let ledger = std::fs::read_to_string(a.join("absorb-ledger.json")).unwrap();
    assert!(
        ledger.contains("\"id\": \"old_format\""),
        "a superseded decision that names no successor is ledgered: {ledger}"
    );

    let again = absorb(&["--from", OLD, "--to", path(&b)]);
    assert_eq!(again.code, 0, "{}", again.stderr);
    assert_eq!(
        spec_tree(&a),
        spec_tree(&b),
        "two runs write the same files"
    );
    assert_eq!(
        ledger,
        std::fs::read_to_string(b.join("absorb-ledger.json")).unwrap()
    );
    insta::assert_snapshot!("ledger", ledger);
}
