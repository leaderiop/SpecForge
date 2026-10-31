//! `security_hooks <point>`: the security capability's hooks at a loop point, as `gsd-tools loop render-hooks <point>`
//! answered for the `security` entries (`capabilities/security/capability.json` at v1.18.0). `@gsd/core`'s capability
//! points ask every loaded capability extension this one question (ADR gsd/0002).

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

use super::support::{answer, required, run};

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

fn security_hooks(call: &CommandCall<'_>) -> Result<CommandOutput, String> {
    let point = required(call, "point")?;
    let settings = Settings::current();
    let found = hooks(point, &|key| settings.get(key).cloned())?;
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
