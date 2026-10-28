//! K15 (ADR 0295): a judged rule's judge may be `prompt:<id>`, a prompt of the rule's own extension.

use serde_json::json;
use specforge_protocol_types::{ExtensionDeclaration, PeerDependency};
use specforge_registry::build_registries;
use specforge_test_macros::test as specforge_test;

/// `@t/core`, declaring the prompt `spec_phase` (and `peer_only` in `@t/peer`), with one judged rule `W311` whose
/// judge is `judge`.
fn declarations(judge: &str) -> Vec<ExtensionDeclaration> {
    let core = |name: &str, prompt: &str| -> ExtensionDeclaration {
        serde_json::from_value(json!({
            "handshake": {
                "protocol_version": "1.10.0", "name": name, "version": "1.0.0", "peer_dependencies": [],
                "sandbox_policy": null, "ext_short": "t",
            },
            "entities": [{"name": "phase", "keyword": "phase"}],
            "surfaces": {"prompts": [
                {"id": prompt, "title": "S", "description": "S", "export": format!("prompt__{prompt}")},
            ]},
        }))
        .unwrap()
    };
    let mut declaration = core("@t/core", "spec_phase");
    declaration.validation_rules = serde_json::from_value(json!([
        {"code": "W311", "severity": "warning", "check": "judged", "message_template": "{id} is not specified",
         "target_kind": "phase",
         "judged": {"judge": judge, "outcomes": ["passed", "issues"], "passing": ["passed"],
                    "targets": [{"kind": "phase", "inputs": [{"name": "phase", "selector": "self"}]}]}},
    ]))
    .unwrap();
    declaration.handshake.peer_dependencies = vec![PeerDependency {
        name: "@t/peer".into(),
        version: "^1.0".into(),
        optional: false,
    }];
    vec![declaration, core("@t/peer", "peer_only")]
}

fn w112(judge: &str) -> Vec<String> {
    build_registries(declarations(judge))
        .diagnostics()
        .filter(|d| d.code == "W112")
        .map(|d| d.message.clone())
        .collect()
}

#[specforge_test(
    behavior = "declare_judged_rule",
    verify = "a judge may be a prompt of the rule's own extension, and a missing or a peer's prompt is W112"
)]
fn a_prompt_judge_registers() {
    let build = build_registries(declarations("prompt:spec_phase"));
    let w112: Vec<String> = build
        .diagnostics()
        .filter(|d| d.code == "W112")
        .map(|d| d.message.clone())
        .collect();
    assert!(w112.is_empty(), "{w112:#?}");
    let judges: Vec<&str> = build.rules().judged().map(|r| r.judge.as_str()).collect();
    assert_eq!(judges, ["prompt:spec_phase"]);
    let judged: Vec<&str> = build
        .judged_rules_of_prompt("@t/core", "spec_phase")
        .iter()
        .map(|r| r.code.as_str())
        .collect();
    assert_eq!(judged.len(), 1);
    assert!(build.judged_rules_of_prompt("@t/core", "other").is_empty());
    assert!(
        build
            .judged_rules_of_prompt("@t/peer", "spec_phase")
            .is_empty()
    );
}

#[specforge_test(
    behavior = "declare_judged_rule",
    verify = "a judge may be a prompt of the rule's own extension, and a missing or a peer's prompt is W112"
)]
fn a_missing_prompt_judge_is_w112() {
    let found = w112("prompt:missing");
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].contains("its judge 'prompt:missing' is no prompt of the extension"),
        "{found:?}"
    );
}

#[specforge_test(
    behavior = "declare_judged_rule",
    verify = "a judge may be a prompt of the rule's own extension, and a missing or a peer's prompt is W112"
)]
fn a_peers_prompt_is_not_a_judge() {
    let found = w112("prompt:peer_only");
    assert_eq!(found.len(), 1, "{found:#?}");
}

#[specforge_test(
    behavior = "declare_judged_rule",
    verify = "a judge may be a prompt of the rule's own extension, and a missing or a peer's prompt is W112"
)]
fn a_malformed_prompt_judge_is_w112() {
    for judge in ["prompt:", "prompt:a b", "prompt:a:b", "Prompt x"] {
        assert_eq!(w112(judge).len(), 1, "{judge}: {:#?}", w112(judge));
    }
}

#[specforge_test(
    behavior = "declare_judged_rule",
    verify = "a judge may be a prompt of the rule's own extension, and a missing or a peer's prompt is W112"
)]
fn a_prompt_the_surfaces_check_dropped_is_not_a_judge() {
    // An argument named for something the host owns drops the prompt whole (W021): it is not registered.
    let mut declarations = declarations("prompt:spec_phase");
    declarations[0].surfaces.prompts[0].args = serde_json::from_value(json!([
        {"name": "path", "field_type": "string", "description": "reserved"},
    ]))
    .unwrap();

    let build = build_registries(declarations);

    let messages: Vec<String> = build
        .diagnostics()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect();
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("W021") && m.contains("prompt 'spec_phase'")),
        "{messages:#?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("W112") && m.contains("no prompt of the extension")),
        "{messages:#?}"
    );
    assert!(
        build
            .judged_rules_of_prompt("@t/core", "spec_phase")
            .is_empty()
    );
}
