//! A human-only kind is written from a terminal and refused elsewhere (ADR 0289). The fixture is `@t/core`'s
//! `override`.

use super::execute_on;
use crate::outcome::OutputFormat;
use crate::terminal::{Facts, Terminal};
use specforge_ops::authoring::edit::Edit;
use specforge_project::testing::{GSD_WRITES_EXTENSIONS, GsdWrites, TestProject};
use specforge_test_macros::test as specforge_test;
use std::collections::BTreeMap;
use std::sync::Arc;

const TERMINAL: Facts = Facts {
    color_out: false,
    color_err: false,
    interactive: true,
};

fn new_override(facts: Facts) -> (bool, String, TestProject) {
    let project = TestProject::new()
        .config(|c| c["name"] = "p".into())
        .enabling(&GSD_WRITES_EXTENSIONS)
        .installing_enabled()
        .file("m.spec", "milestone v1.0 \"One\" {}\n");
    let compiled = project.compile_in(Arc::new(GsdWrites::new().runtime()));
    let edit = Edit::Create {
        kind: "override".into(),
        id: "O1".into(),
        title: "O".into(),
        file: "o.spec".into(),
        fields: BTreeMap::new(),
        verify: Vec::new(),
    };
    let (terminal, capture) = Terminal::captured("", facts);
    let _ = execute_on(
        &compiled,
        vec![edit],
        None,
        false,
        OutputFormat::Human,
        &terminal,
    );
    let wrote = project.root().join("o.spec").exists();
    (
        wrote,
        format!("{}{}", capture.stdout(), capture.stderr()),
        project,
    )
}

#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "the command line writes a human-only value from a terminal and refuses it without one"
)]
fn the_cli_writes_a_human_only_value_from_a_terminal() {
    let (wrote, err, _) = new_override(TERMINAL);
    assert!(wrote, "{err}");
    let (wrote, err, _) = new_override(Facts::PIPED);
    assert!(!wrote && err.contains("[human_only]"), "{err}");
}
