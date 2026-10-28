//! K15 (ADR 0295): a judged rule's judge may be `prompt:<id>`, a prompt of the rule's own extension. Pin for the
//! ticket that delivers it; ignored until then.

use serde_json::json;
use specforge_protocol_types::ExtensionDeclaration;
use specforge_registry::build_registries;

#[test]
#[ignore = "pins K15; T11 un-ignores"]
fn a_prompt_judge_registers() {
    let mut declaration: ExtensionDeclaration = serde_json::from_value(json!({
        "handshake": {
            "protocol_version": "1.10.0", "name": "@t/core", "version": "1.0.0", "peer_dependencies": [],
            "sandbox_policy": null, "ext_short": "t",
        },
        "entities": [{"name": "phase", "keyword": "phase"}],
        "surfaces": {"prompts": [
            {"id": "spec_phase", "title": "S", "description": "S", "export": "prompt__spec_phase"},
        ]},
    }))
    .unwrap();
    declaration.validation_rules = serde_json::from_value(json!([
        {"code": "W311", "severity": "warning", "check": "judged", "message_template": "{id} is not specified",
         "target_kind": "phase",
         "judged": {"judge": "prompt:spec_phase", "outcomes": ["passed", "issues"], "passing": ["passed"],
                    "targets": [{"kind": "phase", "inputs": [{"name": "phase", "selector": "self"}]}]}},
    ]))
    .unwrap();

    let build = build_registries(vec![declaration]);

    let w112: Vec<String> = build
        .diagnostics()
        .filter(|d| d.code == "W112")
        .map(|d| d.message.clone())
        .collect();
    assert!(w112.is_empty(), "{w112:#?}");
    let judges: Vec<&str> = build.rules().judged().map(|r| r.judge.as_str()).collect();
    assert_eq!(judges, ["prompt:spec_phase"]);
}
