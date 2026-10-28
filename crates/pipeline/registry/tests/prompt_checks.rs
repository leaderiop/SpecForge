//! What the registry keeps of a prompt it accepts, and the rest of what it refuses (ADR 0291).

use serde_json::{Value, json};
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::{RegistryBuild, build_registries};
use specforge_test_macros::test as specforge_test;

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

/// `@t/core`: the kind `phase`, the codes `E901` and `W904`, the bool setting `workflow.research`, a command
/// `record_plan`, and `prompts`.
fn core(prompts: Value) -> ExtensionDeclaration {
    declaration(
        "@t/core",
        "t",
        json!({
            "entities": [{"name": "phase", "keyword": "phase"}],
            "codes": [{"code": "E901", "title": "A", "explanation": "A"},
                      {"code": "W904", "title": "B", "explanation": "B"}],
            "settings": [{"kind": "setting", "name": "workflow.research", "field_type": "bool", "default": true}],
            "surfaces": {
                "commands": [{"id": "record_plan", "title": "R", "description": "R", "export": "cmd__record_plan",
                              "args": [{"name": "phase", "arg_type": "string", "required": true}]}],
                "prompts": prompts,
            },
        }),
    )
}

fn prompt(id: &str, extra: Value) -> Value {
    let mut p =
        json!({"id": id, "title": id, "description": id, "export": format!("prompt__{id}")});
    for (key, value) in extra.as_object().unwrap() {
        p[key] = value.clone();
    }
    p
}

fn w021(build: &RegistryBuild) -> Vec<String> {
    build
        .diagnostics()
        .filter(|d| d.code == "W021")
        .map(|d| d.message.clone())
        .collect()
}

#[specforge_test(
    behavior = "register_surface_contributions",
    verify = "a prompt the registry accepts is routed with its arguments, gates, steps and attribution, and no W021"
)]
fn a_sound_prompt_is_routed_whole() {
    let sound = prompt(
        "plan_phase",
        json!({
            "args": [
                {"name": "phase", "field_type": "reference", "target_kind": "phase", "required": true},
                {"name": "depth", "field_type": "integer", "minimum": 1, "maximum": 3, "default": 2},
                {"name": "mode", "field_type": "enum", "enum_values": ["fast", "slow"], "default": "fast"},
                {"name": "labels", "field_type": "string_list"},
            ],
            "gates": ["E901"], "subject": "phase", "when": "workflow.research",
            "steps": [{"id": "a", "title": "A", "command": "record_plan"},
                      {"id": "b", "title": "B"}, {"id": "c", "title": "C", "prompt": "plan_phase"}],
            "attribution": {"upstream": "u", "path": "p", "version": "1", "license": "MIT", "copyright": "c"},
        }),
    );
    let build = build_registries(vec![core(json!([sound.clone()]))]);
    assert_eq!(w021(&build), Vec::<String>::new());
    let routed = build.prompts();
    assert_eq!(routed.len(), 1);
    assert_eq!(routed[0].short(), "t");
    assert_eq!(routed[0].cli_name(), "plan-phase");
    assert_eq!(
        serde_json::to_value(routed[0].declaration()).unwrap(),
        sound
    );
}

