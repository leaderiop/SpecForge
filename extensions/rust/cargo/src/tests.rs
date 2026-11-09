use super::*;
use serde_json::{json, Value};
use specforge_test::prelude::*;

/// The input the host sends for these `(path, text)` manifests and these listed target files.
fn input(manifests: &[(&str, &str)], targets: &[&str]) -> CollectInput {
    let manifests: Vec<Value> = manifests
        .iter()
        .map(|(path, text)| json!({"path": path, "exists": true, "text": text}))
        .collect();
    let targets: Vec<Value> = targets
        .iter()
        .map(|path| json!({"path": path, "exists": true}))
        .collect();
    serde_json::from_value(json!({
        "files": {MANIFESTS: {"facts": manifests}, "targets_lib": {"facts": targets}},
    }))
    .unwrap()
}

/// The facts collected from `manifests` and `targets`, by id.
fn facts(manifests: &[(&str, &str)], targets: &[&str]) -> Vec<Fact> {
    collect(&input(manifests, targets)).unwrap().facts
}

fn ids(facts: &[Fact]) -> Vec<&str> {
    facts.iter().map(|f| f.id.as_str()).collect()
}

const ROOT: &str = "[workspace]\nmembers = [\"crates/*\"]\n";
const BASE: &str = "[package]\nname = \"base\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
const APP: &str = "[package]\nname = \"app\"\nversion = \"0.2.0\"\nedition = \"2021\"\n\n\
                   [dependencies]\nbase = { path = \"../base\" }\nserde = \"1\"\n";

#[specforge_test(
    behavior = "rc_declare_cargo_collector",
    verify = "rust-cargo declares its collector, its reads and its peer"
)]
fn rust_cargo_declares_its_collector_its_reads_and_its_peer() {
    let declaration = crate::specforge_extension_build().declaration();
    assert_eq!(declaration.handshake.name, "@rust/cargo");
    let peers: Vec<(&str, &str, bool)> = declaration
        .handshake
        .peer_dependencies
        .iter()
        .map(|p| (p.name.as_str(), p.version.as_str(), p.optional))
        .collect();
    assert_eq!(peers, [("@code/core", "^1.1", false)]);
    let [collector] = &declaration.collectors[..] else {
        panic!("one collector: {:?}", declaration.collectors);
    };
    assert_eq!(collector.name, COLLECTOR);
    assert_eq!(collector.answers, ["package"]);
    assert_eq!(
        collector.auto_detect.as_ref().map(|d| &d.file_patterns[..]),
        Some(&["Cargo.toml".to_string()][..])
    );
    assert_eq!(
        (&collector.capability, &collector.report),
        (&None, &None),
        "no command, no report"
    );
    let reads: Vec<(String, String, bool)> = collector
        .inputs
        .iter()
        .map(|input| match input {
            Input::Files(read) => (
                read.name.clone(),
                read.glob.clone().unwrap_or_default(),
                read.text,
            ),
            other => panic!("only file reads: {other:?}"),
        })
        .collect();
    assert_eq!(reads.len(), 12);
    assert_eq!(
        reads[0],
        (MANIFESTS.to_string(), "**/Cargo.toml".to_string(), true)
    );
    assert!(
        reads[1..].iter().all(|(_, _, text)| !text),
        "a target file is listed, never read"
    );
    assert!(reads.contains(&(
        "targets_bin_dir".to_string(),
        "**/src/bin/*/main.rs".to_string(),
        false
    )));
}

#[specforge_test(
    behavior = "rc_map_manifests_to_package_facts",
    verify = "every package becomes a package fact"
)]
fn every_package_becomes_a_package_fact() {
    let facts = facts(
        &[
            ("Cargo.toml", ROOT),
            ("crates/app/Cargo.toml", APP),
            ("crates/base/Cargo.toml", BASE),
        ],
        &["crates/app/src/lib.rs", "crates/base/src/lib.rs"],
    );
    assert_eq!(ids(&facts), ["pkg:cargo/app", "pkg:cargo/base"]);
    let app = &facts[0];
    assert_eq!(app.kind, "package");
    assert_eq!(app.fields["version"], "0.2.0");
    assert_eq!(app.fields["path"], "crates/app");
    assert_eq!(
        app.fields["targets"],
        json!(["lib:app:crates/app/src/lib.rs"])
    );
}

#[specforge_test(
    behavior = "rc_map_manifests_to_package_facts",
    verify = "depends_on names the project packages only"
)]
fn depends_on_names_the_project_packages_only() {
    let facts = facts(
        &[
            ("crates/app/Cargo.toml", APP),
            ("crates/base/Cargo.toml", BASE),
        ],
        &[],
    );
    assert_eq!(facts[0].fields["depends_on"], json!(["pkg:cargo/base"]));
    assert!(
        !facts[1].fields.contains_key("depends_on"),
        "serde is no project package"
    );
}

#[specforge_test(
    behavior = "rc_map_manifests_to_package_facts",
    verify = "a name two workspaces share keeps the nearest plain and the other its directory"
)]
fn a_name_two_workspaces_share_keeps_the_nearest_plain() {
    let facts = facts(
        &[
            ("crates/base/Cargo.toml", BASE),
            ("fixtures/deep/base/Cargo.toml", BASE),
            ("fixtures/x/Cargo.toml", BASE),
        ],
        &[],
    );
    assert_eq!(
        ids(&facts),
        [
            "pkg:cargo/base",
            "pkg:cargo/base#fixtures/deep/base",
            "pkg:cargo/base#fixtures/x",
        ],
        "the shallowest directory (then the lexically first) keeps the plain id"
    );
}

#[specforge_test(
    behavior = "rc_map_manifests_to_package_facts",
    verify = "a withheld or truncated read is the collector's error"
)]
fn a_withheld_or_truncated_read_is_the_collectors_error() {
    let withheld: CollectInput = serde_json::from_value(json!({
        "files": {MANIFESTS: {"facts": [
            {"path": "Cargo.toml", "exists": true, "text": BASE},
            {"path": "big/Cargo.toml", "exists": true, "error": "too_large"},
        ]}},
    }))
    .unwrap();
    let why = collect(&withheld).unwrap_err();
    assert!(why.starts_with("big/Cargo.toml: "), "{why}");
    assert!(why.contains("too_large"), "{why}");
    for read in [MANIFESTS, "targets_test"] {
        let truncated: CollectInput = serde_json::from_value(json!({
            "files": {read: {"facts": [], "truncated": true}},
        }))
        .unwrap();
        let why = collect(&truncated).unwrap_err();
        assert!(why.contains(&format!("'{read}'")), "{why}");
    }
    let unreadable = collect(&input(&[("bad/Cargo.toml", "[package\n")], &[])).unwrap_err();
    assert!(unreadable.starts_with("bad/Cargo.toml: "), "{unreadable}");
}

#[specforge_test(
    behavior = "rc_map_manifests_to_package_facts",
    verify = "a package fact is located at its package table"
)]
fn a_package_fact_is_located_at_its_package_table() {
    let text = format!("# the base\n\n{BASE}");
    let facts = facts(&[("crates/base/Cargo.toml", &text)], &[]);
    let at: Vec<(&str, usize, usize, usize, usize)> = facts[0]
        .at
        .iter()
        .map(|at| {
            (
                at.file.as_str(),
                at.start_line,
                at.start_col,
                at.end_line,
                at.end_col,
            )
        })
        .collect();
    assert_eq!(at, [("crates/base/Cargo.toml", 3, 2, 3, 9)]);
}
