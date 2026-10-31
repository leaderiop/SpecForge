//! `gsd-security-auditor`, ported under MIT from gsd-core v1.18.0 (`assets/agents/`): return-only (#2119), the
//! orchestrator is the sole writer of the audit record.

use specforge_extension_sdk::prelude::*;

/// The full body, and the compact one Codex and Cursor get (`specforge.agent_variant`).
pub const FULL: &str = include_str!("../assets/agents/gsd-security-auditor.md");
pub const COMPACT: &str = include_str!("../assets/agents/gsd-security-auditor.compact.md");

pub const ID: &str = "gsd-security-auditor";

pub fn declare(c: &mut ContributionsBuilder) {
    c.agent(ID, |a| {
        a.description("Verifies threat mitigations from PLAN.md threat model exist in implemented code. Returns structured security verdict (SECURED / OPEN_THREATS / ESCALATE). Spawned by /gsd:secure-phase.")
            // Upstream's Read, Bash, Glob, Grep, Skill; no write tool (#2119).
            .tools(&["read", "shell", "search", "skill"])
            .color("red")
            .model(&format!("setting:model_overrides.{ID}"))
            .attribution(gsd_kit::agent::attribution(&format!("agents/{ID}.md")))
            .variant("compact")
            .variant_setting("specforge.agent_variant");
    });
    c.render(|call| {
        // Upstream installs no compact agent for Claude Code, so it always gets the full body (as core's agents).
        let body = match (call.host(), call.variant()) {
            ("claude-code", _) | (_, None) => FULL,
            _ => COMPACT,
        };
        // the references are core's, served by `specforge gsd reference` (as the prompt's, `prompts.rs`)
        Ok(gsd_kit::prompt::repoint::repoint(
            body,
            call.host(),
            &|_| true,
        ))
    });
}
