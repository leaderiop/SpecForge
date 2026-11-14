//! `specforge context <route> <subject>` (ADR 0356) over a builtin project, whose extensions declare no context route.

use crate::support::{Cli, TestProject};

#[test]
fn a_context_route_no_extension_declares_is_refused() {
    let project = TestProject::new()
        .raw_config(r#"{"name":"t","version":"0.1.0","extensions":["@specforge/software"]}"#);
    project.write(
        "app.spec",
        "behavior save \"Save\" {\n  contract \"The system MUST save\"\n}\n",
    );
    let run = Cli::on(&project)
        .run(&["context", "acme/reader", "save"])
        .exits(1);
    let error = run.json();
    assert_eq!(error["code"], "unknown_route", "{error}");
    assert!(
        error["error"]
            .as_str()
            .unwrap()
            .contains("no loaded extension declares a context route"),
        "{error}"
    );
}
