//! `absorb` end to end: the old fixture project into a fresh `@gsd/core` project, which checks clean. The target loads
//! `@gsd/core` by name, the builtin the CLI embeds, so nothing is built first:
//! `cargo nextest run -p xtask -E 'binary_id(xtask::absorb_e2e)'`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

const OLD: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/absorb/old");
const SELECTION: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/absorb/selection.json"
);

// A test binary's root finds its modules beside itself, in `tests/`, where cargo would build each file as a test
// binary of its own; the path keeps this one inside `absorb_e2e/`. The crate's lint allowance above covers it.
#[path = "absorb_e2e/target_paths.rs"]
mod target_paths;

/// One run of the command: its exit code, its summary (stdout) and its stderr.
struct Run {
    code: i32,
    summary: Value,
    stderr: String,
}

fn absorb(args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_absorb"))
        .args(args)
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

/// Every file under `root/spec`, by its path from `root`, `/`-separated on every OS.
fn spec_tree(root: &Path) -> BTreeMap<String, String> {
    walkdir::WalkDir::new(root.join("spec"))
        .sort_by_file_name()
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let key = e
                .path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (key, std::fs::read_to_string(e.path()).unwrap())
        })
        .collect()
}

/// Every one of the 17 old kinds the fixture holds converts or is dropped whole (ADR 0372 D1): the target checks with
/// no error and no warning, nothing waits, and an entity dropped whole leaves its methods and field annotations in the
/// ledger (ADR 0372 D2), as its `ref` declaration leaves its title and metadata.
#[test]
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
    assert!(
        dropped(&ledger, "gh.issue:42", "title") && dropped(&ledger, "gh.issue:42", "labels"),
        "a ref declaration is dropped whole, its title and metadata ledgered: {ledger}"
    );
    releases_become_version_milestones_and_their_milestones_phases(&a, &ledger);
    only_the_scaffold_entries_are_unconverted(&ledger);

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
    let third = block("phase 01 \"Third Cut\"");
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
    let numbered = entry("third_cut", "title");
    assert_eq!(
        (numbered.len(), &numbered[0]["value"]),
        (1, &json!("Phase 3:")),
        "the old title's number, not written, is in the ledger: {numbered:?}"
    );
    assert!(
        numbered[0]["why"]["dropped"]
            .as_str()
            .unwrap()
            .contains("GSD labels a phase"),
        "{numbered:?}"
    );
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

/// Only the target's scaffold entries, its `spec` block's and its `specforge.json`'s, are `unconverted`, each with the
/// one outcome (ADR 0372 D8).
fn only_the_scaffold_entries_are_unconverted(ledger: &str) {
    let ledger: Value = serde_json::from_str(ledger).unwrap();
    let unconverted: Vec<&Value> = ledger["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["why"].get("unconverted").is_some())
        .collect();
    assert!(!unconverted.is_empty(), "{ledger:#}");
    for e in unconverted {
        assert!(
            (e["kind"] == "spec" || e["kind"] == "specforge.json")
                && e["why"]["unconverted"] == "written fresh by absorb",
            "{e:#}"
        );
    }
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
    only_the_scaffold_entries_are_unconverted(&ledger.to_string());

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

/// An old id that is a kind keyword of the target (gsd declares a `blocker` kind) converts under its GSD id: a decision
/// `blocker` is `D-01`, and the target checks clean (ADR 0372 D3).
#[test]
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

/// A source at `old` loading the product and governance builtins, its one spec file `spec/plan.spec` holding `text`.
fn product_source(old: &Path, text: &str) {
    std::fs::create_dir_all(old.join("spec")).unwrap();
    std::fs::write(
        old.join("specforge.json"),
        r#"{"name": "planned", "spec_root": "spec", "extensions": ["@specforge/product", "@specforge/governance"]}"#,
    )
    .unwrap();
    std::fs::write(old.join("spec/plan.spec"), text).unwrap();
}

/// A feature two converted milestones list is in one phase, the first in source order, and the later listing is
/// ledgered `dropped`: the target reports no `gsd/E101`, and checks clean (ADR 0372 D7). (A source whose id two kinds
/// share is refused before any write: its graph drops the second declaration, W060.)
#[test]
fn a_feature_two_phases_list_is_in_one_phase_and_the_target_checks_clean() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("new"));
    let cut = |id: &str, title: &str| {
        format!(
            "milestone {id} \"{title}\" {{\n  description   \"The {id} cut.\"\n  status        planned\n  \
             features      [store]\n  exit_criteria [\"Entries are saved\", \"Entries are listed\"]\n}}\n\n"
        )
    };
    product_source(
        &old,
        &format!(
            "release first \"First\" {{\n  version    \"0.1.0\"\n  status     planned\n  milestones [one, two]\n}}\n\n\
             {}{}feature store \"Store\" {{\n  problem \"Entries are lost.\"\n  status  done\n}}\n",
            cut("one", "One"),
            cut("two", "Two")
        ),
    );
    let run = absorb(&["--from", path(&old), "--to", path(&to)]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.summary["target"]["errors"], 0, "{}", run.summary);
    assert_eq!(run.summary["target"]["warnings"], 0, "{}", run.summary);
    let written = std::fs::read_to_string(to.join("spec/plan.spec")).unwrap();
    assert_eq!(
        written.matches("[FEATURE-01]").count(),
        1,
        "one phase lists the requirement: {written}"
    );
    let ledger = std::fs::read_to_string(to.join("absorb-ledger.json")).unwrap();
    assert!(dropped(&ledger, "two", "features"), "{ledger}");
    assert!(!dropped(&ledger, "one", "features"), "{ledger}");
}

