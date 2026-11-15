//! `absorb` end to end: the old fixture project into a fresh `@gsd/core` project, which checks clean. Needs the built
//! component: `GSD_CORE_WASM=<gsd_core.wasm> cargo nextest run -p xtask --run-ignored all -E 'binary_id(xtask::absorb_e2e)'`;
//! the proof test also needs `SPECFORGE_BIN`, the built `specforge` binary that collects its fixture.
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
const PROOF: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/absorb/proof");
const SELECTION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/absorb/selection.json"
);

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

#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn a_selection_converts_only_its_entities_and_checks_clean() {
    let dir = tempfile::tempdir().unwrap();
    let to = dir.path().join("t");
    let only = format!("ids:{SELECTION}");
    let run = absorb(&["--from", OLD, "--to", path(&to), "--only", &only]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.summary["target"]["errors"], 0, "{}", run.summary);
    assert_eq!(run.summary["target"]["warnings"], 0, "{}", run.summary);
    assert_eq!(
        (&run.summary["converted"], &run.summary["created"]),
        (&json!(9), &json!(1)),
        "the nine selected ids and the selected feature's solution: {}",
        run.summary
    );
    let tree = spec_tree(&to);
    assert!(
        !tree.values().any(|text| text.contains("second_store")),
        "a reference leaving the selection is never stubbed"
    );
    let ledger = std::fs::read_to_string(to.join("absorb-ledger.json")).unwrap();
    assert!(
        ledger.contains("\"out_of_selection\": \"second_store\""),
        "{ledger}"
    );
    insta::assert_snapshot!("selection_ledger", ledger);
}

/// `from`'s files copied under `to`.
fn copy_tree(from: &Path, to: &Path) {
    for entry in walkdir::WalkDir::new(from)
        .into_iter()
        .filter_map(Result::ok)
    {
        let target = to.join(entry.path().strip_prefix(from).unwrap());
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target).unwrap();
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

#[test]
#[ignore = "needs GSD_CORE_WASM and SPECFORGE_BIN"]
fn invariant_obligations_become_proofs_of_the_collected_items() {
    let specforge = std::env::var_os("SPECFORGE_BIN").expect(
        "SPECFORGE_BIN names the built specforge: cargo build -p specforge-cli --bin specforge",
    );
    let dir = tempfile::tempdir().unwrap();
    let (source, home, to) = (
        dir.path().join("source"),
        dir.path().join("home"),
        dir.path().join("target"),
    );
    copy_tree(Path::new(PROOF), &source);
    std::fs::create_dir(&home).unwrap();
    let collect = Command::new(specforge)
        .arg("collect")
        .current_dir(&source)
        .env("HOME", &home)
        .output()
        .expect("specforge collect runs");
    assert!(
        source.join(".specforge/facts").is_dir(),
        "the source's items are recorded: {}",
        String::from_utf8_lossy(&collect.stderr)
    );

    let run = absorb(&[
        "--from",
        path(&source),
        "--to",
        path(&to),
        "--extension",
        "@rust/cargo",
        "--extension",
        "@rust/source",
    ]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let cited = [
        "rust:absorb-proof#tests::a_killed_writer_leaves_the_old_entry",
        "rust:absorb-proof#tests::an_acknowledged_write_is_on_disk",
    ];
    assert_eq!(run.summary["cited_tests"], json!(cited), "{}", run.summary);
    let invariants = std::fs::read_to_string(to.join("spec/invariants.spec")).unwrap();
    for test in cited {
        assert!(invariants.contains(&format!("\"{test}\"")), "{invariants}");
    }
    assert_eq!(run.summary["target"]["errors"], 0, "{}", run.summary);
    let findings = run.summary["target"]["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 2, "{}", run.summary);
    assert!(
        findings
            .iter()
            .all(|f| f.as_str().unwrap().starts_with("code/W102 ")),
        "until the target collects its test facts, a written proof is code/W102, and nothing else warns: {}",
        run.summary
    );
    let ledger = std::fs::read_to_string(to.join("absorb-ledger.json")).unwrap();
    assert_eq!(run.summary["ledger"], json!({"unproven": 1}), "{ledger}");
    insta::assert_snapshot!("proof_ledger", ledger);
}

#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn an_old_id_the_target_reserves_stops_the_run_before_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("new"));
    std::fs::create_dir_all(old.join("spec")).unwrap();
    std::fs::write(
        old.join("specforge.json"),
        r#"{"name": "reserved", "spec_root": "spec", "extensions": ["@specforge/product"]}"#,
    )
    .unwrap();
    std::fs::write(
        old.join("spec/terms.spec"),
        "term blocker \"Blocker\" {\n  definition \"What stops work.\"\n}\n",
    )
    .unwrap();
    let run = absorb(&["--from", path(&old), "--to", path(&to)]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr.contains("1 id(s) the target refuses")
            && run
                .stderr
                .contains("term 'blocker': invalid entity ID 'blocker'"),
        "{}",
        run.stderr
    );
    assert!(!to.exists(), "the scaffold is removed");
}
