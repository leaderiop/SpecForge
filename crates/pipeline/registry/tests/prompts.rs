//! The prompts an extension declares, and the gates of its commands, as the registry build judges them (ADR 0291):
//! one route for a command and a prompt, and W021 for what the registry refuses.

use serde_json::{Value, json};
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::{RegistryBuild, build_registries};
use specforge_test_macros::test as specforge_test;

/// A declaration of `name` (short name `short`) with `surfaces`, `codes` and `settings` as the wire carries them.
fn declaration(name: &str, short: &str, extras: Value) -> ExtensionDeclaration {
    let mut wire = json!({
        "handshake": {
            "protocol_version": "1.10.0", "name": name, "version": "1.0.0", "peer_dependencies": [],
            "sandbox_policy": null, "ext_short": short,
        },
    });
    for (key, value) in extras.as_object().unwrap() {
        wire[key] = value.clone();
    }
    serde_json::from_value(wire).unwrap()
}

fn prompt(id: &str) -> Value {
    json!({"id": id, "title": id, "description": id, "export": format!("prompt__{id}")})
}

fn command(id: &str) -> Value {
    json!({"id": id, "title": id, "description": id, "export": format!("cmd__{id}"), "args": []})
}

fn messages(build: &RegistryBuild, code: &str) -> Vec<String> {
    build
        .diagnostics()
        .filter(|d| d.code == code)
        .map(|d| d.message.clone())
        .collect()
}

#[specforge_test(
    behavior = "register_surface_contributions",
    verify = "a prompt and a command of one route are E039, and the command is the one routed"
)]
#[ignore = "K8 T4"]
fn a_prompt_and_a_command_of_one_route_are_e039() {
    let build = build_registries(vec![
        declaration(
            "@t/core",
            "t",
            json!({"surfaces": {"prompts": [prompt("plan_phase")]}}),
        ),
        declaration(
            "@t/other",
            "t",
            json!({"surfaces": {"commands": [command("plan_phase")]}}),
        ),
    ]);

    let e039 = messages(&build, "E039");
    assert_eq!(e039.len(), 1, "{e039:?}");
    assert!(
        e039[0].contains("prompt 'plan_phase' of '@t/core'") && e039[0].contains("@t/other"),
        "the prompt is the one that is not routed: {}",
        e039[0]
    );
    assert_eq!(build.commands().routed().len(), 1, "the command is routed");
}

#[specforge_test(
    behavior = "register_surface_contributions",
    verify = "a prompt declaration the registry refuses is W021 and the rest of the declaration is kept"
)]
#[ignore = "K8 T4"]
fn a_prompt_declaration_the_registry_refuses_is_w021() {
    let defects = [
        (
            "bad_type",
            json!({"args": [{"name": "x", "field_type": "frobnicate"}]}),
        ),
        ("bad_gate", json!({"gates": ["E999"]})),
        (
            "bad_step",
            json!({"steps": [{"id": "s", "title": "S", "command": "nowhere"}]}),
        ),
        ("bad_when", json!({"when": "workflow.level"})),
        (
            "bad_argv",
            json!({"args": [{"name": "argv", "field_type": "string"}]}),
        ),
    ];
    let mut prompts = vec![prompt("fine")];
    for (id, defect) in &defects {
        let mut broken = prompt(id);
        for (key, value) in defect.as_object().unwrap() {
            broken[key] = value.clone();
        }
        prompts.push(broken);
    }
    let build = build_registries(vec![declaration(
        "@t/core",
        "t",
        json!({
            "surfaces": {"prompts": prompts, "commands": [command("record_plan")]},
            "codes": [{"code": "E901", "title": "A", "explanation": "A"}],
            "settings": [{"kind": "setting", "name": "workflow.level", "field_type": "integer"}],
        }),
    )]);

    let w021 = messages(&build, "W021");
    for (id, _) in &defects {
        assert_eq!(
            w021.iter()
                .filter(|m| m.contains(&format!("'{id}'")))
                .count(),
            1,
            "{id}: {w021:#?}"
        );
    }
    assert!(w021.iter().all(|m| !m.contains("'fine'")), "{w021:#?}");
}