/// `--selection` is written only once the run passes its stops: a sample whose milestone two releases list stops before
/// any write, and leaves no selection file behind, nor a target.
#[test]
fn a_stop_leaves_no_selection_behind() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("new"));
    let selection = dir.path().join("selection.json");
    std::fs::create_dir_all(old.join("spec/product")).unwrap();
    std::fs::write(
        old.join("specforge.json"),
        r#"{"name": "sampled", "spec_root": "spec", "extensions": ["@specforge/product"]}"#,
    )
    .unwrap();
    // The wasm-host sample takes every entity of this file whole.
    std::fs::write(
        old.join("spec/product/personas.spec"),
        "release one \"One\" {\n  version    \"0.1.0\"\n  status     planned\n  milestones [shared]\n}\n\n\
         release two \"Two\" {\n  version    \"0.2.0\"\n  status     planned\n  milestones [shared]\n}\n\n\
         milestone shared \"Shared\" {\n  description \"Listed twice.\"\n  status      planned\n}\n",
    )
    .unwrap();
    let run = absorb(&[
        "--from",
        path(&old),
        "--to",
        path(&to),
        "--only",
        "sample:wasm-host",
        "--selection",
        path(&selection),
    ]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr.contains("stop(s) before any write")
            && run.stderr.contains("listed by 2 releases"),
        "{}",
        run.stderr
    );
    assert!(!selection.exists(), "a stop writes no selection");
    assert!(!to.exists(), "nor a target");
}

/// An empty required value is a missing one: a phase whose name gives an empty slug and a constraint whose only text is
/// empty stop the run before any write, each named, and no target is left.
#[test]
fn an_empty_required_value_stops_the_run_as_missing() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("new"));
    product_source(
        &old,
        "release first \"First\" {\n  version    \"0.1.0\"\n  status     planned\n  milestones [one]\n}\n\n\
         milestone one \"第一段\" {\n  description   \"The first cut.\"\n  status        planned\n  \
         exit_criteria [\"Entries are saved\", \"Entries are listed\"]\n}\n\n\
         constraint blank \"Blank\" {\n  description \"\"\n}\n",
    );
    let run = absorb(&["--from", path(&old), "--to", path(&to)]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(
        run.stderr.contains("2 required value(s)")
            && run.stderr.contains("phase 'v0.1.0/01' slug")
            && run.stderr.contains("requirement 'CONSTRAINT-01' text"),
        "{}",
        run.stderr
    );
    assert!(!to.exists(), "the scaffold is removed");
}

/// A constraint of category `d` converts beside a decision: its id is `CONSTRAINT-01`, never the decision's `D-01`.
#[test]
fn a_constraint_of_category_d_converts_beside_a_decision() {
    let dir = tempfile::tempdir().unwrap();
    let (old, to) = (dir.path().join("old"), dir.path().join("new"));
    product_source(
        &old,
        "constraint short \"Short\" {\n  description \"Short words.\"\n  category    \"d\"\n}\n\n\
         decision one_parser \"One Parser\" {\n  status   accepted\n  context  \"Readers drift.\"\n  \
         decision \"Read with one parser.\"\n}\n",
    );
    let run = absorb(&["--from", path(&old), "--to", path(&to)]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.summary["target"]["errors"], 0, "{}", run.summary);
    let written = std::fs::read_to_string(to.join("spec/plan.spec")).unwrap();
    assert!(
        written.contains("requirement CONSTRAINT-01") && written.contains("decision D-01"),
        "{written}"
    );
}
