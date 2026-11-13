//! Every SpecForge command an agent body runs is a tool its role grants it (ADR 0334).

use std::collections::BTreeSet;

use gsd_core::agents::roster::AGENTS;
use gsd_kit::markdown::{fences, inline_code};
use specforge_extension_sdk::engine_tools::{EngineGroup, by_cli};

/// The words of each `specforge …` command line in `text`'s fences and code spans, up to the first shell operator.
fn invocations(text: &str) -> Vec<Vec<String>> {
    let code = fences(text).into_iter().map(|f| f.text);
    let spans = inline_code(text).into_iter().map(|(_, s)| s);
    let mut out = Vec::new();
    for snippet in code.chain(spans) {
        // A shell comment says what the commands are; it runs none.
        for line in snippet.lines().filter(|l| !l.trim_start().starts_with('#')) {
            for (start, _) in line.match_indices("specforge ") {
                // A word that ends a path (`bin/specforge`, `.specforge`) is not the command.
                if line[..start].ends_with(|c: char| c.is_alphanumeric() || "/._-".contains(c)) {
                    continue;
                }
                let rest = &line[start + "specforge ".len()..];
                let end = rest
                    .find(['|', ';', '&', ')', '>', '<', '`'])
                    .unwrap_or(rest.len());
                let words: Vec<String> =
                    rest[..end].split_whitespace().map(str::to_string).collect();
                if words
                    .first()
                    .is_some_and(|w| w.starts_with(|c: char| c.is_ascii_lowercase()))
                {
                    out.push(words);
                }
            }
        }
    }
    out
}

/// The `specforge:` class a command line runs: an engine tool by its CLI twin, a `gsd` command by its id; none for a
/// CLI-only command (`git`, `files`, `run`, `http`) or a setup tool (`settings`: no role grants one), which the agent
/// reaches through its shell.
fn class(words: &[String]) -> Option<String> {
    if words[0] == "gsd" {
        return words
            .get(1)
            .map(|id| format!("specforge:gsd.{}", id.replace('-', "_")));
    }
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    by_cli(&words)
        .filter(|tool| tool.group != EngineGroup::Setup)
        .map(|tool| format!("specforge:{}", tool.name))
}

#[test]
fn every_specforge_command_an_agent_runs_is_a_tool_its_role_grants() {
    let (mut checked, mut cli_only) = (0, BTreeSet::new());
    let mut missing = Vec::new();
    for agent in AGENTS {
        let judges = gsd_core::judged::JUDGED.iter().any(|j| j.judge == agent.id);
        let granted = gsd_kit::agent::tools(agent.tools, agent.role, judges);
        let bodies = std::iter::once(agent.full).chain(agent.compact);
        for words in bodies.flat_map(invocations) {
            match class(&words) {
                Some(class) if granted.contains(&class.as_str()) => checked += 1,
                Some(class) => missing.push(format!("{}: {class} ({})", agent.id, words.join(" "))),
                None => {
                    cli_only.insert(words[0].clone());
                }
            }
        }
    }
    assert!(missing.is_empty(), "{missing:#?}");
    // Counted work: a scan that lost commands would pass vacuously, so the count is pinned to the bodies'.
    assert_eq!(checked, 158, "commands checked");
    assert_eq!(
        cli_only.into_iter().collect::<Vec<_>>(),
        ["files", "git", "http", "run", "settings"],
        "a CLI-only command an agent runs: grant its tool, or list it here"
    );
}
