//! `security_hooks <point>`: the security capability's hooks at a loop point, as `gsd-tools loop render-hooks <point>`
//! answered for the `security` entries (`capabilities/security/capability.json` at v1.18.0). `@gsd/core`'s capability
//! points ask every loaded capability extension this one question (ADR gsd/0002).

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, node, required, run, target};

/// The capability file, byte for byte upstream's; only the `plan:pre` fragment is rewritten ([`FRAGMENT`]).
const CAPABILITY: &str = include_str!("../../assets/capabilities/security/capability.json");

/// The fragment upstream injects into the planner at `plan:pre`.
pub const UPSTREAM_FRAGMENT: &str = "Each PLAN.md must include a <threat_model> block when security enforcement is active. Use the configured ASVS level and blocking threshold from workflow.security_asvs_level and workflow.security_block_on.";

/// `[fragment-repointed]`: here the threat model is entities of the plan, not a `<threat_model>` block of a PLAN.md.
pub const FRAGMENT: &str = "Each plan must record its threat model as `trust_boundary` and `threat` entities when security enforcement is active: a `threat` has a STRIDE category, a component, a severity, a disposition and a mitigation, and every plan keeps a `T-<phase>-SC` threat for its supply chain. Use the configured ASVS level and blocking threshold from workflow.security_asvs_level and workflow.security_block_on.";

/// The capability with the repointed fragment.
fn capability() -> Value {
    let mut cap: Value = serde_json::from_str(CAPABILITY).expect("capability.json is JSON");
    for hook in cap["contributions"].as_array_mut().into_iter().flatten() {
        if hook["fragment"]["inline"] == UPSTREAM_FRAGMENT {
            hook["fragment"]["inline"] = FRAGMENT.into();
        }
    }
    cap
}

/// The answer for `point` under `setting`.
pub fn hooks(point: &str, setting: &dyn Fn(&str) -> Option<Value>) -> Result<Value, String> {
    gsd_kit::capability::render(&capability(), setting, point)
}

/// The phase an `--after-fingerprint` value names: the phase itself, or a phase directory as core hands it
/// (`.planning/phases/01-setup`: its last segment, or the number before the first `-`).
fn fingerprint_phase<'a>(call: &CommandCall<'a>, value: &str) -> Result<&'a GraphNode, String> {
    let last = value
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(value);
    [value, last, last.split('-').next().unwrap_or(last)]
        .into_iter()
        .find_map(|candidate| node(call, "phase", candidate).ok())
        .ok_or_else(|| {
            format!(
                "--after-fingerprint phase not found or unreadable: {value} (no phase of that name)"
            )
        })
}

/// Upstream's `partitionHooksByFingerprint` (#5105 R2): a `step` whose every `produces` artifact exists moves from
/// `activeHooks` to `skippedHooks`; execute-phase already dispatched it. `SECURITY.md` is the phase's `security_audit`
/// (`[verify-post-produces]`); any other artifact name is never present here.
fn skip_produced(found: &mut Value, audit_exists: bool) {
    let hooks = found["activeHooks"].as_array().cloned().unwrap_or_default();
    let (mut kept, mut skipped) = (Vec::new(), Vec::new());
    for hook in hooks {
        let produces: Vec<&str> = hook["produces"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if hook["kind"] == "step"
            && !produces.is_empty()
            && produces.iter().all(|p| audit_exists && *p == "SECURITY.md")
        {
            skipped.push(serde_json::json!({
                "capId": hook["capId"], "kind": "step", "ref": hook["ref"],
                "reason": "produces-present", "artifacts": produces,
            }));
        } else {
            kept.push(hook);
        }
    }
    found["activeHooks"] = kept.into();
    found["skippedHooks"] = skipped.into();
}

fn security_hooks(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let point = required(call, "point")?;
    let settings = Settings::current();
    let mut found = hooks(point, &|key| settings.get(key).cloned())?;
    if let Some(value) = call.str("after_fingerprint") {
        let phase = fingerprint_phase(call, value)?;
        let graph = call.graph();
        let audit = graph
            .nodes_of_kind("security_audit")
            .any(|a| target(graph, &a.id, "phase") == Some(phase.id.as_str()));
        skip_produced(&mut found, audit);
    }
    let count = found["activeHooks"].as_array().map_or(0, Vec::len);
    Ok(answer(
        call,
        found,
        format!("{count} active security hook(s) at {point}\n"),
    ))
}

pub fn declare(c: &mut ContributionsBuilder) {
    c.command("security_hooks", |cmd| {
        cmd.description(
            "The security capability's hooks at a loop point, as loop render-hooks answered",
        )
        .arg("point", |a| {
            a.required()
                .description("a loop point (plan:pre, verify:post, ship:pre, ...)");
        })
        .arg("after_fingerprint", |a| {
            a.description("a phase (or its directory): a step whose artifact the phase already has moves to skippedHooks (#5105)");
        })
        .handler(|call| run(call, security_hooks));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_capability_file_is_upstreams_but_for_the_fragment() {
        let upstream: Value = serde_json::from_str(CAPABILITY).expect("JSON");
        assert_eq!(
            upstream["contributions"][0]["fragment"]["inline"],
            UPSTREAM_FRAGMENT
        );
        let mut ours = capability();
        assert_eq!(ours["contributions"][0]["fragment"]["inline"], FRAGMENT);
        ours["contributions"][0]["fragment"]["inline"] = UPSTREAM_FRAGMENT.into();
        assert_eq!(ours, upstream);
    }
}
