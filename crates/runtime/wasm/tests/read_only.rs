//! A command that declares no `writes` is called exactly as before edit plans (ADR 0289): its input holds none of
//! the plan's fields.

use serde_json::Value;
use specforge_protocol_types::{CommandInput, RawGraph};

fn golden() -> Value {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/wire/command.input.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[specforge_test_macros::test(
    behavior = "apply_edit_plan",
    verify = "a read-only command's input carries no revisions"
)]
fn a_read_only_commands_input_is_unchanged() {
    let golden = golden();
    let input = CommandInput {
        args: golden["args"].as_object().unwrap().clone(),
        cwd: golden["cwd"].as_str().unwrap().to_string(),
        format: serde_json::from_value(golden["format"].clone()).unwrap(),
        today: golden["today"].as_str().unwrap().to_string(),
        graph: RawGraph::new(golden["graph"].to_string()).unwrap(),
        ..Default::default()
    };
    let sent = serde_json::to_value(&input).unwrap();
    assert_eq!(sent, golden);
    assert!(sent.get("revisions").is_none(), "{sent}");
}

#[specforge_test_macros::test(
    behavior = "run_a_hook",
    verify = "a command without state has a byte-equal input"
)]
fn a_command_without_state_has_a_byte_equal_input() {
    let golden = golden();
    let input = CommandInput {
        args: golden["args"].as_object().unwrap().clone(),
        cwd: golden["cwd"].as_str().unwrap().to_string(),
        format: serde_json::from_value(golden["format"].clone()).unwrap(),
        today: golden["today"].as_str().unwrap().to_string(),
        graph: RawGraph::new(golden["graph"].to_string()).unwrap(),
        ..Default::default()
    };
    let sent = serde_json::to_value(&input).unwrap();
    assert_eq!(sent, golden);
    assert!(sent.get("state").is_none(), "{sent}");
}
