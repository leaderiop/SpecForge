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
    // A file over the cap is sized, never fetched; a file git cannot give is asked once.
    let asked = fake.blobs_paths.lock().unwrap().clone();
    assert!(!asked.contains(&PathBuf::from("src/big.bin")), "{asked:?}");
    assert!(asked.contains(&PathBuf::from("src/lost.rs")), "{asked:?}");
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
        let sizes = |keys: &[String]| {
            let asked: Vec<PathBuf> = keys.iter().map(PathBuf::from).collect();
            let found = History::sizes(&fake, &place, &sha, &asked).unwrap();
            keys.iter()
                .cloned()
                .zip(found.into_iter().map(|(_, size)| size))
                .collect::<Vec<_>>()
        };
        let files = History::files(&fake, &place, &sha, Path::new("")).unwrap();
        let tree = History::tree(&fake, &place, &sha, Path::new("")).unwrap();
        let baseline = Baseline::compile(
            &env,
            sha.clone(),
            Path::new(""),
            files,
            tree,
            &sizes,
            &fetch,
        );
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

/// `Baseline::at` of `fake`'s `HEAD` in `project`, as a result.
fn at(
    project: &TestProject,
    runtime: &Arc<InProcessRuntime>,
    fake: &FakeHistory,
) -> Result<(), String> {
    let env = project.environment_in(runtime.clone());
    let Located::Repo(place) = History::locate(fake, project.root()) else {
        panic!("a repository")
    };
    Baseline::at(&env, fake, &place, &Sha::new("c".repeat(40))).map(|_| ())
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a baseline git could not fully read is used and not cached, and a tree error is not cached"
)]
fn a_baseline_git_could_not_fully_read_is_not_cached() {
    let project = gsd_facts_project();
    let spec = project.read("a.spec").replace(
        "[\"src/lib.rs\", \"docs/gone.md\"]",
        "[\"src/lib.rs\", \"src/lost.rs\"]",
    );
    project.write("a.spec", &spec);
    let runtime = Arc::new(scan_runtime());
    let mut fake = commit(&project);
    // the tree lists a file whose blob git does not have (a blobless clone)
    fake.lost = vec![PathBuf::from("src/lost.rs")];
    at(&project, &runtime, &fake).expect("a baseline");
    at(&project, &runtime, &fake).expect("a baseline");
    assert_eq!(fake.trees_asked(), 2, "read again: the blob may come later");
    // a tree git could not list is an unreadable baseline, never an empty one
    let project = gsd_facts_project();
    let mut fake = commit(&project);
    fake.tree_fail = Some(specforge_project::history::HistoryError("no tree".into()));
    let why = at(&project, &runtime, &fake).expect_err("an unreadable baseline");
    assert!(why.contains("no tree"), "{why}");
    at(&project, &runtime, &fake).expect_err("still unreadable");
    assert_eq!(fake.trees_asked(), 2);
    // a complete baseline is read once
    let project = gsd_facts_project();
    let fake = commit(&project);
    at(&project, &runtime, &fake).expect("a baseline");
    at(&project, &runtime, &fake).expect("a baseline");
    assert_eq!(fake.trees_asked(), 1);
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a baseline whose listed file git then gives no bytes for is not cached"
)]
fn a_sized_file_without_bytes_makes_the_baseline_uncached() {
    let project = gsd_facts_project();
    let spec = project.read("a.spec").replace(
        "[\"src/lib.rs\", \"docs/gone.md\"]",
        "[\"src/lib.rs\", \"src/lost.rs\"]",
    );
    project.write("a.spec", &spec);
    let runtime = Arc::new(scan_runtime());
    let mut fake = commit(&project);
    // `sizes` answers, `blobs` then has nothing: the answer set is short at the end, not at the sizes
    fake.listed = vec![(PathBuf::from("src/lost.rs"), Some(10))];
    at(&project, &runtime, &fake).expect("a baseline");
    at(&project, &runtime, &fake).expect("a baseline");
    assert_eq!(fake.trees_asked(), 2, "read again: the blob may come later");
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a file git can never be asked for is permanently unreadable and the baseline is cached"
)]
fn a_name_git_cannot_be_asked_for_is_cached() {
    let project = gsd_facts_project();
    let spec = project.read("a.spec").replace(
        "[\"src/lib.rs\", \"docs/gone.md\"]",
        "[\"src/lib.rs\", \"src/new\\nline.rs\"]",
    );
    project.write("a.spec", &spec);
    let runtime = Arc::new(scan_runtime());
    let mut fake = commit(&project);
    fake.lost = vec![PathBuf::from("src/new\nline.rs")];
    at(&project, &runtime, &fake).expect("a baseline");
    at(&project, &runtime, &fake).expect("a baseline");
    assert_eq!(fake.trees_asked(), 1, "no later fetch can give that name");
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a spec a compute pass reads at the baseline is not asked of git again"
)]
fn a_spec_a_pass_reads_is_not_fetched_again() {
    let project = gsd_facts_project();
    let spec = project.read("a.spec").replace(
        "[\"src/lib.rs\", \"docs/gone.md\"]",
        "[\"src/lib.rs\", \"a.spec\"]",
    );
    project.write("a.spec", &spec);
    let runtime = Arc::new(scan_runtime());
    let mut fake = FakeHistory::repo(
        project.root(),
        &"c".repeat(40),
        vec![("a.spec".into(), spec.clone().into_bytes())],
    );
    // git could give it (it has a size), so only the seeding keeps it from being asked
    fake.blobs = vec![("a.spec".into(), spec.into_bytes())];
    baseline_of(&project, &runtime, &fake);
    let asked = fake.blobs_paths.lock().unwrap().clone();
    assert!(!asked.contains(&PathBuf::from("a.spec")), "{asked:?}");
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a project in a subdirectory of the repository reads its own files at the baseline"
)]
fn a_subdirectory_project_reads_its_own_files() {
    let project = gsd_facts_project();
    let top = project.root().parent().unwrap().to_path_buf();
    let dir = PathBuf::from(project.root().file_name().unwrap());
    let mut fake = FakeHistory::repo(
        &top,
        &"c".repeat(40),
        vec![(dir.join("a.spec"), project.read("a.spec").into_bytes())],
    );
    fake.blobs = vec![(dir.join("src/lib.rs"), b"pub fn old() {}\n".to_vec())];
    // another project's file at the same relative path is not ours
    fake.listed = vec![(PathBuf::from("other/src/lib.rs"), Some(1))];
    let runtime = Arc::new(scan_runtime());
    at(&project, &runtime, &fake).expect("a baseline");
    let input = pass_input(&runtime, "scan");
    let lib = &input["files"]["modified"]["facts"][1];
    assert_eq!(lib["path"], "src/lib.rs");
    assert_eq!(lib["text"], "pub fn old() {}\n", "{lib}");
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a path only a computed value names is read in a later run, and a short answer ends the loop"
)]
fn a_path_only_a_computed_value_names_is_read_in_a_later_run() {
    let project = gsd_facts_project();
    let wire = json!([
        {"name": "phase", "keyword": "phase", "fields": [{"name": "criteria", "field_type": "string_list"}]},
        {"name": "plan", "keyword": "plan", "fields": [
            {"name": "phase", "field_type": "reference", "target_kind": "phase"},
            {"name": "files_modified", "field_type": "string_list"},
            {"name": "touched", "field_type": "string_list", "writer": "computed"},
        ]},
    ]);
    let passes = json!([
        {"name": "first", "phase": "compute", "baseline": ["plan"], "computes": ["plan.touched"]},
        {"name": "second", "phase": "compute", "baseline": ["plan"], "reads": [
            {"name": "touched", "field": "plan.touched", "text": true},
        ]},
    ]);
    let none = json!({"diagnostics": [], "summary": {}, "values": []});
    let first = json!({"diagnostics": [], "summary": {}, "values": [
        {"entity": "pl1", "field": "touched", "value": ["src/lib.rs"]},
    ]});
    let ok = |value: &Value| specforge_wasm::WasmCallResult::Ok(value.to_string().into_bytes());
    let runtime = Arc::new(
        InProcessRuntime::new()
            .with(move || {
                let mut c = specforge_extension_sdk::prelude::ContributionsBuilder::new(
                    specforge_extension_sdk::prelude::ExtensionMeta::new("@t/facts", "1.0.0"),
                );
                c.raw_category(
                    specforge_extension_sdk::DeclaredCategory::Entities,
                    wire.clone(),
                );
                c.raw_category(
                    specforge_extension_sdk::DeclaredCategory::Passes,
                    passes.clone(),
                );
                c
            })
            .answer_raw("@t/facts", "__pass_first", ok(&first))
            .answer_raw("@t/facts", "__pass_second", ok(&none)),
    );
    let fake = commit(&project);
    at(&project, &runtime, &fake).expect("a baseline");
    let input = pass_input(&runtime, "second");
    let lib = &input["files"]["touched"]["facts"][0];
    assert_eq!(lib["path"], "src/lib.rs");
    assert_eq!(lib["text"], "pub fn old() {}\n", "{lib}");
    assert_eq!(fake.blobs_asked(), 1);
}

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "a path only a computed value names is read in a later run, and a short answer ends the loop"
)]
fn a_short_answer_from_git_ends_the_loop() {
    let project = gsd_facts_project();
    let runtime = Arc::new(scan_runtime());
    let env = project.environment_in(runtime.clone());
    let files = vec![("a.spec".into(), project.read("a.spec").into_bytes())];
    let tree = vec![("a.spec".into(), true), ("src/lib.rs".into(), true)];
    // sizes and bytes both answer nothing for the file asked: it reads unreadable, and the run ends
    Baseline::compile(
        &env,
        Sha::new("c".repeat(40)),
        Path::new(""),
        files,
        tree,
        &|_: &[String]| Vec::new(),
        &|_: &[String]| Vec::new(),
    );
    let input = pass_input(&runtime, "scan");
    assert_eq!(
        input["files"]["modified"]["facts"][1]["error"],
        "unreadable"
    );
}
