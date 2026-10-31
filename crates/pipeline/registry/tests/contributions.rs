//! What the registry makes of the agents, skills, slash commands and hooks an extension declares (ADR 0292): one
//! namespace of host file names per project, hook routes, and a contribution it refuses dropped with a W021.

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

/// `@t/core` (short `t`): the kind `phase`, the bool setting `workflow.on`, the string setting `models.planner`,
/// a prompt `plan_phase`, and `surfaces` merged over that.
fn core(surfaces: Value) -> ExtensionDeclaration {
    let mut all = json!({
        "prompts": [{"id": "plan_phase", "title": "P", "description": "P", "export": "prompt__plan_phase"}],
    });
    for (key, value) in surfaces.as_object().unwrap() {
        all[key] = value.clone();
    }
    declaration(
        "@t/core",
        "t",
        json!({
            "entities": [{"name": "phase", "keyword": "phase"}],
            "settings": [
                {"kind": "setting", "name": "workflow.on", "field_type": "bool", "default": true},
                {"kind": "setting", "name": "models.planner", "field_type": "string", "default": "standard"},
                {"kind": "setting", "name": "count", "field_type": "integer", "default": 1},
            ],
            "surfaces": all,
        }),
    )
}

fn w021(build: &RegistryBuild) -> Vec<String> {
    build
        .diagnostics()
        .filter(|d| d.code == "W021")
        .map(|d| d.message.clone())
        .collect()
}

/// The registry's answers, as the pins read them.
mod asked {
    use super::RegistryBuild;

    pub fn agents(build: &RegistryBuild) -> Vec<String> {
        build.agents().map(|a| a.item.id.clone()).collect()
    }
    pub fn hooks(build: &RegistryBuild) -> Vec<String> {
        build.hooks().map(|h| h.item.id.clone()).collect()
    }
    /// The host names kept across skills and slash commands.
    pub fn skills_and_slashes(build: &RegistryBuild) -> usize {
        build.skills().count() + build.slash_commands().count()
    }
    /// The extension and description of the agent `id`.
    pub fn agent(build: &RegistryBuild, id: &str) -> Option<(String, String)> {
        build
            .agent(id)
            .map(|(extension, a)| (extension.to_string(), a.description.clone()))
    }
}

fn hook(id: &str, event: &str, extra: Value) -> Value {
    let mut hook =
        json!({"id": id, "description": id, "event": event, "export": format!("hook__{id}")});
    for (key, value) in extra.as_object().unwrap() {
        hook[key] = value.clone();
    }
    hook
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "a host name claimed twice is E039 and the later claimant is not rendered"
)]
fn a_host_name_claimed_twice_is_e039() {
    // The slash command of the prompt `plan_phase` is named `t-plan-phase`, as is this skill.
    let build = build_registries(vec![core(json!({
        "skills": [{"id": "t-plan-phase", "description": "S"}],
        "slash_commands": [{"prompt": "plan_phase"}],
        "agents": [{"id": "t-x", "description": "A"}],
    }))]);
    let e039: Vec<String> = build
        .diagnostics()
        .filter(|d| d.code == "E039")
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(e039.len(), 1, "{e039:#?}");
    assert!(e039[0].contains("t-plan-phase"), "{e039:#?}");
    assert_eq!(asked::skills_and_slashes(&build), 1, "one claimant is kept");
    assert_eq!(asked::agents(&build).len(), 1, "an unrelated name is kept");
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "a contribution the registry refuses is W021 and dropped, and the rest is kept"
)]
fn a_contribution_the_registry_refuses_is_w021() {
    let refused = [
        (
            "agent",
            json!({"id": "t-badtool", "description": "A", "tools": ["teleport"]}),
        ),
        (
            "agent",
            json!({"id": "t-badmodel", "description": "A", "model": "gigantic"}),
        ),
        (
            "agent",
            json!({"id": "t-ghostsetting", "description": "A", "model": "setting:nope"}),
        ),
        (
            "agent",
            json!({"id": "t-novariant", "description": "A", "variants": ["compact"]}),
        ),
        ("agent", json!({"id": "Bad_Id", "description": "A"})),
    ];
    let mut agents: Vec<Value> =
        vec![json!({"id": "t-fine", "description": "A", "tools": ["read"]})];
    agents.extend(refused.iter().map(|(_, a)| a.clone()));
    let build = build_registries(vec![core(json!({
        "agents": agents,
        "slash_commands": [{"prompt": "nowhere"}],
        "hooks": [
            hook("fine", "pre_tool", json!({"tools": ["write"]})),
            hook("tools_on_session", "session_start", json!({"tools": ["read"]})),
            hook("slow", "pre_tool", json!({"timeout_ms": 60000})),
            hook("zero", "pre_tool", json!({"timeout_ms": 0})),
            hook("writing_statusline", "statusline", json!({"writes": true})),
            hook("not_bool", "pre_tool", json!({"when": "models.planner"})),
            hook("wrong_export", "pre_tool", json!({"export": "cmd__wrong_export"})),
            hook("elsewhere", "pre_tool", json!({"hosts": ["windsurf"]})),
            hook("cursor_only", "pre_tool", json!({"hosts": ["cursor"]})),
            hook("watch_nothing", "file_changed", json!({})),
            hook("watch_with_tools", "file_changed", json!({"paths": ["specforge.json"], "tools": ["write"]})),
            hook("watch_a_glob", "file_changed", json!({"paths": ["*.json"]})),
            hook("watch_a_file", "file_changed", json!({"paths": ["specforge.json"]})),
        ],
    }))]);
    let messages = w021(&build);
    for named in [
        "t-badtool",
        "t-badmodel",
        "t-ghostsetting",
        "t-novariant",
        "Bad_Id",
        "nowhere",
        "tools_on_session",
        "slow",
        "zero",
        "writing_statusline",
        "not_bool",
        "wrong_export",
        "elsewhere",
        "watch_nothing",
        "watch_with_tools",
        "watch_a_glob",
    ] {
        let hits = messages
            .iter()
            .filter(|m| m.contains(&format!("'{named}'")))
            .count();
        assert_eq!(hits, 1, "{named}: {messages:#?}");
    }
    assert_eq!(asked::agents(&build), ["t-fine"]);
    assert!(
        messages
            .iter()
            .any(|m| m.contains("its host 'windsurf' is not claude-code, codex or cursor")),
        "{messages:#?}"
    );
    assert_eq!(
        asked::hooks(&build),
        ["fine", "cursor_only", "watch_a_file"]
    );
    for why in [
        "a file_changed hook names the files it watches in paths",
        "the event 'file_changed' takes no tools",
        "a file_changed hook's paths are file names, not globs: '*.json'",
    ] {
        assert!(
            messages.iter().any(|m| m.contains(why)),
            "{why}: {messages:#?}"
        );
    }
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "the registry answers an agent by its id, with the extension that declares it"
)]
fn the_registry_answers_an_agent_by_id() {
    let build = build_registries(vec![core(json!({
        "agents": [{"id": "t-plan-checker", "description": "Checks"}],
    }))]);
    let (extension, description) =
        asked::agent(&build, "t-plan-checker").expect("the agent is declared");
    assert_eq!(extension, "@t/core");
    assert_eq!(description, "Checks");
    assert!(asked::agent(&build, "t-nobody").is_none());
}

