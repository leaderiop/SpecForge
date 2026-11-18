//! `specforge diff [<from>[..<to>]]` (ADR 0376 D5): the nodes and edges that changed between two snapshots of the
//! project, over a copy of the product-cycle fixture in a repository built with no git program.

use std::path::Path;

use specforge_git::testing::Repo;

use crate::support::{Cli, TestProject};

/// A third module, committed and then removed.
const OLD: &str = "module old_mod \"Old\" {\n  description \"Going\"\n}\n";
/// A module the working tree adds.
const NEW: &str = "module new_mod \"New\" {\n  description \"Coming\"\n}\n";

/// The fixture with [`OLD`] in `old.spec`, all committed: the project, its repository and the commit.
fn committed() -> (TestProject, Repo, String) {
    let project = TestProject::copy_of(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parity/product_cycle"),
    );
    project.write("old.spec", OLD);
    let repo = Repo::init(project.root());
    let sha = repo.commit_all("first");
    (project, repo, sha)
}

/// The working tree of [`committed`] with one module edited, one added and one removed.
fn edited(project: &TestProject) {
    let main = project.read("main.spec");
    project.write(
        "main.spec",
        main.replace("\"The core\"", "\"The new core\""),
    );
    project.write("new.spec", NEW);
    project.remove("old.spec");
}

/// `text` with each of `shas` written `<sha>`, so a golden holds no commit id.
fn redacted(text: &str, shas: &[&str]) -> String {
    shas.iter()
        .fold(text.to_string(), |text, sha| text.replace(sha, "<sha>"))
}

#[specforge_test_macros::test(requirement = "FEATURE-142")]
fn diff_text() {
    let (project, _repo, _) = committed();
    edited(&project);
    let run = Cli::on(&project).run(&["diff"]).exits(0);
    insta::assert_snapshot!("diff_text", run.stdout);
}

#[specforge_test_macros::test(requirement = "FEATURE-142")]
fn diff_json() {
    let (project, _repo, sha) = committed();
    edited(&project);
    let run = Cli::on(&project)
        .run(&["diff", "--format", "json"])
        .exits(0);
    let json = run.json();
    assert_eq!(json["from"]["name"], "HEAD");
    assert_eq!(json["from"]["key"], sha.as_str());
    assert_eq!(json["to"]["name"], "working tree");
    let pretty = serde_json::to_string_pretty(&json).unwrap();
    insta::assert_snapshot!("diff_json", redacted(&pretty, &[&sha]));
}

#[specforge_test_macros::test(requirement = "FEATURE-142")]
fn diff_of_a_clean_tree_is_empty() {
    let (project, _repo, sha) = committed();
    let cli = Cli::on(&project);
    assert_eq!(cli.run(&["diff"]).exits(0).stdout, "no change\n");
    assert_eq!(cli.run(&["diff", &sha]).exits(0).stdout, "no change\n");
    let json = cli.run(&["diff", "--format", "json"]).exits(0).json();
    assert_eq!(json["nodes"], serde_json::json!([]));
    assert_eq!(json["edges"], serde_json::json!([]));
}

#[specforge_test_macros::test(requirement = "FEATURE-142")]
fn diff_two_revisions() {
    let (project, repo, first) = committed();
    edited(&project);
    let second = repo.commit_all("second");
    let cli = Cli::on(&project);
    let range = format!("{first}..{second}");
    let between = cli.run(&["diff", &range]).exits(0).stdout;
    // the working tree is the second commit: the same change, named by two revisions
    assert_eq!(between, cli.run(&["diff", &first]).exits(0).stdout);
    assert!(between.contains("+ module new_mod\n"), "{between}");
    assert!(between.contains("- module old_mod\n"), "{between}");
    assert!(
        between.contains("~ module core_mod  fields: description\n"),
        "{between}"
    );
    let json = cli
        .run(&["diff", &range, "--format", "json"])
        .exits(0)
        .json();
    assert_eq!(json["from"]["key"], first.as_str());
    assert_eq!(json["to"]["key"], second.as_str());
    assert_eq!(json["to"]["name"], second.as_str());
    let back = cli
        .run(&["diff", &format!("{second}..{first}")])
        .exits(0)
        .stdout;
    assert!(back.contains("- module new_mod\n"), "{back}");
}

#[specforge_test_macros::test(requirement = "FEATURE-142")]
fn diff_unknown_revision() {
    let (project, _repo, _) = committed();
    let cli = Cli::on(&project);
    let run = cli.run(&["diff", "no-such-rev"]).exits(1);
    assert!(run.stderr.contains("unknown_revision"), "{}", run.stderr);
    assert!(
        run.stderr.contains("unknown revision 'no-such-rev'"),
        "{}",
        run.stderr
    );
    let json = cli
        .run(&["diff", "HEAD..nope", "--format", "json"])
        .exits(1);
    assert!(json.stdout.contains("unknown_revision"), "{}", json.stdout);
}

#[specforge_test_macros::test(requirement = "FEATURE-142")]
fn diff_symmetric_range() {
    let (project, _repo, _) = committed();
    let cli = Cli::on(&project);
    let run = cli.run(&["diff", "HEAD...HEAD"]).exits(2);
    assert!(run.stderr.contains("symmetric range"), "{}", run.stderr);
    let run = cli.run(&["diff", "HEAD.."]).exits(2);
    assert!(run.stderr.contains("names no revision"), "{}", run.stderr);
}

