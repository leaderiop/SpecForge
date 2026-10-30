//! A compute pass at the baseline reads the files the commit holds, not the working tree's (ADR 0300, behavior
//! `read_git_baseline`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Value, json};
use specforge_common::regular::MAX_FILE_BYTES;
use specforge_project::baseline::Baseline;
use specforge_project::history::{History, Located, Sha};
use specforge_project::judged::file_digest;
use specforge_project::testing::{
    FakeHistory, TestProject, gsd_facts_project, gsd_facts_serving, gsd_history_runtime, pass_input,
};
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

/// A fake repository at `project` whose commit holds only `a.spec` as a spec, and `src/lib.rs` and
/// `src/nested/deep.rs` as other files.
fn commit(project: &TestProject) -> FakeHistory {
    let mut fake = FakeHistory::repo(
        project.root(),
        &"c".repeat(40),
        vec![("a.spec".into(), project.read("a.spec").into_bytes())],
    );
    fake.blobs = vec![
        (PathBuf::from("src/lib.rs"), b"pub fn old() {}\n".to_vec()),
        (PathBuf::from("src/nested/deep.rs"), b"x\n".to_vec()),
    ];
    fake
}

/// The baseline of `fake`'s `HEAD` in `project` over `runtime`.
fn baseline_of(project: &TestProject, runtime: &Arc<InProcessRuntime>, fake: &FakeHistory) {
    let env = project.environment_in(runtime.clone());
    let Located::Repo(place) = History::locate(fake, project.root()) else {
        panic!("a repository")
    };
    let sha = Sha::new("c".repeat(40));
    Baseline::at(&env, fake, &place, &sha).expect("a baseline");
}

/// The `scan` input of the baseline of [`commit`].
fn baseline_input() -> Value {
    let project = gsd_facts_project();
    let runtime = Arc::new(scan_runtime());
    baseline_of(&project, &runtime, &commit(&project));
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

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a baseline asks for the commit's tree only when a compute pass reads, and runs each compute pass once"
)]
fn a_baseline_asks_no_tree_without_a_reading_compute_pass() {
    let project = crate::computed::project("verification V \"V\" {\n  phase 03\n}\n");
    let runtime = Arc::new(gsd_history_runtime(json!([])));
    let fake = FakeHistory::repo(
        project.root(),
        &"c".repeat(40),
        vec![("a.spec".into(), project.read("a.spec").into_bytes())],
    );
    baseline_of(&project, &runtime, &fake);
    assert_eq!(fake.trees_asked(), 0);
    assert_eq!(fake.blobs_asked(), 0);
}

/// Guards the prefetch: it passes before the fix too, it is not a red-first pin.
#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a baseline asks for the commit's tree only when a compute pass reads, and runs each compute pass once"
)]
fn each_compute_pass_runs_once_at_the_baseline() {
    let project = gsd_facts_project();
    let runtime = Arc::new(scan_runtime());
    let fake = commit(&project);
    baseline_of(&project, &runtime, &fake);
    let calls = runtime
        .calls()
        .into_iter()
        .filter(|call| call.export == "__pass_scan")
        .count();
    assert_eq!(calls, 1);
    assert_eq!(fake.trees_asked(), 1);
    assert_eq!(fake.blobs_asked(), 1);
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a file git cannot give or a file over the cap is not absent at the baseline"
)]
fn a_file_git_cannot_give_is_unreadable_at_the_baseline() {
    let project = gsd_facts_project();
    let spec = project.read("a.spec").replace(
        "[\"src/lib.rs\", \"docs/gone.md\"]",
        "[\"src/lost.rs\", \"src/big.bin\", \"vendor/lib/x.rs\", \"src\"]",
    );
    project.write("a.spec", &spec);
    let mut fake = commit(&project);
    fake.listed = vec![
        (PathBuf::from("src/lost.rs"), Some(10)),
        (PathBuf::from("src/big.bin"), Some(MAX_FILE_BYTES + 1)),
        (PathBuf::from("vendor/lib"), None),
    ];
    let runtime = Arc::new(scan_runtime());
    baseline_of(&project, &runtime, &fake);
    let input = pass_input(&runtime, "scan");
    let facts = input["files"]["modified"]["facts"].as_array().unwrap();
    let seen: Vec<(&str, bool, &str)> = facts
        .iter()
        .map(|f| {
            (
                f["path"].as_str().unwrap(),
                f["exists"].as_bool().unwrap(),
                f["error"].as_str().unwrap_or(""),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("src", true, "unreadable"),
            ("src/big.bin", true, "too_large"),
            ("src/lost.rs", true, "unreadable"),
            ("vendor/lib/x.rs", true, "unreadable"),
        ]
    );
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a compute pass at the baseline reads the commit's files"
)]
fn a_baseline_compiles_to_the_same_bytes_twice() {
    let project = gsd_facts_project();
    let runtime = Arc::new(scan_runtime());
    let env = project.environment_in(runtime.clone());
    let fake = commit(&project);
    let Located::Repo(place) = History::locate(&fake, project.root()) else {
        panic!("a repository")
    };
    let sha = Sha::new("c".repeat(40));
    let compile = || {
        let fetch = |keys: &[String]| {
            let asked: Vec<PathBuf> = keys.iter().map(PathBuf::from).collect();
            let found = History::blobs(&fake, &place, &sha, &asked).unwrap();
            keys.iter()
                .cloned()
                .zip(found.into_iter().map(|(_, bytes)| bytes))
                .collect::<Vec<_>>()
        };
        let files = History::files(&fake, &place, &sha, Path::new("")).unwrap();
        let tree = History::tree(&fake, &place, &sha, Path::new("")).unwrap();
        let baseline = Baseline::compile(&env, sha.clone(), Path::new(""), files, tree, &fetch);
        (
            serde_json::to_vec(&baseline).unwrap(),
            pass_input(&runtime, "scan"),
        )
    };
    let (first, first_input) = compile();
    let (second, second_input) = compile();
    assert_eq!(first, second);
    assert_eq!(
        first_input["files"]["modified"],
        second_input["files"]["modified"]
    );
}
