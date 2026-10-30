//! A compute pass at the baseline reads the files the commit holds, not the working tree's (ADR 0300, behavior
//! `read_git_baseline`).

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Value, json};
use specforge_project::baseline::Baseline;
use specforge_project::history::{History, Located, Sha};
use specforge_project::judged::file_digest;
use specforge_project::testing::{FakeHistory, gsd_facts_project, gsd_facts_serving, pass_input};
use specforge_test::prelude::*;
use specforge_wasm::testing::InProcessRuntime;

/// The compute pass `scan`: it reads the files `plan.files_modified` names and the sources under `src`.
fn scan_runtime() -> InProcessRuntime {
    gsd_facts_serving(
        json!([{"name": "scan", "phase": "compute", "baseline": ["plan"], "reads": [
            {"name": "modified", "field": "plan.files_modified", "text": true},
            {"name": "sources", "glob": "src/**/*.rs"},
        ]}]),
    )
}

/// The `scan` input of a baseline at a commit that holds only `a.spec` as a spec, and `src/lib.rs` and
/// `src/nested/deep.rs` as other files.
fn baseline_input() -> Value {
    let project = gsd_facts_project();
    let runtime = Arc::new(scan_runtime());
    let env = project.environment_in(runtime.clone());
    let sha = Sha::new("c".repeat(40));
    let mut fake = FakeHistory::repo(
        project.root(),
        sha.as_str(),
        vec![("a.spec".into(), project.read("a.spec").into_bytes())],
    );
    fake.blobs = vec![
        (PathBuf::from("src/lib.rs"), b"pub fn old() {}\n".to_vec()),
        (PathBuf::from("src/nested/deep.rs"), b"x\n".to_vec()),
    ];
    let Located::Repo(place) = History::locate(&fake, project.root()) else {
        panic!("a repository")
    };
    Baseline::at(&env, &fake, &place, &sha).expect("a baseline");
    pass_input(&runtime, "scan")
}

fn paths(input: &Value, read: &str) -> Vec<String> {
    input["files"][read]["facts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap().to_string())
        .collect()
}

#[ignore = "pins K19; T3 un-ignores"]
#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a compute pass at the baseline reads the commit's files"
)]
fn a_compute_pass_at_the_baseline_reads_the_commits_files() {
    let input = baseline_input();
    let facts = input["files"]["modified"]["facts"].as_array().unwrap();
    assert_eq!(paths(&input, "modified"), ["docs/gone.md", "src/lib.rs"]);
    assert_eq!(facts[0]["exists"], false);
    let lib = &facts[1];
    assert_eq!(lib["exists"], true, "{lib}");
    assert_eq!(lib["bytes"], 16);
    assert_eq!(lib["lines"], 1);
    assert_eq!(lib["text"], "pub fn old() {}\n");
    assert_eq!(lib["digest"], file_digest(b"pub fn old() {}\n").as_str());
}

#[ignore = "pins K19; T3 un-ignores"]
#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a glob read at the baseline lists the commit's tree"
)]
fn a_glob_read_at_the_baseline_lists_the_commits_tree() {
    let input = baseline_input();
    assert_eq!(
        paths(&input, "sources"),
        ["src/lib.rs", "src/nested/deep.rs"],
        "the working tree has no deep.rs"
    );
}
