//! The 15 core agents (K9): metadata from [`roster`], bodies ported under MIT from gsd-core v1.18.0 (`assets/agents/`).

pub mod roster;

use specforge_extension_sdk::prelude::*;

/// The notice every ported agent carries: its upstream file, version and licence.
pub fn attribution(file: &str) -> Attribution {
    Attribution {
        upstream: "gsd-core".to_string(),
        path: format!("agents/{file}"),
        version: "v1.18.0".to_string(),
        license: "MIT".to_string(),
        copyright: "Copyright (c) 2026 Open GSD".to_string(),
    }
}

/// The enum setting that picks an agent's body: `full` or `compact` (see `settings::profiles`).
pub const VARIANT_SETTING: &str = "specforge.agent_variant";

pub fn declare(c: &mut ContributionsBuilder) {
    for agent in roster::AGENTS {
        c.agent(agent.id, |a| {
            a.description(agent.description)
                .tools(agent.tools)
                .color(agent.color)
                .attribution(attribution(&format!("{}.md", agent.id)));
            if agent.compact.is_some() {
                a.variant("compact").variant_setting(VARIANT_SETTING);
            }
        });
    }
    c.render(|call| {
        let agent = roster::AGENTS
            .iter()
            .find(|a| a.id == call.id())
            .ok_or_else(|| format!("no agent '{}'", call.id()))?;
        // Upstream installs no compact agent for Claude Code (`capability.json`), so it always gets the full body.
        let body = match (call.host(), call.variant(), agent.compact) {
            ("claude-code", _, _) | (_, None, _) | (_, _, None) => agent.full,
            (_, Some(_), Some(compact)) => compact,
        };
        Ok(body.to_string())
    });
}
