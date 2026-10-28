//! Prompts and gates on the wire (ADR 0291, protocol 1.10.0): what a declaration without them serializes as, and
//! what one with them round-trips as.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::json;
use specforge_protocol_types::{CommandDescriptor, SurfaceDescriptor};

#[specforge_test_macros::test(
    behavior = "gate_a_command",
    verify = "a command descriptor that declares no gate serializes as it did"
)]
fn the_command_descriptor_without_gates_is_byte_equal() {
    let command = CommandDescriptor {
        id: "report".into(),
        title: "Report".into(),
        description: "A report".into(),
        export: "cmd__report".into(),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_string(&command).unwrap(),
        r#"{"id":"report","title":"Report","description":"A report","export":"cmd__report","args":[]}"#
    );
    let surfaces = SurfaceDescriptor {
        commands: vec![command],
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_value(&surfaces).unwrap(),
        json!({"commands": [{
            "id": "report", "title": "Report", "description": "A report", "export": "cmd__report", "args": [],
        }]}),
        "no `prompts`, `gates` or `subject` key"
    );
}
