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
    assert!(
        journeys
            .lines()
            .any(|l| l.split_whitespace().eq(["milestones", "[store_cli]"])),
        "the journey with features lists the milestone its deliverable became: {journeys}"
    );
    assert!(
        !run.summary["target"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f.as_str().unwrap().starts_with("gsd/W530 ")),
        "a milestone with no phase asks no journey for its requirements (gsd/W530): {}",
        run.summary
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
    assert!(
        left_out(&ledger, "record_an_entry", "milestones", "store_cli"),
        "the deliverable left out took its milestone off the selected journey: {ledger}"
    );
    insta::assert_snapshot!("selection_ledger", ledger);
}

/// Whether `ledger` holds the row for the item `left` that the selected `id` loses from `field` because `left` is out of
/// the selection.
fn left_out(ledger: &str, id: &str, field: &str, left: &str) -> bool {
    let ledger: serde_json::Value = serde_json::from_str(ledger).unwrap();
    ledger["entries"].as_array().unwrap().iter().any(|e| {
        e["id"] == id
            && e["field"] == field
            && e["value"] == left
            && e["why"]["out_of_selection"] == left
    })
}

/// An inverse write from an entity the selection leaves out is a value the selected entity loses: the decision that
/// `first_store` says supersedes it converts with no `supersedes`, and the ledger says why, naming `first_store`.
#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn a_selected_entity_losing_an_inverse_write_to_the_selection_is_ledgered() {
    let dir = tempfile::tempdir().unwrap();
    let (only, to) = (dir.path().join("only.json"), dir.path().join("t"));
    std::fs::write(&only, r#"["second_store"]"#).unwrap();
    let run = absorb(&[
        "--from",
        OLD,
        "--to",
        path(&to),
        "--only",
        &format!("ids:{}", path(&only)),
    ]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.summary["target"]["errors"], 0, "{}", run.summary);
    assert!(
        !spec_tree(&to)
            .values()
            .any(|text| text.contains("first_store")),
        "never stubbed"
    );
    let ledger = std::fs::read_to_string(to.join("absorb-ledger.json")).unwrap();
    assert!(
        left_out(&ledger, "second_store", "supersedes", "first_store"),
        "{ledger}"
    );
}

/// A selection that leaves out what a selected entity requires (a journey without its persona) stops before any write,
/// naming the selected entity, the field and the ids to add to it, not the old project; the ledger is written, so
/// every reference the selection left out is listed.
#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn a_selection_leaving_out_a_required_reference_stops_naming_the_selection() {
    let dir = tempfile::tempdir().unwrap();
    let (only, to) = (dir.path().join("only.json"), dir.path().join("t"));
    std::fs::write(&only, r#"["record_an_entry", "store_entries"]"#).unwrap();
    let run = absorb(&[
        "--from",
        OLD,
        "--to",
        path(&to),
        "--only",
        &format!("ids:{}", path(&only)),
    ]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr.contains(
            "1 required value(s) the selection leaves out; add the ids to --only (the ledger lists each as \
             out_of_selection):\n  journey 'record_an_entry' persona needs 'writer', which --only leaves out"
        ),
        "{}",
        run.stderr
    );
    assert!(
        !run.stderr.contains("the old project lacks"),
        "{}",
        run.stderr
    );
    let ledger = std::fs::read_to_string(to.join("absorb-ledger.json")).unwrap();
    assert!(
        ledger.contains("\"out_of_selection\": \"writer\""),
        "{ledger}"
    );
    assert!(
        !to.join("specforge.json").exists(),
        "the scaffold is removed"
    );
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
    assert_eq!(
        run.summary["ledger"],
        json!({"collapsed": 2, "unconverted": 5, "unproven": 1}),
        "the two proved obligations' kind and text, the five old extensions the target does not load, and the \
         obligation no test item states: {ledger}"
    );
    insta::assert_snapshot!("proof_ledger", ledger);
}

