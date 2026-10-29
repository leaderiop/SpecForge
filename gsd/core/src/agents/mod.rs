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

/// The section added after the ported body of an agent that judges rules. The host adds its own "Judged rules" list
/// after it, which names `specforge.verdict_inputs` only; this names the command that records the verdict, so an
/// installed judge can complete the loop (the ported lines still name upstream's tools, `spec/README.md` M6).
pub fn recording(agent: &str) -> String {
    let codes: Vec<String> = crate::judged::JUDGED
        .iter()
        .filter(|j| j.judge == agent)
        .map(|j| format!("`gsd/{}`", j.code))
        .collect();
    if codes.is_empty() {
        return String::new();
    }
    format!(
        "\n## Recording a verdict (@gsd/core)\n\n\
         You judge {}. Where the ported text above says to write a report or a status, record a verdict instead: only a \
         verdict makes the rule's result count, and a status is computed from the verdicts, never written.\n\n\
         1. Read the rule's inputs: `specforge query <entity> --inputs gsd/<code> --format json` (the MCP tool \
         `specforge.verdict_inputs`). Its `basis` object is the exact state you judged.\n\
         2. Judge what the inputs show.\n\
         3. Record it with the command `record_verdict` of `@gsd/core`: `specforge gsd record-verdict <entity> <code> \
         <outcome> <model> '<basis>'`, where `<code>` is the local code (`E501`), `<outcome>` is one of the rule's \
         outcomes (listed below), `<model>` names you and `<basis>` is the `basis` object of step 1, as JSON. Options: \
         `--findings '[{{\"severity\": \"error\", \"message\": \"...\"}}]'`, `--cited '[{{\"file\": \"src/a.js\"}}]'` (cite \
         every file you read: a verdict goes stale when a cited file changes, and not otherwise) and `--values '{{...}}'` \
         for the fields the rule binds.\n\n\
         A verdict recorded over a `basis` that has since changed is refused (`stale_revision`): read the inputs again. \
         The host refuses, whole and with `E080`, a verdict that would add an error the project does not have (a failing \
         outcome on a rule whose severity is an Error) and a failing verdict over a recorded pass. Do not edit the spec \
         to get around the refusal; report the findings to the user instead.\n",
        codes.join(", ")
    )
}

pub fn declare(c: &mut ContributionsBuilder) {
    for agent in roster::AGENTS {
        c.agent(agent.id, |a| {
            a.description(agent.description)
                .tools(agent.tools)
                .color(agent.color)
                .model(&format!(
                    "setting:{}",
                    crate::settings::models::setting(agent.id)
                ))
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
        Ok(format!("{body}{}", recording(agent.id)))
    });
}
