use super::*;
use serde_json::json;
use specforge_extension_sdk::SettingsItem;
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

#[specforge_test(requirement = "FEATURE-002")]
#[specforge_test(requirement = "FEATURE-108")]
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
    let [SettingsItem::Setting(exclude)] = &declaration.settings[..] else {
        panic!("one setting: {:?}", declaration.settings);
    };
    assert_eq!(
        (
            exclude.name.as_str(),
            exclude.field_type.as_str(),
            &exclude.default
        ),
        ("exclude", "string_list", &Some(json!([])))
    );
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

#[specforge_test(requirement = "FEATURE-002")]
#[specforge_test(requirement = "FEATURE-108")]
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

#[specforge_test(requirement = "FEATURE-002")]
#[specforge_test(requirement = "FEATURE-108")]
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

#[specforge_test(requirement = "FEATURE-002")]
#[specforge_test(requirement = "FEATURE-108")]
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

#[specforge_test(requirement = "FEATURE-002")]
#[specforge_test(requirement = "FEATURE-108")]
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
        // The guest cannot tell the walk's entry cap from the file cap: the error names both.
        assert!(
            why.contains("stopped early: the host's walk reached its cap of 20000 entries")
                && why.contains("or more than 2000 files matched"),
            "{why}"
        );
    }
    let unreadable = collect(&input(&[("bad/Cargo.toml", "[package\n")], &[])).unwrap_err();
    assert!(unreadable.starts_with("bad/Cargo.toml: "), "{unreadable}");
}

#[specforge_test(requirement = "FEATURE-002")]
#[specforge_test(requirement = "FEATURE-108")]
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

#[specforge_test(requirement = "FEATURE-002")]
#[specforge_test(requirement = "FEATURE-108")]
fn a_directory_excludes_itself_and_what_lies_under_it() {
    let examples = ["examples".to_string()];
    assert!(excluded("examples/shop", &examples));
    assert!(excluded("examples/shop", &["examples/shop".to_string()]));
    assert!(!excluded("examplesx", &examples), "a whole segment only");
    assert!(!excluded("", &examples), "the root is under no entry");
}

#[specforge_test(requirement = "FEATURE-002")]
#[specforge_test(requirement = "FEATURE-108")]
fn the_exclude_list_takes_project_relative_directories_only() {
    assert_eq!(exclude_list(None), Ok(Vec::new()));
    assert_eq!(
        exclude_list(Some(&json!(["gsd/parity/fixtures/", "examples"]))),
        Ok(vec![
            "gsd/parity/fixtures".to_string(),
            "examples".to_string()
        ])
    );
    for (entry, named) in [
        (json!("/abs"), "/abs"),
        (json!("C:/x"), "C:/x"),
        (json!(""), ""),
        (json!("a/../b"), "a/../b"),
        (json!(7), "7"),
    ] {
        assert_eq!(
            exclude_list(Some(&json!([entry]))),
            Err(format!(
                "exclude entry '{named}' is not a project-relative directory"
            ))
        );
    }
}

#[specforge_test(requirement = "FEATURE-002")]
#[specforge_test(requirement = "FEATURE-108")]
fn a_member_under_an_excluded_directory_is_no_package() {
    let manifests = [
        ("Cargo.toml", ROOT),
        ("crates/app/Cargo.toml", APP),
        ("crates/base/Cargo.toml", BASE),
        ("fixtures/x/Cargo.toml", BASE),
    ];
    let out = packages_of(&input(&manifests, &[]), &["fixtures".to_string()]).unwrap();
    assert_eq!(ids(&out.facts), ["pkg:cargo/app", "pkg:cargo/base"]);
    let out = packages_of(&input(&manifests, &[]), &["crates/base".to_string()]).unwrap();
    assert_eq!(
        ids(&out.facts),
        ["pkg:cargo/app", "pkg:cargo/base#fixtures/x"],
        "a workspace member excluded after the read; its twin keeps its own id"
    );
}
