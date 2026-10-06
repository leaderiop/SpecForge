//! The operational payloads (`specforge_protocol_types::calls`): each is
//! one type the host and the SDK share, and the host's bytes, pinned in
//! `crates/specforge-wasm/tests/wire/`, decode as it.

use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use specforge_protocol_types::{
    CollectInput, CommandError, CommandFormat, CommandInput, CommandOutput, GraphWire,
    McpResourceContent, McpResourceRequest, MigrationInput, PassAnswer, PassDiagnostic, PassInput,
    PassOutput, PassSeverity, ScanRequest, ValidatorContext,
};

fn wire(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../specforge-wasm/tests/wire")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

/// `value` without its `null` members: an absent optional field and a
/// `null` one read alike, and the protocol writes neither.
fn without_nulls(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), without_nulls(v)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(without_nulls).collect()),
        other => other.clone(),
    }
}

/// `golden` decodes as `T`, and `T` encodes back to it (nulls aside).
fn round_trips<T: DeserializeOwned + Serialize>(golden: &Value) -> T {
    let decoded: T = serde_json::from_value(golden.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&decoded).unwrap(),
        without_nulls(golden)
    );
    decoded
}

#[specforge_test_macros::test(
    behavior = "call_extension_exports",
    verify = "every operational payload is one protocol type the host and the SDK share"
)]
fn the_host_payloads_decode_as_their_protocol_types() {
    // The command input decodes with the graph export's extra keys
    // (`format_version`, a node's `file` and `line`) ignored.
    let command = wire("command.input.json");
    let input: CommandInput<GraphWire> = serde_json::from_value(command.clone()).unwrap();
    assert_eq!(input.args, *command["args"].as_object().unwrap());
    assert_eq!(
        (input.cwd.as_str(), input.format, input.today.as_str()),
        ("/p", CommandFormat::Json, "2026-10-03")
    );
    assert_eq!(input.graph.nodes[0].id, "f1");
    assert_eq!(input.graph.nodes[0].title.as_deref(), Some("One"));

    round_trips::<McpResourceRequest>(&wire("resource.input.json"));
    // The analysis input decodes with the report's keys the protocol does
    // not define (a result's `file`, a test's `duration_ms` and `runner`)
    // ignored.
    let pass: PassInput = serde_json::from_value(wire("pass.input.json")).unwrap();
    assert_eq!(pass.entities.len(), 3);
    assert_eq!(pass.edges[0].label, "needs");
    assert_eq!(
        pass.proved_claims.as_deref(),
        Some(&["a".into(), "b".into()][..])
    );
    let tests = &pass.test_results.unwrap().results["a"].tests;
    assert_eq!(tests[0].verify.as_deref(), Some("a works"));
    let check: PassInput = round_trips(&wire("pass.check.input.json"));
    assert_eq!(check.previous.unwrap().statuses["a"].status, "draft");
    let collect: CollectInput = round_trips(&wire("collect.input.json"));
    assert_eq!(collect.stdout, None);
    let Value::Array(contexts) = wire("validate.input.json") else {
        panic!("validate.input.json is a list of contexts");
    };
    for context in &contexts {
        round_trips::<ValidatorContext>(context);
    }
    round_trips::<ScanRequest>(&wire("scan.input.json"));
    round_trips::<MigrationInput>(&wire("migrate.input.json"));
}

#[specforge_test_macros::test(
    behavior = "run_check_phase_passes",
    verify = "a pass entity carries whether the host found it exempt"
)]
fn a_pass_entity_carries_its_exemption() {
    let input: PassInput = serde_json::from_value(wire("pass.input.json")).unwrap();
    let exempt: Vec<(&str, bool)> = input
        .entities
        .iter()
        .map(|e| (e.id.as_str(), e.exempt))
        .collect();
    assert_eq!(exempt, [("a", false), ("b", false), ("c", true)]);
    // An older host's entity, without the key, owes obligations.
    let older: PassInput =
        serde_json::from_value(json!({"entities": [{"id": "a", "kind": "k"}]})).unwrap();
    assert!(!older.entities[0].exempt);
}

