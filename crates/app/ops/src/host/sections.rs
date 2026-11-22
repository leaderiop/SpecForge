//! The sections a host adds after the body an extension renders (ADR 0334 D6), in one order: the SpecForge tools the
//! contribution is granted, then the context route an agent reads (ADR 0356), then the rules an agent judges, so every
//! later section names tools the first lists.

use specforge_protocol_types::ContextTier;

use super::engine::EngineLine;
use crate::view::ProjectView;

/// The `## SpecForge tools` section: each tool's MCP name, its CLI line (or "MCP only") and when to use it; nothing
/// for no tool.
pub fn tools_section(lines: &[EngineLine]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "\n## SpecForge tools\n\nCall these through the SpecForge MCP server by the tool's name, or run the CLI line \
         in a shell: both do the same thing.\n\n",
    );
    for line in lines {
        let cli = if line.cli.is_empty() {
            "MCP only".to_string()
        } else {
            let twins: Vec<String> = line.cli.iter().map(|c| format!("`{c}`")).collect();
            format!("CLI: {}", twins.join(", "))
        };
        out.push_str(&format!("- `{}` ({cli}): {}\n", line.mcp, line.use_when));
    }
    out
}

/// The section a host adds after the body of an agent that judges rules: each rule, the tool that reads its
/// inputs and the outcomes it may answer.
pub(crate) fn judged_section(view: &ProjectView<'_>, agent: &str) -> String {
    let rules = view.registries().judged_rules_of(agent);
    if rules.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n## Judged rules\n\n");
    for rule in rules {
        out.push_str(&format!(
            "- `{}`: read the inputs with `specforge.verdict_inputs`; the outcomes are {} (passing: {}).\n",
            rule.code,
            rule.outcomes.join(", "),
            rule.passing.join(", ")
        ));
    }
    out
}

/// The section a host adds after the body of an agent a context route names (ADR 0356, ADR 0386): the route, its
/// subject kind, its sections in order (core marked), that the core is served whole and, when it has enrich sections,
/// the allowance they fill and how to read what did not fit, how to read a pack too large for the agent, and the CLI
/// line, the MCP tool and the resource that read the pack; nothing for an agent no route names.
pub(crate) fn context_section(view: &ProjectView<'_>, agent: &str) -> String {
    let Some(route) = view.registries().contexts().of_agent(agent) else {
        return String::new();
    };
    let order: Vec<String> = route
        .sections
        .iter()
        .map(|section| match section.tier {
            ContextTier::Core => format!("{} (core)", section.name),
            ContextTier::Enrich => section.name.clone(),
        })
        .collect();
    let order = order.join(", ");
    // The core is served whole whatever it costs (ADR 0386 D1): a brief read is how an agent shrinks one too large.
    let brief = format!(
        "`budget.estimated_tokens` is what the pack costs: when that is more than you can read, read it brief \
         (`specforge context {} <id> --format brief`; MCP: `format` `brief`), each entity's id, kind and title, then \
         query the entities you need.",
        route.name
    );
    let fit = if route
        .sections
        .iter()
        .any(|section| section.tier == ContextTier::Enrich)
    {
        format!(
            "Sections, in order: {order}. The core sections are served whole; the others fill up to {} tokens beyond \
             them, in order.\n\n\
             Cite the file digests it lists; `budget.truncated_sections` names what did not fit: read it again with a \
             `max_tokens` above its `budget.budget_tokens` when you need it. {brief}\n",
            route.budget
        )
    } else {
        format!(
            "Sections, in order: {order}, every one served whole whatever it costs, so no `max_tokens` shrinks it.\n\n\
             Cite the file digests it lists. {brief}\n"
        )
    };
    format!(
        "\n## Context\n\nBefore you start, read the context of the `{subject}` you were given: `specforge context {name} \
         <id>` (MCP: the tool `specforge.context`, or the resource `specforge://context/<id>?role={name}`).\n\n{fit}",
        subject = route.subject,
        name = route.name,
    )
}

/// Every section an agent's body is followed by, in order.
pub(crate) fn agent_sections(view: &ProjectView<'_>, agent: &str, lines: &[EngineLine]) -> String {
    tools_section(lines) + &context_section(view, agent) + &judged_section(view, agent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[specforge_test_macros::test(requirement = "FEATURE-013")]
    fn the_tools_section_names_each_tool_its_cli_and_when_to_use_it() {
        assert_eq!(tools_section(&[]), "");
        let line = |mcp: &str, cli: &[&str], use_when: &str| EngineLine {
            mcp: mcp.into(),
            cli: cli.iter().map(|c| (*c).to_string()).collect(),
            use_when: use_when.into(),
        };
        let section = tools_section(&[
            line("specforge.query", &["specforge query"], "Read it."),
            line("specforge.search", &[], "Find it."),
            line(
                "specforge.edit",
                &["specforge new", "specforge set"],
                "Edit it.",
            ),
        ]);
        assert_eq!(
            section,
            "\n## SpecForge tools\n\nCall these through the SpecForge MCP server by the tool's name, or run the CLI \
             line in a shell: both do the same thing.\n\n\
             - `specforge.query` (CLI: `specforge query`): Read it.\n\
             - `specforge.search` (MCP only): Find it.\n\
             - `specforge.edit` (CLI: `specforge new`, `specforge set`): Edit it.\n"
        );
    }
}
