//! The 15 core agents (K9): metadata from [`roster`], bodies ported under MIT from gsd-core v1.18.0 (`assets/agents/`).

pub mod roster;

use specforge_extension_sdk::prelude::*;

pub fn declare(c: &mut ContributionsBuilder) {
    for agent in roster::AGENTS {
        c.agent(agent.id, |a| {
            a.description(agent.description)
                .tools(agent.tools)
                .color(agent.color);
        });
    }
    c.render(|call| {
        let agent = roster::AGENTS
            .iter()
            .find(|a| a.id == call.id())
            .ok_or_else(|| format!("no agent '{}'", call.id()))?;
        Ok(agent.full.to_string())
    });
}