#[specforge_test(
    behavior = "install_into_a_host",
    verify = "a judged rule whose judge is no agent of its extension or a declared peer is W112"
)]
fn a_judge_naming_no_agent_of_its_extension_is_w112() {
    let rule = |code: &str, judge: &str| {
        json!({"code": code, "severity": "warning", "check": "judged", "message_template": "{id} does not hold",
               "target_kind": "phase",
               "judged": {"judge": judge, "outcomes": ["passed", "issues"], "passing": ["passed"],
                          "targets": [{"kind": "phase", "inputs": [{"name": "phase", "selector": "self"}]}]}})
    };
    let mut declaration =
        core(json!({"agents": [{"id": "t-plan-checker", "description": "Checks"}]}));
    declaration.validation_rules = serde_json::from_value(json!([
        rule("W311", "t-plan-checker"),
        rule("W312", "t-nobody"),
    ]))
    .unwrap();
    let build = build_registries(vec![declaration]);
    let w112: Vec<String> = build
        .diagnostics()
        .filter(|d| d.code == "W112")
        .map(|d| d.message.clone())
        .collect();
    assert_eq!(w112.len(), 1, "{w112:#?}");
    assert!(w112[0].contains("t-nobody"), "{w112:#?}");
    assert_eq!(
        build
            .rules()
            .judged()
            .filter(|r| r.code.ends_with("W311"))
            .count(),
        1,
        "the rule with an agent is kept"
    );
}

#[specforge_test(
    behavior = "register_surface_contributions",
    verify = "every setting a when names must be a bool setting of the extension"
)]
fn each_when_name_is_a_bool_setting() {
    let build = build_registries(vec![core(json!({
        "agents": [
            {"id": "t-fine", "description": "A", "when": ["workflow.on", "workflow.on"]},
            {"id": "t-ghost", "description": "A", "when": ["workflow.on", "nope"]},
            {"id": "t-text", "description": "A", "when": ["models.planner"]},
        ],
        "prompts": [
            {"id": "plan_phase", "title": "P", "description": "P", "export": "prompt__plan_phase",
             "when": ["workflow.on", "count"]},
        ],
    }))]);
    let messages = w021(&build);
    assert_eq!(asked::agents(&build), ["t-fine"]);
    assert!(
        messages
            .iter()
            .any(|m| m.contains("'t-ghost'") && m.contains("'nope'")),
        "{messages:#?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("'t-text'")),
        "{messages:#?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("'plan_phase'") && m.contains("'count'")),
        "{messages:#?}"
    );
}
