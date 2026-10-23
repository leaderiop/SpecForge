//! The call target takes its own arguments (ADR 0239): an extension command
//! or explicit tool given its own project's `path`, or a null one, runs as it
//! does without one; another project's `path` is still the target's refusal.

use crate::fake_extension::{self, FakeExtension};
use crate::support::*;
use crate::tool_errors::mcp_error;
use serde_json::{Value, json};
use specforge_test::prelude::*;

#[specforge_test(
    behavior = "read_mcp_arguments_as_declared",
    verify = "an extension command given its own project's path, or a null one, runs as it does without one, and another project's path is the target's refusal"
)]
#[ignore = "flipped by T4"]
fn an_extension_command_given_its_projects_path_runs_as_without_it() {
    let fake = FakeExtension::new().with_output(
        "cmd__report",
        json!({"exit_code": 0, "stdout": "{}", "stderr": ""}),
    );
    let (mut server, ext, dir) = fake_extension::initialized(fake);
    let root = dir.path().to_str().unwrap().to_string();
    let resp = call_tool(&mut server, "specforge.cmds.report", json!({"style": "md"}));
    assert_eq!(resp["result"]["isError"], false, "{resp}");
    let expected = ext.calls()[0].2["args"].clone();
    for path in [json!("."), json!(root), Value::Null] {
        let resp = call_tool(
            &mut server,
            "specforge.cmds.report",
            json!({"style": "md", "path": path}),
        );
        assert_eq!(resp["result"]["isError"], false, "{path}: {resp}");
        assert_eq!(ext.calls().last().unwrap().2["args"], expected, "{path}");
    }
    // Another root is still the target's refusal, before any export runs.
    let before = ext.calls().len();
    let resp = call_tool(
        &mut server,
        "specforge.cmds.report",
        json!({"style": "md", "path": "/"}),
    );
    let error = mcp_error(&resp);
    assert_eq!(
        (error["code"].as_str(), error["argument"].as_str()),
        (Some("invalid_input"), Some("path")),
        "{error}"
    );
    assert_eq!(ext.calls().len(), before);
}

#[specforge_test(
    behavior = "read_mcp_arguments_as_declared",
    verify = "an explicit extension tool's export and its schema check are never handed the call target's path"
)]
#[ignore = "flipped by T4"]
fn an_explicit_extension_tool_is_never_handed_the_targets_path() {
    let closed = json!({"name": "test.closed_in", "description": "Closed input",
        "export": "mcp__closed_in",
        "input_schema": {"type": "object", "properties": {"strict": {"type": "boolean"}},
                         "additionalProperties": false}});
    let fake = FakeExtension::new()
        .with_tool(closed)
        .with_output("mcp__check", json!({"checked": true}))
        .with_output("mcp__closed_in", json!({}));
    let (mut server, ext, dir) = fake_extension::initialized(fake);
    let root = dir.path().to_str().unwrap().to_string();
    for tool in ["specforge.cmds.check", "test.closed_in"] {
        for path in [json!("."), json!(root), Value::Null] {
            let before = ext.calls().len();
            let resp = call_tool(&mut server, tool, json!({"strict": true, "path": path}));
            assert_eq!(resp["result"]["isError"], false, "{tool} {path}: {resp}");
            assert_eq!(
                ext.calls()[before].2,
                json!({"strict": true}),
                "{tool} {path}"
            );
        }
    }
}
