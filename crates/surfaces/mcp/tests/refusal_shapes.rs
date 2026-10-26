//! A command tool refuses in the one shape every tool refuses in (ADR 0252): an
//! `isError` result carrying an `McpError` that names the tool.

use crate::fake_extension::{self, FakeExtension};
use crate::support::*;
use crate::tool_errors::mcp_error;
use serde_json::{Value, json};
use specforge_test::prelude::*;

const REPORT: &str = "specforge.cmds.report";

/// A server whose `report` command answers `output`.
fn answering(output: Value) -> (Logged, std::sync::Arc<FakeExtension>, tempfile::TempDir) {
    fake_extension::initialized(FakeExtension::new().with_output("cmd__report", output))
}

/// The `McpError` of a failed `report` run that wrote `stdout` and `stderr`.
fn failed_run(exit_code: i32, stdout: &str, stderr: &str) -> (Value, Logged) {
    let (mut server, _ext, _dir) =
        answering(json!({"exit_code": exit_code, "stdout": stdout, "stderr": stderr}));
    let resp = call_tool(&mut server, REPORT, json!({"style": "md"}));
    (mcp_error(&resp), server)
}

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "over MCP an argument the command's declaration refuses is invalid_input on that argument, with the message the CLI writes (a missing one in the sentence every MCP tool uses), and the export is not called"
)]
fn a_commands_refused_argument_is_invalid_input_on_that_argument() {
    let (mut server, ext, _dir) = fake_extension::initialized(FakeExtension::new().with_command(
        json!({"id": "pair", "title": "Pair", "description": "Two required args",
            "export": "cmd__pair", "args": [
                {"name": "a", "arg_type": "string", "required": true},
                {"name": "b", "arg_type": "string", "required": true}]}),
    ));
    for (tool, arguments, argument, message, suggestion) in [
        (
            REPORT,
            json!({}),
            "style",
            "Missing required parameter: style",
            None,
        ),
        (
            REPORT,
            json!({"style": "xml"}),
            "style",
            "style must be one of md, json, got 'xml'",
            None,
        ),
        (
            REPORT,
            json!({"style": "jsno"}),
            "style",
            "style must be one of md, json, got 'jsno'",
            Some("did you mean 'json'?"),
        ),
        (
            REPORT,
            json!({"style": "md", "limit": "x"}),
            "limit",
            "limit must be an integer, got 'x'",
            None,
        ),
        (
            REPORT,
            json!({"style": "md", "bogus": 1}),
            "bogus",
            "unknown argument 'bogus'",
            None,
        ),
        (
            REPORT,
            json!({"style": "md", "verbos": true}),
            "verbos",
            "unknown argument 'verbos'",
            Some("did you mean 'verbose'?"),
        ),
        (
            "specforge.cmds.pair",
            json!({}),
            "a",
            "Missing required parameter: a",
            None,
        ),
    ] {
        let resp = call_tool(&mut server, tool, arguments.clone());
        let error = mcp_error(&resp);
        assert_eq!(
            (
                &error["code"],
                &error["argument"],
                &error["message"],
                &error["tool"]
            ),
            (
                &json!("invalid_input"),
                &json!(argument),
                &json!(message),
                &json!(tool)
            ),
            "{arguments}: {error}"
        );
        assert_eq!(
            error.pointer("/data/suggestion").and_then(Value::as_str),
            suggestion,
            "{arguments}"
        );
        assert_eq!(resp["result"]["structuredContent"], error, "{arguments}");
    }
    assert!(ext.calls().is_empty(), "no export ran: {:?}", ext.calls());
    assert!(events(&server, "surface_command_dispatched").is_empty());
}

