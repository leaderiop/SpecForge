//! A project's starter is checked by the extensions `init` enables (ADR 0189).

use crate::support::{Cli, TestProject};
use specforge_test_macros::test as specforge_test;

#[specforge_test(
    behavior = "non_interactive_init",
    verify = "a directory named after a kind init enables is refused as the project name, writing nothing"
)]
fn a_directory_named_after_a_kind_init_enables_is_refused_writing_nothing() {
    let dir = TestProject::new().without_config();
    let release = dir.root().join("release");
    std::fs::create_dir_all(&release).unwrap();

    Cli::on(&dir)
        .bare_command(&[
            "init",
            "--extensions",
            "@specforge/product,@specforge/software",
        ])
        .current_dir(&release)
        .assert()
        .code(1)
        .stderr(predicates::str::contains("invalid project name 'release'"));
    assert_eq!(std::fs::read_dir(&release).unwrap().count(), 0);
}

#[specforge_test(
    behavior = "non_interactive_init",
    verify = "an init whose version holds a quote or a backslash writes a starter that passes format --check and check"
)]
fn an_init_whose_version_holds_a_quote_formats_and_checks_clean() {
    let dir = TestProject::new().without_config();
    Cli::on(&dir)
        .bare_command(&[])
        .args([
            "init",
            "--name",
            "quoted",
            "--version",
            "1.0\" oops {\\q",
            "--extensions",
            "@specforge/software",
        ])
        .current_dir(dir.root())
        .assert()
        .success();
    for command in [&["format", "--check"][..], &["check"][..]] {
        let run = Cli::on(&dir)
            .bare_command(&[])
            .args(command)
            .current_dir(dir.root())
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "{command:?}:\n{}{}",
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr),
        );
    }
}

#[specforge_test(
    behavior = "scaffold_starter_spec_file",
    verify = "the product starter's journey flow is a list of steps"
)]
#[ignore = "flipped by T4"]
fn the_product_starters_flow_is_a_list_of_steps() {
    let starter = include_str!("../../../../extensions/product/src/starter.spec")
        .replace("{project}", "demo")
        .replace("{version}", "0.1.0");
    let dir = TestProject::new()
        .enabling(&["@specforge/product"])
        .config(|c| c["spec_root"] = "spec".into())
        .file("spec/demo.spec", &starter);
    let cli = Cli::on(&dir);
    let export = cli.run(&["export", "--format", "json"]).exits(0).json();
    let text = export.to_string();
    let flow = find_flow(&export).unwrap_or_else(|| panic!("no onboarding flow in {text}"));
    let steps = flow.as_array().expect("flow is a list");
    assert_eq!(steps.len(), 2, "{flow}");
    assert!(
        steps.iter().all(|s| !s.as_str().unwrap().contains('\n')),
        "{flow}"
    );
    let w159 = cli
        .document(&["check"])
        .json
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["code"] == "W159")
        .count();
    assert_eq!(w159, 0);
}

/// The `flow` of the node `onboarding`, wherever the export puts it.
fn find_flow(v: &serde_json::Value) -> Option<&serde_json::Value> {
    match v {
        serde_json::Value::Object(m) => {
            let is_it = m.get("id").and_then(|i| i.as_str()) == Some("onboarding");
            let flow = m
                .get("flow")
                .or_else(|| m.get("fields").and_then(|f| f.get("flow")));
            if let (true, Some(f)) = (is_it, flow) {
                return Some(f);
            }
            m.values().find_map(find_flow)
        }
        serde_json::Value::Array(a) => a.iter().find_map(find_flow),
        _ => None,
    }
}
