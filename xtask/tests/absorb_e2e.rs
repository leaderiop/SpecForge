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

/// Every one of the 17 old kinds the fixture holds converts or is dropped whole (ADR 0372 D1): the target checks with
/// no error and no warning, nothing waits, and an entity dropped whole leaves its methods and field annotations in the
/// ledger (ADR 0372 D2).
#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn all_seventeen_old_kinds_convert_or_drop_with_no_error_and_no_warning() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a"), dir.path().join("b"));
    let run = absorb(&["--from", OLD, "--to", path(&a)]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.summary["target"]["errors"], 0, "{}", run.summary);
    assert_eq!(run.summary["target"]["warnings"], 0, "{}", run.summary);
    assert_eq!(
        run.summary["work"],
        json!({"compiles": 3, "edit_plans": 1}),
        "three compiles and one edit plan, whatever the size"
    );
    assert_eq!(
        (&run.summary["converted"], &run.summary["creates"]),
        (&json!(17), &json!(17)),
        "the decisions in force, constraints, invariants, features, releases and the milestones they list (ADR \
         0372 D1, D4, D5: the decisions not in force, the milestone no release lists and the eleven kinds dropped \
         whole convert to nothing); nothing is created: {}",
        run.summary
    );
    assert_eq!(run.summary.get("created"), None, "{}", run.summary);
    for dropped_whole in [
        "spec/product/journeys.spec",
        "spec/product/deliverables.spec",
        "spec/product/modules.spec",
        "spec/behaviors",
        "spec/software",
    ] {
        assert!(
            !a.join(dropped_whole).exists(),
            "{dropped_whole}: a kind dropped whole writes nothing"
        );
    }
    let invariants = std::fs::read_to_string(a.join("spec/governance/invariants.spec")).unwrap();
    assert_eq!(
        invariants.matches("requirement ").count(),
        2,
        "each invariant is a requirement: {invariants}"
    );
    assert!(
        run.summary["ledger"].get("later").is_none(),
        "nothing waits: {}",
        run.summary["ledger"]
    );
    assert_eq!(
        run.summary["cited_tests"],
        json!([]),
        "absorb cites no test"
    );
    let features = std::fs::read_to_string(a.join("spec/product/features.spec")).unwrap();
    assert!(
        !features.contains("decision ")
            && features.contains("Write each entry to the store as it is saved."),
        "a feature's solution is in its requirement's text, and no decision is created: {features}"
    );
    let constraints = std::fs::read_to_string(a.join("spec/governance/constraints.spec")).unwrap();
    let throughput = &constraints[constraints.find("requirement PERFORMANCE-02").unwrap()..];
    let throughput = &throughput[..throughput.find("\n}").unwrap()];
    let lines: Vec<&str> = throughput.lines().map(str::trim).collect();
    let closing = lines.iter().rposition(|line| *line == "\"\"\"").unwrap();
    assert_eq!(
        lines[closing - 1],
        "throughput >= 200",
        "the bound ends the text, its threshold on the metric's line: {throughput}"
    );
    let decisions = std::fs::read_to_string(a.join("spec/governance/decisions.spec")).unwrap();
    let written: Vec<&str> = decisions
        .lines()
        .filter_map(|line| line.strip_prefix("decision "))
        .filter_map(|rest| rest.split_whitespace().next())
        .collect();
    assert_eq!(
        written,
        ["D-01", "D-02"],
        "only a decision in force converts, as GSD's D-NN in old-id order: {decisions}"
    );
    let ledger = std::fs::read_to_string(a.join("absorb-ledger.json")).unwrap();
    let entries: Value = serde_json::from_str(&ledger).unwrap();
    for id in [
        "lazy_cache",
        "sync_writes",
        "first_store",
        "old_format",
        "stale_choice",
    ] {
        let reasons: Vec<&str> = entries["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "decision" && e["id"] == id)
            .map(|e| e["why"]["dropped"].as_str().unwrap_or_default())
            .collect();
        assert!(
            reasons.len() >= 3 && reasons.iter().all(|r| r.contains("docs/adr/")),
            "{id}: a decision not in force is dropped whole, docs/adr/ its record: {reasons:?}"
        );
    }
    assert!(
        dropped(&ledger, "EntryStore", "method") && dropped(&ledger, "SavedEntry", "annotations"),
        "a port's methods and a type's member annotations are ledgered: {ledger}"
    );
    releases_become_version_milestones_and_their_milestones_phases(&a, &ledger);

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
        (&run.summary["converted"], &run.summary["creates"]),
        (&json!(4), &json!(4)),
        "the decision, the constraint, the invariant and the feature of the nine selected ids (the superseded \
         decision, the failure mode, the journey, the persona and the channel are dropped whole): {}",
        run.summary
    );
    assert_eq!(run.summary.get("created"), None, "{}", run.summary);
    let tree = spec_tree(&to);
    assert!(
        !tree.values().any(|text| text.contains("second_store")),
        "an entity the selection leaves out is never stubbed"
    );
    let ledger = std::fs::read_to_string(to.join("absorb-ledger.json")).unwrap();
    assert!(
        dropped(&ledger, "record_an_entry", "persona"),
        "a selected journey's values are dropped: {ledger}"
    );
    insta::assert_snapshot!("selection_ledger", ledger);
}

