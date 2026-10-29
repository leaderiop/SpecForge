//! The text every lifecycle prompt renders, swept without the host: for each prompt, host and flag combination, no
//! upstream path or tool name survives (D-r42 black-box invariants), and every name the text points at is known.

use std::collections::BTreeSet;

use gsd_core::prompts::{asset, asset_paths, render_text, repoint, without_notice};
use regex::Regex;

const FORBIDDEN: [&str; 7] = [
    ".planning",
    "gsd-tools",
    "gsd_run",
    "gsd-sdk",
    "gsd.cjs",
    "~/.claude/gsd-core",
    "$HOME/.claude/gsd-core",
];

const HOSTS: [&str; 3] = ["claude-code", "codex", "cursor"];

/// Workflows of the next round (r43): the text may point at them, and the preamble says to skip them.
const R43_COMMANDS: &[&str] = &[
    "ai-integration-phase",
    "autonomous",
    "debug",
    "extract-learnings",
    "import",
    "ingest-docs",
    "manager",
    "map-codebase",
    "next",
    "onboard",
    "progress",
    "quick",
    "review",
    "secure-phase",
    "settings",
    "sketch",
    "spike",
    "ui-phase",
    "ui-review",
];

/// Commands of GSD that `@gsd/core` does not carry, or that name a thing that is not a command.
const NON_CORE_COMMANDS: &[&str] = &[
    "alternative-1",
    "alternative-2",
    "discuss-milestone",
    "research-phase",
    "section",
    "transition",
];

/// Files of upstream's tree that r43 ports: a rendered text may name them as "not installed yet".
const R43_PATHS: &[&str] = &[
    "gsd-core/bin/lib/edge-probe.cjs",
    "gsd-core/templates/research-project/",
    "gsd-core/workflows/node-repair.md",
];

/// Every prompt as `(command stem, argument texts)`: none, every flag, every enum value, text and integer options.
fn prompts() -> Vec<(String, Vec<String>)> {
    let declaration = gsd_core::build().declaration();
    declaration
        .surfaces
        .prompts
        .iter()
        .map(|p| {
            let mut base = Vec::new();
            for _ in p.args.iter().filter(|a| a.required) {
                base.push("03".to_string());
            }
            let mut texts = vec![base.join(" ")];
            let mut flags = base.clone();
            for a in &p.args {
                let option = format!("--{}", a.name.replace('_', "-"));
                match a.field_type.as_str() {
                    "bool" => flags.push(option),
                    "integer" => flags.push(format!("{option} 2")),
                    "string" => flags.push(format!("{option} \"text\"")),
                    "enum" => {
                        for value in &a.enum_values {
                            let mut one = base.clone();
                            one.push(format!("{option} {value}"));
                            texts.push(one.join(" "));
                        }
                    }
                    _ => {}
                }
            }
            texts.push(flags.join(" "));
            (p.id.replace('_', "-"), texts)
        })
        .collect()
}

/// `(command, host, arguments, text)` for the whole sweep.
fn sweep() -> Vec<(String, &'static str, String, String)> {
    let mut all = Vec::new();
    for (command, texts) in prompts() {
        for host in HOSTS {
            for arguments in &texts {
                let text = render_text(&command, host, arguments)
                    .unwrap_or_else(|e| panic!("{command} on {host} with '{arguments}': {e}"));
                all.push((command.clone(), host, arguments.clone(), text));
            }
        }
    }
    all
}

fn assert_clean(label: &str, text: &str) {
    for forbidden in FORBIDDEN {
        assert!(!text.contains(forbidden), "{label} names {forbidden}");
    }
    let mut rest = text;
    while let Some(at) = rest.find("spec/gsd/") {
        let token: String = rest[at..]
            .chars()
            .take_while(|c| !c.is_whitespace())
            .collect();
        assert!(!token.contains(".md"), "{label} names {token}");
        rest = &rest[at + 9..];
    }
}

#[test]
fn no_planning_paths_in_rendered_text() {
    let all = sweep();
    assert_eq!(prompts().len(), 17);
    assert!(all.len() >= 17 * 3 * 2, "{} renders", all.len());
    for (command, host, arguments, text) in &all {
        assert_clean(&format!("{command} on {host} with '{arguments}'"), text);
    }
}

#[test]
fn every_reference_answer_is_clean() {
    let mut seen = 0;
    for path in asset_paths() {
        let text = asset(path).expect("an asset");
        assert_clean(path, &repoint(without_notice(text)));
        seen += 1;
    }
    assert!(seen > 150, "{seen} assets");
}

/// The command names (`plan-phase`) the text writes as `/gsd:plan-phase` or `/gsd-plan-phase`.
fn slash_names(text: &str) -> BTreeSet<String> {
    let rx = Regex::new(r"(?:^|[\s`(\[])/gsd[:-]([a-z][a-z0-9-]*)").expect("a regex");
    rx.captures_iter(text)
        .map(|c| c[1].trim_end_matches('-').to_string())
        .collect()
}