#[specforge_test(
    behavior = "register_surface_contributions",
    verify = "a prompt declaration the registry refuses is W021 and the rest of the declaration is kept"
)]
fn each_refused_prompt_is_dropped_whole_and_the_rest_is_kept() {
    let defects = [
        ("Bad-Id", json!({"id": "Bad-Id"})),
        ("wrong_export", json!({"export": "cmd__wrong_export"})),
        (
            "twice",
            json!({"args": [{"name": "a", "field_type": "string"}, {"name": "a", "field_type": "string"}]}),
        ),
        (
            "host_name",
            json!({"args": [{"name": "path", "field_type": "string"}]}),
        ),
        (
            "no_target",
            json!({"args": [{"name": "p", "field_type": "reference"}]}),
        ),
        (
            "ghost_kind",
            json!({"args": [{"name": "p", "field_type": "reference", "target_kind": "ghost"}]}),
        ),
        (
            "bounds",
            json!({"args": [{"name": "p", "field_type": "string", "minimum": 1}]}),
        ),
        (
            "bad_default",
            json!({"args": [{"name": "n", "field_type": "integer", "default": "x"}]}),
        ),
        (
            "empty_enum",
            json!({"args": [{"name": "e", "field_type": "enum"}]}),
        ),
        ("no_subject", json!({"subject": "phase", "gates": ["E901"]})),
        (
            "peerless",
            json!({"steps": [{"id": "s", "title": "S", "command": "ghost:go"}]}),
        ),
        (
            "two_targets",
            json!({"steps": [{"id": "s", "title": "S", "command": "record_plan", "prompt": "x"}]}),
        ),
    ];
    let mut prompts = vec![prompt("fine", json!({}))];
    for (id, defect) in &defects {
        prompts.push(prompt(
            if *id == "Bad-Id" { "Bad-Id" } else { id },
            defect.clone(),
        ));
    }
    let build = build_registries(vec![core(json!(prompts))]);
    let messages = w021(&build);
    for (id, _) in &defects {
        let named = messages
            .iter()
            .filter(|m| m.contains(&format!("'{id}'")))
            .count();
        assert_eq!(named, 1, "{id}: {messages:#?}");
    }
    let served: Vec<&str> = build
        .prompts()
        .iter()
        .map(|p| p.declaration().id.as_str())
        .collect();
    assert_eq!(served, ["fine"], "only the sound prompt is served");
    assert_eq!(build.commands().routed().len(), 1, "the commands are kept");
}

#[specforge_test(
    behavior = "register_surface_contributions",
    verify = "a step's agent and an empty attribution are dropped alone, with a W021 each"
)]
fn an_agent_step_and_an_empty_attribution_are_dropped_alone() {
    let p = prompt(
        "plan_phase",
        json!({
            "steps": [{"id": "s", "title": "S", "agent": "checker"}],
            "attribution": {"upstream": "u", "path": "", "version": "1", "license": "MIT", "copyright": "c"},
        }),
    );
    let build = build_registries(vec![core(json!([p]))]);
    assert_eq!(w021(&build).len(), 2, "{:#?}", w021(&build));
    let kept = build.prompts()[0].declaration();
    assert_eq!(kept.steps[0].agent, None);
    assert_eq!(kept.attribution, None);
}

#[specforge_test(
    behavior = "register_surface_contributions",
    verify = "a command whose gate the extension does not catalog, or whose subject is no arg, is W021 and not routed"
)]
fn a_command_with_a_gate_it_cannot_evaluate_is_not_routed() {
    let mut declaration = core(json!([]));
    declaration.surfaces.commands[0].gates = vec!["E999".into()];
    let mut second = declaration.surfaces.commands[0].clone();
    second.id = "ship".into();
    second.export = "cmd__ship".into();
    second.gates = vec!["E901".into()];
    second.subject = Some("nowhere".into());
    declaration.surfaces.commands.push(second);
    let build = build_registries(vec![declaration]);
    let messages = w021(&build);
    assert_eq!(messages.len(), 2, "{messages:#?}");
    assert!(
        messages
            .iter()
            .any(|m| m.contains("'record_plan'") && m.contains("E999"))
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("'ship'") && m.contains("nowhere"))
    );
    assert!(build.commands().routed().is_empty());
}

#[specforge_test(
    behavior = "register_surface_contributions",
    verify = "two prompts of one route are E039 and the later is not routed"
)]
fn the_later_of_two_prompts_of_one_route_is_e039() {
    let build = build_registries(vec![core(json!([
        prompt("plan_phase", json!({})),
        prompt("plan-phase", json!({}))
    ]))]);
    // `plan-phase` is not snake_case, so the registry drops it (W021); the E039 is for two valid ids that spell one name.
    assert_eq!(build.prompts().len(), 1);
    let other = declaration(
        "@t/other",
        "t",
        json!({"surfaces": {"prompts": [prompt("plan_phase", json!({}))]}}),
    );
    let build = build_registries(vec![core(json!([prompt("plan_phase", json!({}))])), other]);
    let e039: Vec<_> = build.diagnostics().filter(|d| d.code == "E039").collect();
    assert_eq!(e039.len(), 1);
    assert!(
        e039[0]
            .message
            .contains("prompt 'plan_phase' of '@t/other'"),
        "{}",
        e039[0].message
    );
    assert_eq!(build.prompts().len(), 1);
    assert_eq!(build.prompts()[0].extension(), "@t/core");
}