#[specforge_test_macros::test(requirement = "FEATURE-142")]
fn diff_outside_a_repository_is_refused() {
    let project = TestProject::copy_of(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parity/product_cycle"),
    );
    let run = Cli::on(&project).run(&["diff"]).exits(1);
    assert!(run.stderr.contains("not_a_repository"), "{}", run.stderr);
}

#[specforge_test_macros::test(requirement = "FEATURE-142")]
fn diff_keeps_the_kinds_named_and_counts_them() {
    let (project, _repo, _) = committed();
    edited(&project);
    let cli = Cli::on(&project);
    let all = cli.run(&["diff"]).exits(0).stdout;
    assert!(
        all.ends_with(
            "3 nodes changed (1 added, 1 removed, 1 modified), 0 edges (0 added, 0 removed)\n"
        ),
        "{all}"
    );
    let modules = cli.run(&["diff", "--kind", "module", "--kind", "feature"]);
    assert_eq!(modules.exits(0).stdout, all);
    let none = cli.run(&["diff", "--kind", "feature"]).exits(0);
    assert_eq!(none.stdout, "no change\n");
    let json = cli
        .run(&["diff", "--kind", "module", "--format", "json"])
        .exits(0)
        .json();
    assert_eq!(json["summary"]["nodes_added"], 1);
    assert_eq!(json["summary"]["by_kind"][0]["kind"], "module");
    assert_eq!(json["summary"]["by_kind"][0]["modified"], 1);
    let picked = cli
        .run(&["diff", "--pick", ".summary.nodes_removed"])
        .exits(0);
    assert_eq!(picked.stdout, "1\n");
}

#[specforge_test_macros::test(requirement = "FEATURE-142")]
fn diff_notes_a_revision_loading_another_extension_set() {
    let project = TestProject::copy_of(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parity/product_cycle"),
    );
    let current = project.read("specforge.json");
    // the revision loads one extension fewer than the working tree
    project.write(
        "specforge.json",
        current.replace(",\"@specforge/testing\"", ""),
    );
    let repo = Repo::init(project.root());
    let sha = repo.commit_all("fewer extensions");
    project.write("specforge.json", &current);
    let cli = Cli::on(&project);
    let run = cli.run(&["diff", "--format", "json"]).exits(0);
    let json = run.json();
    assert_eq!(json["notes"][0]["about"], "extensions_differ", "{json}");
    assert_eq!(json["notes"][0]["subject"], "@specforge/testing", "{json}");
    let pretty = serde_json::to_string_pretty(&json).unwrap();
    insta::assert_snapshot!(
        "diff_json_another_extension_set",
        redacted(&pretty, &[&sha])
    );
    let text = cli.run(&["diff"]).exits(0).stdout;
    assert!(
        text.contains("note (from): the extensions at HEAD differ from the current ones"),
        "{text}"
    );
}

/// A crate whose `rust-cargo` facts are collected, then committed with the ledger recording them (the store itself
/// is git-ignored), then collected again once its version moved: the project and the commit.
fn collected_twice() -> (TestProject, String) {
    let project = TestProject::new()
        .raw_config(
            r#"{"name":"demo","version":"0.1.0","extensions":["@code/core","@rust/cargo"]}"#,
        )
        .file(
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
        )
        .file("src/lib.rs", "")
        .file(".gitignore", ".specforge/\n");
    let repo = Repo::init(project.root());
    repo.commit_all("the crate");
    let cli = Cli::on(&project);
    let collect = [
        "collect",
        "--runner",
        "rust-cargo",
        "--yes",
        "--effects",
        "online",
        "--format",
        "json",
    ];
    let first = cli.run(&collect);
    assert_eq!(first.code, 0, "{}{}", first.stdout, first.stderr);
    let a = repo.commit_all("collected");
    project.write(
        "Cargo.toml",
        project.read("Cargo.toml").replace("0.1.0", "0.2.0"),
    );
    let second = cli.run(&collect);
    assert_eq!(second.code, 0, "{}{}", second.stdout, second.stderr);
    (project, a)
}

#[specforge_test_macros::test(requirement = "FEATURE-142")]
fn diff_reads_a_revisions_facts_from_the_store_a_later_collect_archived() {
    let (project, a) = collected_twice();
    let history = project.root().join(".specforge/facts-history");
    let archived = std::fs::read_dir(&history)
        .unwrap_or_else(|e| panic!("{}: {e}", history.display()))
        .count();
    assert_eq!(archived, 1, "the second collect archived the first store");
    let json = Cli::on(&project)
        .run(&["diff", &a, "--format", "json"])
        .exits(0)
        .json();
    assert_eq!(json["notes"], serde_json::json!([]), "{json}");
    let versions_moved = json["nodes"].as_array().unwrap().iter().any(|n| {
        n["change"] == "modified"
            && n["fields"]
                .as_array()
                .is_some_and(|f| f.iter().any(|f| f == "version"))
    });
    assert!(versions_moved, "the crate's version changed: {json}");
}