/// ADR 0372 D3, D4, D7 on the converted fixture: GSD's requirement and decision ids, two version milestones, three
/// phases keyed by path with their references written through the id map, the tiers the rule gives, and each phase's
/// computed status read by its path.
fn releases_become_version_milestones_and_their_milestones_phases(to: &Path, ledger: &str) {
    let releases = std::fs::read_to_string(to.join("spec/product/releases.spec")).unwrap();
    assert!(
        releases.contains("milestone v0.1.0 \"First Release\"")
            && releases.contains("milestone v0.2.0 \"Second Release\""),
        "{releases}"
    );
    let phases = std::fs::read_to_string(to.join("spec/product/milestones.spec")).unwrap();
    assert_eq!(
        phases.matches("\nphase ").count() + usize::from(phases.starts_with("phase ")),
        3,
        "{phases}"
    );
    let block = |header: &str| {
        let at = phases
            .find(header)
            .unwrap_or_else(|| panic!("{header}: {phases}"));
        let end = phases[at..]
            .find("\n}")
            .map_or(phases.len(), |end| at + end);
        phases[at..end].to_string()
    };
    let first = block("phase 01 \"First Cut\"");
    assert!(
        first.contains("milestone        v0.1.0")
            && first.contains("requirements     [FEATURE-03]"),
        "{first}"
    );
    let second = block("phase 02 \"Second Cut\"");
    assert!(
        second.contains("depends_on       [01]"),
        "a dependency within one version milestone, as the writer prints v0.1.0/01 there: {second}"
    );
    let third = block("phase 01 \"Phase 3: Third Cut\"");
    assert!(
        third.contains("milestone        v0.2.0")
            && third.contains("name             \"Third Cut\"")
            && !third.contains("depends_on"),
        "{third}"
    );

    let ledger: Value = serde_json::from_str(ledger).unwrap();
    let ids: BTreeMap<&str, &str> = ledger["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["old"].as_str().unwrap(), e["new"].as_str().unwrap()))
        .collect();
    for (old, new) in [
        ("list_entries", "FEATURE-01"),
        ("old_export", "FEATURE-02"),
        ("store_entries", "FEATURE-03"),
        ("entries_are_whole", "INVARIANT-01"),
        ("writes_are_durable", "INVARIANT-02"),
        ("bounded_retries", "INTEROPERABILITY-01"),
        ("write_latency", "PERFORMANCE-01"),
        ("write_throughput", "PERFORMANCE-02"),
        ("retry_window", "RELIABILITY-01"),
        ("calm_wait", "USABILITY-01"),
        ("one_parser", "D-01"),
        ("second_store", "D-02"),
        ("first_release", "v0.1.0"),
        ("second_release", "v0.2.0"),
        ("first_cut", "v0.1.0/01"),
        ("second_cut", "v0.1.0/02"),
        ("third_cut", "v0.2.0/01"),
    ] {
        assert_eq!(ids.get(old), Some(&new), "{old}");
    }
    assert!(
        !ids.contains_key("groundwork"),
        "a milestone no release lists converts to nothing"
    );
    let entries = ledger["entries"].as_array().unwrap();
    let entry = |id: &str, field: &str| {
        entries
            .iter()
            .filter(|e| e["id"] == id && e["field"] == field)
            .collect::<Vec<_>>()
    };
    assert!(
        entry("groundwork", "title")[0]["why"]["dropped"]
            .as_str()
            .unwrap()
            .contains("no converted release lists this milestone")
    );
    for (id, on) in [("third_cut", "first_cut"), ("first_cut", "groundwork")] {
        assert!(
            entry(id, "depends_on")
                .iter()
                .any(|e| e["value"] == on && e["why"]["dropped"].is_string()),
            "{id} on {on}"
        );
    }
    assert_eq!(
        entry("first_cut", "status")[0]["why"],
        json!({"computed": {"field": "phase.status", "new": "not_started"}}),
        "the phase's computed status, read by its path v0.1.0/01: a null would mean it was not found"
    );
    let constraints = std::fs::read_to_string(to.join("spec/governance/constraints.spec")).unwrap();
    let tier_is = |text: &str, tier: &str| {
        text.lines()
            .filter(|l| {
                l.trim_start().starts_with("tier ") && l.trim_end().ends_with(&format!(" {tier}"))
            })
            .count()
    };
    let tiers = tier_is(&constraints, "v2");
    assert_eq!(
        (tiers, constraints.matches("\nrequirement ").count() + 1),
        (5, 5),
        "every constraint is v2: no phase lists one: {constraints}"
    );
    let features = std::fs::read_to_string(to.join("spec/product/features.spec")).unwrap();
    let store = &features[features.find("requirement FEATURE-03").unwrap()..];
    assert_eq!(
        tier_is(&store[..store.find("\n}").unwrap()], "v1"),
        1,
        "{features}"
    );
}

/// Whether `ledger` holds a `dropped` entry for `field` of the old entity `id`.
fn dropped(ledger: &str, id: &str, field: &str) -> bool {
    let ledger: serde_json::Value = serde_json::from_str(ledger).unwrap();
    ledger["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["id"] == id && e["field"] == field && e["why"]["dropped"].is_string())
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

/// The version the old product builtin accepts and the id grammar refuses: its milestone would be `v1.0.0+build.1`.
const REFUSED_VERSION: &str = "1.0.0+build.1";

/// A source at `old` whose one release has [`REFUSED_VERSION`]: it converts to a version milestone whose id the target
/// refuses (`+` is no id character), so the run stops after init wrote, before the edit plan applies.
fn refused_version_source(old: &Path) {
    std::fs::create_dir_all(old.join("spec")).unwrap();
    std::fs::write(
        old.join("specforge.json"),
        r#"{"name": "refused", "spec_root": "spec", "extensions": ["@specforge/product"]}"#,
    )
    .unwrap();
    std::fs::write(
        old.join("spec/releases.spec"),
        format!(
            "release build \"Build\" {{\n  version     \"{REFUSED_VERSION}\"\n  description \"A build.\"\n  status      \
             planned\n}}\n"
        ),
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

/// A stop in a directory the run did not make leaves it byte for byte as it was: the refused version stops the run
/// after init appended to its `.gitignore` and `.gitattributes`, and the roll back gives them back their bytes.
#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn a_stop_leaves_a_directory_it_did_not_make_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("tree"));
    refused_version_source(&old);
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

/// A symbolic link at the ledger path, the default one in `--to` or `--ledger`'s, is refused before any write: the file
/// it points to, outside `--to`, is never written through, and `--to` is left as it was.
#[cfg(unix)]
#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn a_symbolic_link_at_the_ledger_path_is_refused_before_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("outside.json");
    std::fs::write(&outside, "mine\n").unwrap();
    let to = dir.path().join("tree");
    std::fs::create_dir_all(&to).unwrap();
    std::os::unix::fs::symlink(&outside, to.join("absorb-ledger.json")).unwrap();
    let before = tree_bytes(&to);
    let run = absorb(&["--from", OLD, "--to", path(&to)]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr
            .contains("is a symbolic link: absorb never writes through one"),
        "{}",
        run.stderr
    );
    assert_eq!(tree_bytes(&to), before);
    let given = dir.path().join("given.json");
    std::os::unix::fs::symlink(&outside, &given).unwrap();
    let fresh = dir.path().join("fresh");
    let run = absorb(&[
        "--from",
        OLD,
        "--to",
        path(&fresh),
        "--ledger",
        path(&given),
    ]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(!fresh.exists(), "a --to it would make is not made");
    assert_eq!(
        std::fs::read_to_string(&outside).unwrap(),
        "mine\n",
        "never written through"
    );
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

/// A milestone id the target refuses, from a release's version, stops the run before any write, naming it; the scaffold
/// is removed (ADR 0372 D3: every id goes through the target's id rule).
#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn a_version_the_target_refuses_as_an_id_stops_the_run_before_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("new"));
    refused_version_source(&old);
    let run = absorb(&["--from", path(&old), "--to", path(&to)]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr.contains("1 id(s) the target refuses")
            && run.stderr.contains(&format!(
                "milestone 'v{REFUSED_VERSION}': invalid entity ID"
            )),
        "{}",
        run.stderr
    );
    assert!(!to.exists(), "the scaffold is removed");
}

/// An old id that is a kind keyword of the target (gsd declares a `blocker` kind) converts under its GSD id: a decision
/// `blocker` is `D-01`, and the target checks clean (ADR 0372 D3).
#[test]
#[ignore = "needs GSD_CORE_WASM"]
fn an_old_id_that_is_a_kind_keyword_converts_under_its_gsd_id() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("new"));
    std::fs::create_dir_all(old.join("spec")).unwrap();
    std::fs::write(
        old.join("specforge.json"),
        r#"{"name": "reserved", "spec_root": "spec", "extensions": ["@specforge/governance"]}"#,
    )
    .unwrap();
    std::fs::write(
        old.join("spec/decisions.spec"),
        "decision blocker \"Blocker\" {\n  status   accepted\n  context  \"Work stops.\"\n  decision \"Stop \
         work.\"\n}\n",
    )
    .unwrap();
    let run = absorb(&["--from", path(&old), "--to", path(&to)]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.summary["target"]["errors"], 0, "{}", run.summary);
    let decisions = std::fs::read_to_string(to.join("spec/decisions.spec")).unwrap();
    assert!(
        decisions.contains("decision D-01 \"Blocker\""),
        "{decisions}"
    );
    let ledger: Value =
        serde_json::from_str(&std::fs::read_to_string(to.join("absorb-ledger.json")).unwrap())
            .unwrap();
    assert_eq!(
        ledger["ids"],
        json!([{"kind": "decision", "old": "blocker", "new": "D-01"}])
    );
}
