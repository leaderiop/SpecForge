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

#[specforge_test(
    behavior = "refuse_human_only_write",
    verify = "a human command that does not write is dropped"
)]
fn a_human_command_that_does_not_write_is_dropped() {
    let mut mute = command("attest");
    mute["human"] = json!(true);
    let mut writing = command("confirm");
    writing["human"] = json!(true);
    writing["writes"] = json!(true);
    let build = build_registries(vec![declaration(
        "@t/core",
        "t",
        json!({"surfaces": {"commands": [mute, writing]}}),
    )]);
    let w021 = messages(&build, "W021");
    assert_eq!(w021.len(), 1, "{w021:#?}");
    assert!(
        w021[0].contains("'attest'") && w021[0].contains("human and does not write"),
        "{w021:#?}"
    );
    let routed: Vec<_> = build
        .commands()
        .routed()
        .iter()
        .map(|r| r.declaration().id.clone())
        .collect();
    assert_eq!(routed, ["confirm"]);
}

#[specforge_test(
    behavior = "register_surface_contributions",
    verify = "the registry refuses a second free-text tail and a tail that is not text, naming the arguments"
)]
fn a_prompt_takes_one_tail_of_text() {
    let arg = |name: &str, field_type: &str, rest: bool| {
        let mut arg = json!({"name": name, "field_type": field_type});
        if field_type == "reference" {
            arg["target_kind"] = json!("phase");
        }
        if rest {
            arg["rest"] = json!(true);
        }
        arg
    };
    let defects = [
        (
            "two_tails",
            json!({"args": [arg("a", "string", true), arg("b", "string", true)]}),
        ),
        ("number_tail", json!({"args": [arg("n", "integer", true)]})),
    ];
    let mut prompts = vec![prompt("fine")];
    let mut fine = prompt("one_tail");
    fine["args"] = json!([arg("full", "bool", false), arg("task", "string", true)]);
    prompts.push(fine);
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
            "surfaces": {"prompts": prompts},
            "entities": [{"name": "phase", "keyword": "phase"}],
        }),
    )]);
    let w021 = messages(&build, "W021");
    for (id, named) in [
        (
            "two_tails",
            "a prompt takes one free-text tail, and it declares 'a' and 'b'",
        ),
        (
            "number_tail",
            "its free-text tail 'n' is not a string argument",
        ),
    ] {
        let found: Vec<_> = w021
            .iter()
            .filter(|m| m.contains(&format!("'{id}'")))
            .collect();
        assert_eq!(found.len(), 1, "{id}: {w021:#?}");
        assert!(found[0].contains(named), "{id}: {}", found[0]);
    }
    assert!(w021.iter().all(|m| !m.contains("'one_tail'")), "{w021:#?}");
}

#[specforge_test(
    behavior = "register_surface_contributions",
    verify = "a prompt argument may be named path or format, never host, argv, help, settings or secrets"
)]
fn a_prompt_may_name_an_argument_path_or_format() {
    let with = |id: &str, name: &str| {
        let mut p = prompt(id);
        p["args"] = json!([{"name": name, "field_type": "string"}]);
        p
    };
    let build = build_registries(vec![declaration(
        "@t/core",
        "t",
        json!({"surfaces": {"prompts": [
            with("has_path", "path"), with("has_format", "format"),
            with("has_host", "host"), with("has_argv", "argv"), with("has_help", "help"),
            with("has_settings", "settings"), with("has_secrets", "secrets"),
        ]}}),
    )]);
    let w021 = messages(&build, "W021");
    for refused in [
        "has_host",
        "has_argv",
        "has_help",
        "has_settings",
        "has_secrets",
    ] {
        assert_eq!(
            w021.iter()
                .filter(|m| m.contains(&format!("'{refused}'")))
                .count(),
            1,
            "{refused}: {w021:#?}"
        );
    }
    assert!(
        w021.iter()
            .all(|m| !m.contains("'has_path'") && !m.contains("'has_format'")),
        "{w021:#?}"
    );
}

#[specforge_test(
    behavior = "register_surface_contributions",
    verify = "an optional subject follows the required positional arguments"
)]
fn an_optional_subject_follows_the_required_positionals() {
    let phase = json!({"name": "phase", "field_type": "reference", "target_kind": "phase"});
    let name = json!({"name": "name", "field_type": "string", "required": true});
    let mut before = prompt("before");
    before["args"] = json!([phase, name]);
    before["subject"] = json!("phase");
    let mut after = prompt("after");
    after["args"] = json!([name, phase]);
    after["subject"] = json!("phase");
    let build = build_registries(vec![declaration(
        "@t/core",
        "t",
        json!({
            "surfaces": {"prompts": [before, after]},
            "entities": [{"name": "phase", "keyword": "phase"}],
        }),
    )]);
    let w021 = messages(&build, "W021");
    assert_eq!(w021.len(), 1, "{w021:#?}");
    assert!(
        w021[0].contains("'before'") && w021[0].contains("'phase'"),
        "{w021:#?}"
    );
}
