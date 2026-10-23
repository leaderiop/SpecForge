//! One file, one spelling: `check`, `format` and `export` name a source alike (ADR 0242).

use crate::support::{Cli, TestProject};
use specforge_test_macros::test as specforge_test;
use std::fs;

#[specforge_test(
    invariant = "document_project_paths",
    verify = "check, format and export name one file with one spelling"
)]
#[ignore = "flipped by T7"]
fn check_names_a_file_as_format_does() {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/coverage/fx1");
    let project = TestProject::copy_of(fixtures);
    fs::write(
        project.root().join("spec/broken.spec"),
        "behavior broken \"Broken\" {\n      category command\n  contract \"The system MUST work\"\n  produces [ghost_event]\n}\n",
    )
    .unwrap();
    let cli = Cli::on(&project);
    let check = cli.run(&["check", "--format", "json"]).json();
    let files: Vec<&str> = check
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["code"] == "E003")
        .filter_map(|d| d["span"]["file"].as_str())
        .collect();
    assert!(files.contains(&"spec/broken.spec"), "{check}");
    let format = cli.run(&["format", "--check", "--format", "json"]).json();
    assert!(
        format["changed_files"]
            .to_string()
            .contains("\"spec/broken.spec\""),
        "{format}"
    );
    let export = cli.run(&["export", "--format=graph", "--no-schema"]).stdout;
    assert!(
        export.contains("\"file\": \"spec/broken.spec\"")
            || export.contains("\"file\":\"spec/broken.spec\""),
        "{export}"
    );
    let human = cli.run(&["check"]);
    assert!(
        human.stderr.contains("spec/broken.spec:"),
        "{}",
        human.stderr
    );
}
