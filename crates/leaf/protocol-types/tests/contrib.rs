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
#[ignore = "K9 T3"]
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
