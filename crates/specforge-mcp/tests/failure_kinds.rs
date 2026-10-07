//! What kind of failure MCP answers for an unusable test report, what
//! `specforge.format` and `specforge.migrate` return as their verdict, and
//! what they do with a directory that is no project (plan 02, ADR 0029).
//!
//! The first block was taken before ops decided any of it; the commit that
//! changes a pinned fact flips its pin and says so.

use crate::support::*;
use crate::tool_errors::mcp_error;
use serde_json::{Value, json};
use specforge_test::prelude::*;

const MAIN: &str = concat!(
    "behavior alpha \"Alpha\" {\n",
    "  contract \"The system MUST work\"\n",
    "}\n",
);

const MESSY: &str = "behavior messy \"Messy\" {\ncontract \"The system MUST work\"\n}\n";

/// A fresh served project holding [`MAIN`], `@specforge/software` enabled.
fn served() -> Served {
    TestProject::new()
        .enabling(&["@specforge/software"])
        .file("main.spec", MAIN)
        .serve_components()
}

/// Write the recorded report at the served project's root.
fn report(served: &Served, text: &str) {
    served.write("specforge-report.json", text);
}

/// Make the report unreadable; `false` when this process can still read it
/// (running as root).
#[cfg(unix)]
fn lock(served: &Served) -> bool {
    use std::os::unix::fs::PermissionsExt;
    report(served, r#"{"results":{}}"#);
    let path = served.root().join("specforge-report.json");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    std::fs::read(&path).is_err()
}

/// The `code` and the `diagnostic.code` of the refusal `tool` answers.
fn refusal(served: &mut Served, tool: &str, arguments: Value) -> (String, String) {
    let error = mcp_error(&call_tool(served, tool, arguments));
    (
        error["code"].as_str().unwrap().to_string(),
        error["diagnostic"]["code"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    )
}

/// Every tool that reads the recorded report, with arguments that reach it.
fn report_readers() -> Vec<(&'static str, Value)> {
    vec![
        ("specforge.stats", json!({})),
        ("specforge.coverage", json!({})),
        ("specforge.inspect", json!({"entity_id": "alpha"})),
        (
            "specforge.query",
            json!({"entity_id": "alpha", "include_coverage": true}),
        ),
        (
            "specforge.trace",
            json!({"plan": {"entries": [{"entity_id": "alpha"}]}}),
        ),
        ("specforge.analyze", json!({})),
    ]
}

#[test]
fn an_unparsable_report_refuses_every_view() {
    let mut served = served();
    report(&served, "{");

    for (tool, arguments) in report_readers() {
        let (code, diagnostic) = refusal(&mut served, tool, arguments);
        assert_eq!(
            (code.as_str(), diagnostic.as_str()),
            ("schema_mismatch", "E045"),
            "{tool}"
        );
    }
    for (prompt, arguments) in [
        ("review", json!({})),
        (
            "trace",
            json!({"plan": {"entries": [{"entity_id": "alpha"}]}}),
        ),
    ] {
        let reply = get_prompt(
            &mut served,
            &format!("specforge://prompts/{prompt}"),
            arguments,
        );
        let data = &reply["error"]["data"];
        assert_eq!(data["code"], "schema_mismatch", "{prompt}: {reply}");
        assert_eq!(data["diagnostic"]["code"], "E045", "{prompt}: {reply}");
    }
}

#[cfg(unix)]
#[specforge_test(
    behavior = "provide_mcp_coverage_tool",
    verify = "a report the OS refuses to read is a permission_denied error on every tool that reads it"
)]
fn a_locked_report_is_permission_denied_on_every_tool() {
    let mut served = served();
    if !lock(&served) {
        return;
    }

    for (tool, arguments) in report_readers() {
        let (code, diagnostic) = refusal(&mut served, tool, arguments);
        assert_eq!(
            (code.as_str(), diagnostic.as_str()),
            ("permission_denied", "E045"),
            "{tool}"
        );
    }
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "a test_results file that does not exist is a file_not_found error"
)]
fn a_missing_named_report_is_file_not_found() {
    let mut served = served();
    let missing = served.root().join("none.json");

    let (code, diagnostic) = refusal(
        &mut served,
        "specforge.analyze",
        json!({"test_results": missing.to_str().unwrap()}),
    );

    assert_eq!(
        (code.as_str(), diagnostic.as_str()),
        ("file_not_found", "E045")
    );
}

#[specforge_test(
    behavior = "provide_mcp_analyze_tool",
    verify = "a relative test_results names a file under the project's root"
)]
fn a_relative_test_results_is_under_the_project_root() {
    let mut served = served();
    served.write("r.json", r#"{"results":{}}"#);

    let response = call_tool(
        &mut served,
        "specforge.analyze",
        json!({"test_results": "r.json"}),
    );

    // The test process's cwd is the crate directory, not the project.
    assert_ne!(response["result"]["isError"], true, "{response}");
}

#[test]
fn format_returns_no_verdict() {
    let mut check = served();
    check.write("messy.spec", MESSY);
    let checked = tool(&mut check, "specforge.format", json!({"check": true}));
    assert_eq!(checked["all_clean"], false, "{checked}");
    assert!(checked.get("ok").is_none(), "{checked}");

    let mut write = served();
    write.write("messy.spec", MESSY);
    let written = tool(&mut write, "specforge.format", json!({}));
    assert_eq!(written["all_clean"], false, "{written}");
    assert!(written.get("ok").is_none(), "{written}");
}

/// A directory with one unformatted `a.spec` and no project config.
fn loose(text: &str, name: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(name), text).unwrap();
    dir
}

#[test]
fn format_refuses_a_directory_that_is_no_project() {
    let mut served = served();
    let dir = loose(MESSY, "a.spec");

    let response = call_tool(
        &mut served,
        "specforge.format",
        json!({"path": dir.path().to_str().unwrap(), "check": true}),
    );

    assert_eq!(mcp_error(&response)["code"], "precondition_failed");
}

#[test]
fn migrate_refuses_a_directory_that_is_no_project() {
    let mut served = served();
    let dir = loose(
        "// specforge-format: 0.1\nbehavior old_one \"Old\" {\n}\n",
        "old.spec",
    );

    let response = call_tool(
        &mut served,
        "specforge.migrate",
        json!({"path": dir.path().to_str().unwrap(), "dry_run": true}),
    );

    assert_eq!(mcp_error(&response)["code"], "precondition_failed");
}

#[test]
fn a_migration_that_fails_is_an_internal_error() {
    let mut served = served();
    served.write(
        "bad.spec",
        "// specforge-format: 99.0\nbehavior bad \"Bad\" {\n}\n",
    );

    let error = mcp_error(&call_tool(&mut served, "specforge.migrate", json!({})));

    assert_eq!(error["code"], "internal_error", "{error}");
    assert_eq!(error["message"], "the migration failed", "{error}");
    assert_eq!(error["data"]["files_failed"], 1, "{error}");
    assert!(error["data"].get("ok").is_none(), "{error}");
}