/// A governance source at `old` holding one decision in `<file>`, its `specforge.json` the members `config` (a JSON
/// object's inside) beside its name and extension.
fn decision_source(old: &Path, config: &str, file: &str) {
    std::fs::create_dir_all(old.join(file).parent().unwrap()).unwrap();
    std::fs::write(
        old.join("specforge.json"),
        format!(r#"{{"name": "carried", {config} "extensions": ["@specforge/governance"]}}"#),
    )
    .unwrap();
    std::fs::write(
        old.join(file),
        "decision one_parser \"One Parser\" {\n  status   accepted\n  context  \"Readers drift.\"\n  \
         decision \"Read with one parser.\"\n}\n",
    )
    .unwrap();
}

/// The target keeps the source's spec root and version: a nested one, and none (the sources at the project's root,
/// converted into a tree that already holds other files). What the source's project block holds beyond them is
/// ledgered: its extensions, and the spec block's own version and extensions.
#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn the_target_keeps_the_source_spec_root_and_version() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("new"));
    decision_source(
        &old,
        r#""version": "2.3.0", "spec_root": "specs/main","#,
        "specs/main/governance/decisions.spec",
    );
    std::fs::write(
        old.join("specs/main/carried.spec"),
        "spec \"carried\" {\n  version    \"9.9\"\n  extensions [\"@specforge/governance\"]\n}\n",
    )
    .unwrap();
    let run = absorb(&["--from", path(&old), "--to", path(&to)]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.summary["target"]["errors"], 0, "{}", run.summary);
    let config: Value =
        serde_json::from_str(&std::fs::read_to_string(to.join("specforge.json")).unwrap()).unwrap();
    assert_eq!(
        (&config["spec_root"], &config["version"]),
        (&json!("specs/main"), &json!("2.3.0"))
    );
    assert!(to.join("specs/main/governance/decisions.spec").is_file());
    assert!(to.join("specs/main/hello.spec").is_file());
    assert!(!to.join("spec").exists());
    let ledger: Value =
        serde_json::from_str(&std::fs::read_to_string(to.join("absorb-ledger.json")).unwrap())
            .unwrap();
    let kept: Vec<(&str, &str, &str)> = ledger["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["kind"] == "specforge.json" || e["kind"] == "spec")
        .map(|e| {
            let text = |key: &str| e[key].as_str().unwrap();
            (text("kind"), text("id"), text("field"))
        })
        .collect();
    assert_eq!(
        kept,
        [
            ("spec", "carried", "extensions"),
            ("spec", "carried", "version"),
            ("specforge.json", "carried", "extensions"),
        ],
        "{ledger:#}"
    );

    let (old, to) = (dir.path().join("rooted"), dir.path().join("tree"));
    decision_source(&old, "", "governance/decisions.spec");
    std::fs::create_dir_all(to.join("governance")).unwrap();
    std::fs::write(to.join("governance/README.md"), "notes\n").unwrap();
    let run = absorb(&["--from", path(&old), "--to", path(&to)]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.summary["target"]["errors"], 0, "{}", run.summary);
    let config: Value =
        serde_json::from_str(&std::fs::read_to_string(to.join("specforge.json")).unwrap()).unwrap();
    assert_eq!(config.get("spec_root"), None, "{config}");
    assert_eq!(config["version"], "0.1.0");
    assert!(to.join("governance/decisions.spec").is_file());
    assert!(to.join("hello.spec").is_file());
    let ledger = std::fs::read_to_string(to.join("absorb-ledger.json")).unwrap();
    assert!(
        ledger.contains("\"field\": \"specforge.json.version\""),
        "a version the source lacks is filled: {ledger}"
    );
}

/// A failure mode that names no invariant and rates neither its occurrence nor its detection converts: both are filled
/// at 10, the worst (ledgered), so its rpn is at or above `risk_block_on`, and a risk that high that nothing mitigates
/// is gsd/W532 in the target. That is the warning an unrated, unmitigated risk should give, not a converter gap.
#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn an_unrated_failure_mode_with_no_invariant_is_w532_in_the_target() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("new"));
    std::fs::create_dir_all(old.join("spec")).unwrap();
    std::fs::write(
        old.join("specforge.json"),
        r#"{"name": "risky", "spec_root": "spec", "extensions": ["@specforge/governance"]}"#,
    )
    .unwrap();
    std::fs::write(
        old.join("spec/failure-modes.spec"),
        "failure_mode lost_power \"Lost Power\" {\n  severity high\n  cause    \"The machine loses power mid-write.\"\n  \
         effect   \"The last entry is gone.\"\n}\n",
    )
    .unwrap();
    let run = absorb(&["--from", path(&old), "--to", path(&to)]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.summary["target"]["errors"], 0, "{}", run.summary);
    let findings: Vec<&str> = run.summary["target"]["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert_eq!(findings.len(), 1, "{findings:#?}");
    assert!(findings[0].starts_with("gsd/W532 "), "{findings:#?}");
    assert!(findings[0].contains("lost_power"), "{findings:#?}");
    let ledger = std::fs::read_to_string(to.join("absorb-ledger.json")).unwrap();
    for filled in ["risk.occurrence", "risk.detection"] {
        assert!(
            ledger.contains(&format!("\"field\": \"{filled}\"")),
            "{ledger}"
        );
    }
}

/// A source at `old` whose one term is `blocker`, an id the target reserves (gsd declares a `blocker` kind).
fn reserved_source(old: &Path) {
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
}

/// Every entry under `root` (symbolic links not followed), by its path from `root`, with a file's bytes.
fn tree_bytes(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    walkdir::WalkDir::new(root)
        .min_depth(1)
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .map(|e| {
            let key = e.path().strip_prefix(root).unwrap().display().to_string();
            let bytes = e
                .file_type()
                .is_file()
                .then(|| std::fs::read(e.path()).unwrap());
            (key, bytes)
        })
        .collect()
}

/// A stop in a directory the run did not make leaves it byte for byte as it was: init appended to its `.gitignore`
/// and `.gitattributes`, and the roll back gives them back their bytes.
#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn a_stop_leaves_a_directory_it_did_not_make_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("tree"));
    reserved_source(&old);
    std::fs::create_dir_all(to.join("src")).unwrap();
    std::fs::write(to.join(".gitignore"), "/target").unwrap();
    std::fs::write(to.join(".gitattributes"), "*.rs text\n").unwrap();
    std::fs::write(to.join("Cargo.toml"), "[workspace]\n").unwrap();
    std::fs::write(to.join("src/lib.rs"), "").unwrap();
    let before = tree_bytes(&to);
    let run = absorb(&["--from", path(&old), "--to", path(&to)]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr.contains("1 id(s) the target refuses"),
        "{}",
        run.stderr
    );
    assert_eq!(tree_bytes(&to), before);
}

/// A ledger that cannot be written is a stop before any write: the ledger is written before the edit plan applies, so
/// the record of what does not convert is never lost behind a written target, and the tree is left as it was.
#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn a_ledger_that_cannot_be_written_stops_before_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let to = dir.path().join("tree");
    std::fs::create_dir_all(&to).unwrap();
    std::fs::write(to.join("README.md"), "notes\n").unwrap();
    let not_a_dir = dir.path().join("a-file");
    std::fs::write(&not_a_dir, "").unwrap();
    let ledger = not_a_dir.join("ledger.json");
    let before = tree_bytes(&to);
    let run = absorb(&["--from", OLD, "--to", path(&to), "--ledger", path(&ledger)]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr.contains("the ledger: ") && run.stderr.contains("nothing was written"),
        "{}",
        run.stderr
    );
    assert_eq!(tree_bytes(&to), before);
}

#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn an_old_id_the_target_reserves_stops_the_run_before_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("new"));
    reserved_source(&old);
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
