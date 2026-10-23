//! What `format` names and refuses on its command line (ADR 0242): a path that names nothing, the name of a
//! file the run found, and `--stdin` beside paths.

use crate::support::{Cli, TestProject};
use specforge_test_macros::test as specforge_test;
use std::fs;

const MESSY: &str = "behavior bar \"Bar\" {\ncontract \"The system MUST work\"\n}\n";

fn project() -> TestProject {
    let project = TestProject::new().raw_config(r#"{"name":"test","version":"0.1.0"}"#);
    fs::create_dir_all(project.root().join("spec")).unwrap();
    fs::write(project.root().join("spec/bad.spec"), MESSY).unwrap();
    project
}

#[specforge_test(
    behavior = "discover_format_targets",
    verify = "format refuses a path that names nothing and exits 1, so a mistyped directory fails a check"
)]
fn format_refuses_a_path_that_names_nothing() {
    let project = project();
    let cli = Cli::on(&project);
    let run = cli.run(&["format", "--check", "more.spec"]).exits(1);
    assert!(
        run.stderr
            .contains("error[file_not_found]: 'more.spec' names no file or directory"),
        "{}",
        run.stderr
    );
    assert!(!run.stderr.contains("hint:"), "{}", run.stderr);
    cli.run(&["format", "--check", "nodir/"]).exits(1);
    let json = cli.document(&["format", "--check", "more.spec"]);
    assert_eq!(json.code, 1);
    assert_eq!(json.json["code"], "file_not_found", "{}", json.json);
}

#[specforge_test(
    behavior = "format_spec_files",
    verify = "with no path typed, the human output names each file it found from the project root"
)]
fn format_names_a_file_it_found_from_the_project_root() {
    let project = project();
    let cli = Cli::on(&project);
    assert_eq!(
        cli.run(&["format", "--check"]).exits(1).stdout,
        "spec/bad.spec\n"
    );
    let diff = cli.run(&["format", "--diff"]);
    assert_eq!(
        diff.stdout.lines().next(),
        Some("--- spec/bad.spec"),
        "{}",
        diff.stdout
    );
    let typed = cli.run(&["format", "--check", "spec/bad.spec"]).exits(1);
    assert_eq!(typed.stdout, "spec/bad.spec\n");
}

#[specforge_test(
    behavior = "format_from_stdin",
    verify = "--stdin refuses a path, --check and --diff as a usage error"
)]
fn stdin_refuses_a_path_check_and_diff() {
    let project = project();
    let cli = Cli::on(&project).stdin(MESSY);
    for extra in ["spec/hello.spec", "--check", "--diff"] {
        let run = cli.run(&["format", "--stdin", extra]).exits(2);
        assert_eq!(run.stdout, "", "{extra}");
    }
}