#[specforge_test(
    behavior = "read_mcp_arguments_as_declared",
    verify = "a missing required argument is refused in one sentence on every tool, a core tool's and an extension command's alike"
)]
fn a_missing_argument_is_refused_in_one_sentence_on_every_tool() {
    let (mut server, _ext, _dir) = fake_extension::initialized(FakeExtension::new().with_command(
        json!({"id": "pair", "title": "Pair", "description": "Two required args",
            "export": "cmd__pair", "args": [
                {"name": "a", "arg_type": "string", "required": true},
                {"name": "b", "arg_type": "string", "required": true}]}),
    ));
    for (tool, argument) in [
        ("specforge.inspect", "entity_id"),
        (REPORT, "style"),
        ("specforge.cmds.pair", "a"),
    ] {
        let error = mcp_error(&call_tool(&mut server, tool, json!({})));
        assert_eq!(
            (&error["code"], &error["argument"], &error["message"]),
            (
                &json!("invalid_input"),
                &json!(argument),
                &json!(format!("Missing required parameter: {argument}"))
            ),
            "{tool}: {error}"
        );
    }
}

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "over MCP a command's own error object is the McpError its code names, its entity and suggestion kept"
)]
fn a_commands_own_error_object_is_the_mcp_error_its_code_names() {
    let not_found = json!({"code": "ENTITY_NOT_FOUND", "message": "milestone 'm2' not found",
        "entity_id": "m2", "suggestion": "m1"});
    let (error, server) = failed_run(1, "", &not_found.to_string());
    assert_eq!(error["code"], "entity_not_found", "{error}");
    assert_eq!(error["message"], "milestone 'm2' not found");
    assert_eq!(error["entity_id"], "m2");
    assert_eq!(error["data"]["suggestion"], "did you mean 'm1'?");
    assert_eq!(error["tool"], REPORT);
    assert!(error.get("argument").is_none(), "{error}");
    assert_eq!(events(&server, "surface_command_dispatched").len(), 1);

    let bad = json!({"code": "INVALID_INPUT", "message": "bad date"});
    let (error, _) = failed_run(2, "", &bad.to_string());
    assert_eq!(
        (&error["code"], &error["message"]),
        (&json!("invalid_input"), &json!("bad date"))
    );

    let other = json!({"code": "OVERDUE", "message": "late"});
    let (error, _) = failed_run(1, "", &other.to_string());
    assert_eq!(error["code"], "internal_error", "{error}");
    assert_eq!(error["data"]["code"], "OVERDUE");
}

#[specforge_test(
    behavior = "dispatch_surface_command",
    verify = "over MCP a command that fails without one error object is internal_error carrying its exit code, stdout and stderr"
)]
fn a_command_that_fails_without_an_error_object_is_internal_error_with_its_output() {
    let (error, _) = failed_run(2, "", "no tests found");
    assert_eq!(error["code"], "internal_error", "{error}");
    assert_eq!(
        error["message"],
        "command cmd__report() of '@test/cmds' exited 2: no tests found"
    );
    assert_eq!(
        error["data"],
        json!({"exit_code": 2, "stdout": "", "stderr": "no tests found"})
    );

    let object = json!({"code": "INVALID_INPUT", "message": "bad"}).to_string();
    let (error, _) = failed_run(1, "partial", &object);
    assert_eq!(error["code"], "internal_error", "{error}");
    assert_eq!(error["data"]["stdout"], "partial");

    let (error, _) = failed_run(2, "", "[\"a\"]");
    assert_eq!(error["code"], "internal_error", "{error}");
}

#[specforge_test(
    invariant = "mcp_structured_error_responses",
    verify = "every refusal of every MCP tool kind is an isError result carrying one McpError that names the tool"
)]
fn every_tool_kind_refuses_with_an_mcp_error() {
    // The twelve slugs of `McpErrorCode` (spec/types/mcp.spec).
    const CODES: [&str; 12] = [
        "invalid_input",
        "compilation_failed",
        "entity_not_found",
        "file_not_found",
        "extension_not_found",
        "permission_denied",
        "timeout",
        "not_initialized",
        "schema_mismatch",
        "internal_error",
        "conflict",
        "precondition_failed",
    ];
    let object = json!({"code": "ENTITY_NOT_FOUND", "message": "gone"}).to_string();
    for (label, ext, tool, arguments) in [
        (
            "core, missing",
            FakeExtension::new(),
            "specforge.query",
            json!({}),
        ),
        (
            "core, unknown entity",
            FakeExtension::new(),
            "specforge.query",
            json!({"entity_id": "nope"}),
        ),
        (
            "command, target path",
            FakeExtension::new(),
            REPORT,
            json!({"style": "md", "path": 5}),
        ),
        ("command, arg rule", FakeExtension::new(), REPORT, json!({})),
        (
            "command, own object",
            FakeExtension::new().with_output(
                "cmd__report",
                json!({"exit_code": 1, "stdout": "", "stderr": object}),
            ),
            REPORT,
            json!({"style": "md"}),
        ),
        (
            "command, prose",
            FakeExtension::new().with_output(
                "cmd__report",
                json!({"exit_code": 2, "stdout": "", "stderr": "no tests found"}),
            ),
            REPORT,
            json!({"style": "md"}),
        ),
        (
            "command, export that does not answer",
            FakeExtension::new(),
            REPORT,
            json!({"style": "md"}),
        ),
        (
            "explicit tool, schema",
            FakeExtension::new(),
            "specforge.cmds.check",
            json!({"strict": "yes"}),
        ),
        (
            "explicit tool, export that does not answer",
            FakeExtension::new(),
            "specforge.cmds.check",
            json!({}),
        ),
    ] {
        let (mut server, _ext, _dir) = fake_extension::initialized(ext);
        let error = mcp_error(&call_tool(&mut server, tool, arguments));
        let code = error["code"].as_str().unwrap();
        assert!(CODES.contains(&code), "{label}: {error}");
        assert_eq!(error["tool"], tool, "{label}: {error}");
    }
}