#[test]
fn a_command_input_carries_the_format_and_the_date() {
    let bare: CommandInput = serde_json::from_value(json!({})).unwrap();
    assert_eq!(bare.format, CommandFormat::Human);
    assert!(!bare.is_json());
    assert_eq!(bare.today, "");
    let input: CommandInput =
        serde_json::from_value(json!({"format": "json", "today": "2026-10-03"})).unwrap();
    assert!(input.is_json());
    assert_eq!(input.today, "2026-10-03");
    assert_eq!(CommandFormat::parse("json"), Some(CommandFormat::Json));
    assert_eq!(CommandFormat::parse("table"), None);
}

#[test]
fn a_command_error_is_an_object_under_json_and_a_line_under_human() {
    let error = CommandError {
        entity_id: Some("m2".into()),
        suggestion: Some("m1".into()),
        ..CommandError::new("ENTITY_NOT_FOUND", "milestone 'm2' not found")
    };
    let json = CommandOutput::error(CommandFormat::Json, &error, 1);
    assert_eq!((json.exit_code, json.stdout.as_str()), (1, ""));
    assert_eq!(
        serde_json::from_str::<Value>(&json.stderr).unwrap(),
        json!({"code": "ENTITY_NOT_FOUND", "message": "milestone 'm2' not found",
            "entity_id": "m2", "suggestion": "m1"})
    );
    let human = CommandOutput::error(CommandFormat::Human, &error, 1);
    assert_eq!(
        human.stderr,
        "error: milestone 'm2' not found\ndid you mean 'm1'?\n"
    );
    let plain = CommandOutput::error(
        CommandFormat::Human,
        &CommandError::new("INVALID_INPUT", "bad"),
        2,
    );
    assert_eq!(
        (plain.exit_code, plain.stderr.as_str()),
        (2, "error: bad\n")
    );
}

#[test]
fn a_command_output_is_the_wire_shape_the_host_reads() {
    let out: Value = serde_json::from_slice(&CommandOutput::fail("nope\n").to_bytes()).unwrap();
    assert_eq!(
        out,
        json!({"exit_code": 1, "stdout": "", "stderr": "nope\n"})
    );
    // `exit_code` is required.
    assert!(serde_json::from_value::<CommandOutput>(json!({"stdout": "x"})).is_err());
}

#[test]
fn a_resource_answer_is_its_content_and_mime_type() {
    let content = McpResourceContent {
        content: "c".into(),
        mime_type: "text/plain".into(),
    };
    assert_eq!(
        serde_json::to_value(&content).unwrap(),
        json!({"content": "c", "mime_type": "text/plain"})
    );
    assert!(serde_json::from_value::<McpResourceContent>(json!({"content": "c"})).is_err());
}

#[test]
fn a_pass_answers_bare_diagnostics_or_diagnostics_with_a_summary() {
    let diagnostic = PassDiagnostic::warning("W1", "m").with_entity("a");
    let bare: PassAnswer = serde_json::from_value(json!([
        {"code": "W1", "severity": "Warning", "message": "m", "entity": "a"}
    ]))
    .unwrap();
    assert_eq!(
        bare.into_output().diagnostics,
        std::slice::from_ref(&diagnostic)
    );
    let with: PassAnswer = serde_json::from_value(json!({
        "diagnostics": [{"code": "E2", "severity": "Error", "message": "e"}],
        "summary": {"n": 1}
    }))
    .unwrap();
    let output = with.into_output();
    assert_eq!(output.diagnostics[0].severity, PassSeverity::Error);
    assert_eq!(output.summary["n"], 1);
    // The answer encodes as its form.
    assert_eq!(
        serde_json::to_value(PassAnswer::from(vec![diagnostic])).unwrap(),
        json!([{"code": "W1", "severity": "Warning", "message": "m", "entity": "a"}])
    );
    assert_eq!(
        serde_json::to_value(PassAnswer::from(PassOutput::default())).unwrap(),
        json!({"diagnostics": [], "summary": {}})
    );
    // A malformed answer says what was wrong with the form it chose.
    let error = serde_json::from_value::<PassAnswer>(json!([{"code": "W1"}])).unwrap_err();
    assert!(
        error.to_string().contains("missing field `severity`"),
        "{error}"
    );
    let error = serde_json::from_value::<PassAnswer>(json!("x")).unwrap_err();
    assert!(error.to_string().contains("expected an array"), "{error}");
}
