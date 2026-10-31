//! Host contributions on the wire (ADR 0292, protocol 1.10.0): what a declaration holding agents, skills, slash
//! commands and hooks round-trips as.
#![allow(
    clippy::unwrap_used,
    reason = "a test's unwrap is its assertion: a None or an Err fails the test"
)]

use serde_json::json;
use specforge_protocol_types::SurfaceDescriptor;

#[specforge_test_macros::test(
    behavior = "install_into_a_host",
    verify = "an agent, a skill, a slash command and a hook round-trip through the protocol"
)]
fn host_contributions_round_trip() {
    let wire = json!({
        "commands": [{"id": "stats", "title": "Stats", "description": "Stats", "export": "cmd__stats", "args": [],
                      "reads_state": true}],
        "agents": [{"id": "t-planner", "description": "Plans", "tools": ["read", "write", "mcp:context7/*"],
                    "model": "setting:models.planner", "effort": "high", "color": "green",
                    "variants": ["compact"], "variant_setting": "context_profile", "when": "workflow.on",
                    "attribution": {"upstream": "u", "path": "p", "version": "1", "license": "MIT", "copyright": "c"}}],
        "skills": [{"id": "t-ns", "description": "Routes", "tools": ["read", "skill"], "argument_hint": "<x>"}],
        "slash_commands": [{"prompt": "plan_phase", "argument_hint": "<phase>", "tools": ["read"], "effort": "max"},
                           {"command": "stats", "tools": ["read"]}],
        "hooks": [{"id": "write_guard", "description": "Guards", "event": "pre_tool", "tools": ["write"],
                   "paths": ["{spec_root}/**"], "timeout_ms": 3000, "on_failure": "deny", "reads_graph": true,
                   "reads_state": true, "writes": true, "when": "workflow.hooks", "export": "hook__write_guard"}],
    });
    let parsed: SurfaceDescriptor = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(&parsed).unwrap(), wire);
}

#[specforge_test_macros::test(
    behavior = "install_into_a_host",
    verify = "when reads a name or a list and writes one name as a string, several as a list and none as nothing"
)]
fn when_reads_a_name_or_a_list_and_writes_one_as_a_name() {
    use specforge_protocol_types::{AgentDescriptor, PromptDescriptor};

    let agent = |when: serde_json::Value| -> AgentDescriptor {
        serde_json::from_value(json!({"id": "t-a", "description": "A", "when": when})).unwrap()
    };
    assert_eq!(agent(json!("a")).when, ["a"]);
    assert_eq!(agent(json!(["a", "b"])).when, ["a", "b"]);
    assert_eq!(agent(json!(null)).when, Vec::<String>::new());
    let written = |a: &AgentDescriptor| serde_json::to_value(a).unwrap();
    assert_eq!(written(&agent(json!("a")))["when"], "a");
    assert_eq!(
        written(&agent(json!(["a", "b"])))["when"],
        json!(["a", "b"])
    );
    let none: AgentDescriptor =
        serde_json::from_value(json!({"id": "t-a", "description": "A"})).unwrap();
    assert!(written(&none).get("when").is_none());
    let prompt: PromptDescriptor = serde_json::from_value(
        json!({"id": "p", "title": "P", "description": "P", "export": "prompt__p", "when": ["x", "y"]}),
    )
    .unwrap();
    assert_eq!(prompt.when, ["x", "y"]);
}
