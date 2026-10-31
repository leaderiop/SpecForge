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

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "a prompt declaration round-trips through the protocol with its args, steps and attribution"
)]
fn a_prompt_declaration_round_trips() {
    let wire = json!({
        "commands": [{
            "id": "execute_phase", "title": "Execute", "description": "Execute the phase",
            "export": "cmd__execute_phase",
            "args": [{"name": "phase", "arg_type": "string", "required": true}],
            "gates": ["E902"], "subject": "phase",
        }],
        "prompts": [{
            "id": "plan_phase", "title": "Plan a phase", "description": "Plan the phase",
            "export": "prompt__plan_phase",
            "args": [
                {"name": "phase", "field_type": "reference", "target_kind": "phase", "required": true,
                 "description": "The phase"},
                {"name": "research", "field_type": "bool", "default": false},
                {"name": "depth", "field_type": "integer", "minimum": 1, "maximum": 3},
                {"name": "mode", "field_type": "enum", "enum_values": ["fast", "slow"]},
            ],
            "gates": ["E901"], "subject": "phase",
            "steps": [
                {"id": "plan", "title": "Plan", "command": "record_plan"},
                {"id": "check", "title": "Check", "agent": "plan-checker"},
                {"id": "next", "title": "Next", "prompt": "execute_phase"},
            ],
            "reads_graph": true, "when": "workflow.research",
            "attribution": {
                "upstream": "gsd-core", "path": "gsd-core/workflows/plan-phase.md", "version": "1.18.0",
                "license": "MIT", "copyright": "Copyright (c) 2026 Open GSD",
            },
        }],
    });
    let surfaces: SurfaceDescriptor = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&surfaces).unwrap(),
        wire,
        "a declared prompt, gate and subject survive a decode and an encode"
    );
}

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "a pass input without a trigger reads a write and a gate trigger round-trips with its command and subject"
)]
fn a_pass_input_trigger_is_absent_for_a_write_and_round_trips_for_a_gate() {
    use specforge_protocol_types::{PassInput, Trigger, TriggerKind};
    let plain = serde_json::to_value(PassInput::default()).unwrap();
    assert!(plain.get("trigger").is_none(), "{plain}");
    let guard = PassInput {
        trigger: Some(Trigger::gate("ship", Some("v1.0/03".into()))),
        ..Default::default()
    };
    let wire = serde_json::to_value(&guard).unwrap();
    assert_eq!(
        wire["trigger"],
        json!({"kind": "gate", "command": "ship", "subject": "v1.0/03"})
    );
    let back: PassInput = serde_json::from_value(wire).unwrap();
    assert_eq!(back.trigger.unwrap().kind, TriggerKind::Gate);
    assert_eq!(
        Trigger::default().kind,
        TriggerKind::Write,
        "no trigger is a write"
    );
}

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "a prompt argument that is not a tail serializes as it did, and a tail round-trips"
)]
fn a_rest_false_argument_has_no_rest_key() {
    use specforge_protocol_types::PromptArgDescriptor;

    let arg = PromptArgDescriptor {
        name: "task".into(),
        field_type: "string".into(),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_string(&arg).unwrap(),
        r#"{"name":"task","field_type":"string"}"#
    );
    let tail = PromptArgDescriptor { rest: true, ..arg };
    let wire = serde_json::to_value(&tail).unwrap();
    assert_eq!(wire["rest"], true);
    assert_eq!(
        serde_json::from_value::<PromptArgDescriptor>(wire).unwrap(),
        tail
    );
}

#[specforge_test_macros::test(
    behavior = "serve_extension_prompt",
    verify = "a prompt answer without a subject serializes as it did, and a picked subject round-trips"
)]
fn a_prompt_output_subject_is_absent_unless_picked() {
    use specforge_protocol_types::PromptOutput;

    let plain = PromptOutput::user("hi");
    assert_eq!(
        serde_json::to_string(&plain).unwrap(),
        r#"{"messages":[{"role":"user","text":"hi"}]}"#
    );
    let picked = plain.with_subject("v1.0/03");
    let wire = serde_json::to_value(&picked).unwrap();
    assert_eq!(wire["subject"], "v1.0/03");
    assert_eq!(
        serde_json::from_value::<PromptOutput>(wire).unwrap(),
        picked
    );
}
