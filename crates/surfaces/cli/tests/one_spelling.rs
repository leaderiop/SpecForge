//! One file, one spelling: `check`, `format` and `export` name a source alike (ADR 0242).

use crate::support::{Cli, TestProject};
use specforge_test_macros::test as specforge_test;
use std::fs;

#[specforge_test(
    invariant = "document_project_paths",
    verify = "check, format and export name one file with one spelling"
)]
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

#[cfg(unix)]
#[specforge_test(
    behavior = "create_entity",
    verify = "new writes into a spec root outside the project by the file check names"
)]
fn new_writes_into_an_outside_spec_root_by_the_file_check_names() {
    let outside = tempfile::TempDir::new().unwrap();
    let outside_root = fs::canonicalize(outside.path()).unwrap();
    fs::write(
        outside_root.join("task.spec"),
        "behavior base \"Base\" {\n  category command\n  contract \"The system MUST base\"\n  produces [ghost_event]\n}\n",
    )
    .unwrap();
    let project = TestProject::new().raw_config(
        serde_json::json!({"name": "t", "spec_root": outside_root, "extensions": ["@specforge/software"]}).to_string(),
    );
    let cli = Cli::on(&project);
    let check = cli.run(&["check", "--format", "json"]).json();
    let file = check
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "E003")
        .and_then(|d| d["span"]["file"].as_str())
        .unwrap_or_else(|| panic!("{check}"))
        .to_owned();
    cli.run(&[
        "new",
        "behavior",
        "foo_it",
        "Foo it",
        "--in",
        &file,
        "--contract",
        "The system MUST foo.",
    ])
    .exits(0);
    assert!(
        fs::read_to_string(outside_root.join("task.spec"))
            .unwrap()
            .contains("behavior foo_it")
    );
    // Spelled from the project root: both temp directories lie in one parent.
    let relative = format!(
        "../{}/task.spec",
        outside_root.file_name().unwrap().to_str().unwrap()
    );
    cli.run(&[
        "new",
        "behavior",
        "bar_it",
        "Bar it",
        "--in",
        &relative,
        "--contract",
        "The system MUST bar.",
    ])
    .exits(0);
    assert!(
        fs::read_to_string(outside_root.join("task.spec"))
            .unwrap()
            .contains("behavior bar_it")
    );
}
