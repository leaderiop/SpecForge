//! The baseline reads a committed source file through the real git adapter, not the working copy (ADR 0300, behavior
//! `read_git_baseline`).

use std::process::Command;
use std::sync::Arc;

use serde_json::{Value, json};
use specforge_git::GitCli;
use specforge_project::baseline::Baseline;
use specforge_project::history::{History, Located};
use specforge_project::testing::{LIB_RS, gsd_facts_project, gsd_facts_serving, pass_input};
use specforge_test_macros::test as specforge_test;

fn facts<'a>(input: &'a Value, read: &str) -> &'a [Value] {
    input["files"][read]["facts"].as_array().unwrap()
}

#[ignore = "pins K19; T3 un-ignores"]
#[specforge_test(
    behavior = "read_git_baseline",
    verify = "the baseline reads a source file through git, and a link at the commit is unreadable"
)]
fn the_baseline_reads_a_source_file_through_git() {
    if Command::new("git").arg("--version").output().is_err() {
        return;
    }
    let project = gsd_facts_project();
    let root = project.root();
    #[cfg(unix)]
    std::os::unix::fs::symlink("lib.rs", root.join("src/link.rs")).unwrap();
    crate::history::git_repo(root);
    project.write("src/lib.rs", "pub fn two() {}\n");
    let runtime = Arc::new(gsd_facts_serving(
        json!([{"name": "scan", "phase": "compute", "baseline": ["plan"], "reads": [
            {"name": "modified", "field": "plan.files_modified", "text": true},
            {"name": "sources", "glob": "src/**/*.rs"},
        ]}]),
    ));
    let env = project.environment_in(runtime.clone());
    let git = GitCli::default();
    let Located::Repo(place) = git.locate(root) else {
        panic!("a repository")
    };
    let head = git.commit(&place, "HEAD").unwrap().expect("a commit");
    Baseline::at(&env, &git, &place, &head).expect("a baseline");
    let input = pass_input(&runtime, "scan");
    let lib = &facts(&input, "modified")[1];
    assert_eq!(lib["path"], "src/lib.rs");
    assert_eq!(lib["exists"], true, "{lib}");
    assert_eq!(lib["text"], LIB_RS, "the commit's text, not the edit");
    #[cfg(unix)]
    {
        let sources = facts(&input, "sources");
        assert_eq!(sources[0]["path"], "src/lib.rs");
        assert!(sources[0].get("error").is_none(), "{}", sources[0]);
        assert_eq!(sources[1]["path"], "src/link.rs");
        assert_eq!(sources[1]["exists"], true);
        assert_eq!(sources[1]["error"], "unreadable");
    }
}