#[test]
fn every_slash_command_a_prompt_names_is_known() {
    let declaration = gsd_core::build().declaration();
    let mut known: BTreeSet<String> = declaration
        .surfaces
        .slash_commands
        .iter()
        .filter_map(|s| s.prompt.as_deref())
        .map(|p| p.replace('_', "-"))
        .collect();
    known.extend(
        declaration
            .surfaces
            .commands
            .iter()
            .map(|c| c.id.replace('_', "-")),
    );
    known.extend(
        R43_COMMANDS
            .iter()
            .chain(NON_CORE_COMMANDS)
            .map(|s| s.to_string()),
    );
    let mut unknown = BTreeSet::new();
    for (command, host, _, text) in sweep().iter().filter(|(_, h, _, _)| *h == "claude-code") {
        for name in slash_names(text) {
            if !known.contains(&name) {
                unknown.insert(format!("{name} (in {command} on {host})"));
            }
        }
    }
    assert!(unknown.is_empty(), "unknown slash commands: {unknown:#?}");
}

#[test]
fn every_referenced_path_is_embedded_or_listed_for_r43() {
    let rx = Regex::new(r"specforge gsd reference (gsd-core/[A-Za-z0-9_./-]+)`( \(not installed by @gsd/core yet\))?")
        .expect("a regex");
    let mut later = BTreeSet::new();
    let mut seen = 0;
    for (command, _, _, text) in sweep().iter().filter(|(_, h, _, _)| *h == "claude-code") {
        for cap in rx.captures_iter(text) {
            seen += 1;
            let path = cap[1].trim_end_matches('.');
            let embedded = asset(path).is_some();
            assert_eq!(
                embedded,
                cap.get(2).is_none(),
                "{command}: {path} is marked wrongly"
            );
            if !embedded {
                later.insert(path.to_string());
            }
        }
    }
    assert!(seen > 20, "{seen} references");
    let listed: BTreeSet<String> = R43_PATHS.iter().map(|s| s.to_string()).collect();
    assert_eq!(later, listed, "files not installed yet");
}

/// close-milestone keeps the requirements (REVIEW-r42 C1): upstream deletes REQUIREMENTS.md after archiving it, but the
/// entities are the milestone's history (W506 and the audit read them), so no rendered prompt may tell an agent to
/// delete, remove or `git rm` one.
#[test]
fn no_prompt_tells_the_agent_to_delete_a_requirement() {
    let rx = Regex::new(r"(?i)\b(delet\w*|remov\w*|git rm|rm -f?)\b").expect("a regex");
    let mut seen = 0;
    for (command, host, _, text) in sweep()
        .iter()
        .filter(|(c, _, _, _)| matches!(c.as_str(), "complete-milestone" | "new-milestone"))
    {
        seen += 1;
        for line in text.lines() {
            let lower = line.to_lowercase();
            assert!(
                !(lower.contains("requirement") && rx.is_match(line)),
                "{command} on {host} deletes a requirement: {line}"
            );
        }
    }
    assert!(seen >= 4, "{seen} renders");
}

/// A setting a workflow reads is the setting (REVIEW-r42 M1), never a default written into the line: no render keeps
/// a token or says "this is its default", and the settings the lines read are keys of the roster.
#[test]
fn no_render_keeps_a_setting_token_or_a_literal_default() {
    for (command, host, arguments, text) in sweep() {
        assert!(!text.contains("{{setting:"), "{command} on {host}: a token");
        assert!(
            !text.contains("read it from the settings block"),
            "{command} on {host} with '{arguments}'"
        );
    }
}

/// Keys of upstream's config that `@gsd/core` does not declare (the roster is the tracked manifest's, and the
/// ephemeral chain flag is runtime state): a line that reads one always takes its fallback, as `config-get` of an unset
/// key did.
const NOT_SETTINGS: &[&str] = &[
    "commit_docs",
    "features.graduation",
    "features.graduation_threshold",
    "features.graduation_window",
    "workflow._auto_chain_active",
    "workflow.code_review_command",
    "workflow.live_dom_uat",
];

#[test]
fn every_setting_a_line_reads_is_a_key_of_the_roster() {
    let rx = Regex::new(r"\{\{setting:([a-z_.]+)\|").expect("a regex");
    let roster: BTreeSet<&str> = gsd_core::settings::key_names().collect();
    let mut seen = 0;
    for path in asset_paths() {
        for cap in rx.captures_iter(asset(path).expect("an asset")) {
            seen += 1;
            let known = roster.contains(&cap[1]) || NOT_SETTINGS.contains(&&cap[1]);
            assert!(known, "{path}: '{}' is not a setting", &cap[1]);
        }
    }
    assert!(seen >= 50, "{seen} tokens");
}
