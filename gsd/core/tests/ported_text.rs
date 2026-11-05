//! The text every lifecycle prompt renders, swept without the host: for each prompt, host and flag combination, no
//! upstream path or tool name survives (D-r42 black-box invariants), and every name the text points at is known.

use std::collections::BTreeSet;
use std::path::Path;

use gsd_core::prompts::{asset, asset_paths, render_text, repoint, without_notice};
use regex::Regex;
use roster::ROSTER;

#[path = "common/roster.rs"]
mod roster;

const FORBIDDEN: [&str; 9] = [
    ".planning",
    "gsd-tools",
    "gsd_run",
    "gsd-sdk",
    "gsd.cjs",
    "~/.claude/gsd-core",
    "$HOME/.claude/gsd-core",
    // The security branches read @gsd/security's gate, not a SECURITY.md file.
    "SECURITY_FILE",
    "-SECURITY.md",
];

const HOSTS: [&str; 3] = ["claude-code", "codex", "cursor"];

/// Names GSD's own text writes after `/gsd-` that are no command of v1.19.0 (its stale mentions and placeholders).
const NOT_COMMANDS: &[&str] = &[
    "alternative-1",
    "alternative-2",
    "discuss-milestone",
    "research-phase",
    "settings-advanced",
    "transition",
];

/// Files of upstream's tree that no ticket of r34-r44 ports: a rendered text names them as "not part of @gsd/core".
const NOT_PORTED: &[&str] = &[
    "gsd-core/templates/research-project/",
    "gsd-core/workflows/node-repair.md",
];

/// One render's input: the argument text and every declared name with its placeholder value (`args::values`).
type Combo = (String, Vec<(String, Option<String>)>);

/// Every prompt as `(command stem, combinations)`: none, every flag, every enum value, text and integer options.
fn prompts() -> Vec<(String, Vec<Combo>)> {
    let declaration = gsd_core::build().declaration();
    declaration
        .surfaces
        .prompts
        .iter()
        .map(|p| {
            // `(name, words, value)` of each given argument
            let combo = |given: &[(String, String, String)]| -> Combo {
                let words: Vec<&str> = given.iter().map(|(_, w, _)| w.as_str()).collect();
                let values = p
                    .args
                    .iter()
                    .map(|a| {
                        let value = given.iter().find(|(n, _, _)| *n == a.name);
                        (a.name.clone(), value.map(|(_, _, v)| v.clone()))
                    })
                    .collect();
                (words.join(" "), values)
            };
            let mut base = Vec::new();
            for a in p.args.iter().filter(|a| a.required) {
                base.push((a.name.clone(), "03".to_string(), "03".to_string()));
            }
            let mut texts = vec![combo(&base)];
            let mut flags = base.clone();
            for a in &p.args {
                let option = format!("--{}", a.name.replace('_', "-"));
                let name = a.name.clone();
                match a.field_type.as_str() {
                    "bool" => flags.push((name, option, "true".to_string())),
                    "integer" => flags.push((name, format!("{option} 2"), "2".to_string())),
                    "string" => {
                        flags.push((name, format!("{option} \"text\""), "text".to_string()))
                    }
                    "enum" => {
                        for value in &a.enum_values {
                            let mut one = base.clone();
                            one.push((name.clone(), format!("{option} {value}"), value.clone()));
                            texts.push(combo(&one));
                        }
                    }
                    _ => {}
                }
            }
            texts.push(combo(&flags));
            (p.id.replace('_', "-"), texts)
        })
        .collect()
}

/// `(command, host, arguments, text)` for the whole sweep.
fn sweep() -> Vec<(String, &'static str, String, String)> {
    let mut all = Vec::new();
    for (command, combos) in prompts() {
        for host in HOSTS {
            for (arguments, values) in &combos {
                let values: Vec<(&str, Option<String>)> = values
                    .iter()
                    .map(|(n, v)| (n.as_str(), v.clone()))
                    .collect();
                let text = render_text(&command, host, arguments, &values)
                    .unwrap_or_else(|e| panic!("{command} on {host} with '{arguments}': {e}"));
                all.push((command.clone(), host, arguments.clone(), text));
            }
        }
    }
    all
}

fn assert_clean(label: &str, text: &str) {
    // the user's own profile file is not an asset: its path stays
    let text = text.replace("/.claude/gsd-core/USER-PROFILE.md", "");
    let text = text.as_str();
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
fn forty_four_prompts_are_swept() {
    assert_eq!(prompts().len(), 44);
}

#[test]
fn no_planning_paths_in_rendered_text() {
    let all = sweep();
    assert!(prompts().len() >= 17, "r42's 17 prompts and every r43 one");
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

/// The names written as `/gsd:name`: skills are installed as `gsd-<name>` (r41), so a colon name is no skill.
fn colon_names(text: &str) -> BTreeSet<String> {
    let rx = Regex::new(r"(?:^|[\s`(\[])/gsd:([a-z][a-z0-9-]*)").expect("a regex");
    rx.captures_iter(text)
        .map(|c| c[1].trim_end_matches('-').to_string())
        .collect()
}

#[test]
fn every_slash_command_a_prompt_names_is_known() {
    // a name is a row of upstream's 72-command roster (D-r43-14), served here or by a named extension
    let known: BTreeSet<String> = ROSTER
        .iter()
        .map(|(n, _)| n.to_string())
        .chain(NOT_COMMANDS.iter().map(|s| s.to_string()))
        .collect();
    let mut unknown = BTreeSet::new();
    for (command, host, _, text) in sweep().iter().filter(|(_, h, _, _)| *h == "claude-code") {
        for name in slash_names(text) {
            if !known.contains(&name) {
                unknown.insert(format!("{name} (in {command} on {host})"));
            }
        }
        // claude-code gets upstream's install: `/gsd:x` is `/gsd-x`, and a colon name left is not a command at all
        for name in colon_names(text) {
            if !NOT_COMMANDS.contains(&name.as_str()) {
                unknown.insert(format!("/gsd:{name} (in {command} on {host})"));
            }
        }
    }
    assert!(unknown.is_empty(), "unknown slash commands: {unknown:#?}");
}

#[test]
fn every_referenced_path_is_embedded() {
    let rx = Regex::new(
        r"specforge gsd reference (gsd-core/[A-Za-z0-9_./-]+)`( \(not part of @gsd/core\))?",
    )
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
    let listed: BTreeSet<String> = NOT_PORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(later, listed, "files no workflow embeds");
}

/// close-milestone keeps the requirements: upstream deletes REQUIREMENTS.md after archiving it, but the
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

/// A setting a workflow reads is the setting, never a default written into the line: no render keeps
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

/// upstream installs a workflow without its `<!-- gsd:section -->` marker lines, and without the runtime notes of
/// another runtime.
#[test]
fn a_claude_render_has_no_section_markers_and_no_copilot_notes() {
    let mut seen = 0;
    for (command, _, _, text) in sweep().iter().filter(|(_, h, _, _)| *h == "claude-code") {
        seen += 1;
        assert!(!text.contains("gsd:section"), "{command}: a section marker");
        assert!(
            !text.contains("<runtime_note runtime=\"copilot\""),
            "{command}: a copilot note"
        );
        assert!(
            !text.contains("Copilot (VS Code)"),
            "{command}: a copilot section"
        );
    }
    assert!(seen > 17, "{seen} renders");
}

/// No text removes a phase on the normal path: phases of a completed milestone stay with it (ids are
/// milestone-scoped), so `phase --remove` and `--unlink-references` appear only in review-backlog's 999.x case. Scans
/// every asset (what `reference` serves) and every rendered prompt.
#[test]
fn no_text_tells_the_agent_to_remove_a_phase() {
    let rx = Regex::new(r"phase --remove|--unlink-references").expect("a regex");
    let backlog = |line: &str| line.contains("999.x");
    for path in asset_paths() {
        for line in asset(path).expect("an asset").lines() {
            assert!(
                !rx.is_match(line) || backlog(line),
                "{path} removes a phase: {line}"
            );
        }
    }
    for (command, host, _, text) in sweep() {
        for line in text.lines() {
            assert!(
                !rx.is_match(line) || backlog(line),
                "{command} on {host} removes a phase: {line}"
            );
        }
    }
}

/// the threat-ID gate reads `threat-ids`, whose answer holds `threat_id_duplicate_count`.
#[test]
fn the_threat_id_gate_reads_threat_ids() {
    // exit 2 is "not loaded": no duplicates; any other failure is an error the text must not read as none
    let line = "THREAT_IDS_JSON=$(specforge gsd threat-ids \"$PHASE_ID\" --format json 2>/dev/null || if [ $? -eq 2 ]; then echo '{\"threat_id_duplicates\":[],\"threat_id_duplicate_count\":0}'; else echo '{\"threat_id_duplicates\":[],\"threat_id_duplicate_count\":null,\"error\":\"threat-ids failed\"}'; fi)";
    for workflow in ["execute-phase", "plan-phase"] {
        let path = format!(
            "{}/assets/gsd-core/workflows/{workflow}.md",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read_to_string(&path).expect("a workflow");
        assert!(
            text.contains(&format!("`{line}`")),
            "{workflow}: no THREAT_IDS_JSON line"
        );
        assert!(
            text.contains("`threat_id_duplicate_count` from `THREAT_IDS_JSON`"),
            "{workflow}: does not parse the count from THREAT_IDS_JSON"
        );
    }
}

/// Every embedded asset of @gsd/core and @gsd/security as `(path under assets/, text)`.
fn asset_files() -> Vec<(String, String)> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).expect("a directory").flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else if path.extension().is_some_and(|e| e == "md") {
                let rel = path.strip_prefix(base).expect("under the base");
                let text = std::fs::read_to_string(&path).expect("an asset");
                out.push((rel.to_string_lossy().replace('\\', "/"), text));
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    for dir in ["assets", "../security/assets"] {
        let base = root.join(dir);
        walk(&base, &base, &mut out);
    }
    out
}

/// The 1-based lines of `text` inside a `bash`/`sh` fence, with their text.
fn shell_lines(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut open = false;
    for (i, line) in text.split('\n').enumerate() {
        let mark = line.trim_start();
        if open && mark.starts_with("```") {
            open = false;
        } else if open {
            out.push((i + 1, line));
        } else if mark == "```bash" || mark == "```sh" {
            open = true;
        }
    }
    out
}

/// No placeholder survives a render: on every host and flag combination, each `{{arg:}}` names a declared argument.
#[test]
fn no_render_keeps_an_arg_token() {
    for (command, host, arguments, text) in sweep() {
        assert!(
            !text.contains("{{arg:"),
            "{command} on {host} with '{arguments}': a token"
        );
    }
}

/// A placeholder's value is escaped for double quotes, so it stands only inside them, in a shell fence (ADR 0340 D4).
#[test]
fn an_arg_token_sits_in_double_quotes_in_a_shell_fence() {
    for (path, text) in asset_files() {
        let fenced: BTreeSet<usize> = shell_lines(&text).into_iter().map(|(n, _)| n).collect();
        for (i, line) in text.split('\n').enumerate() {
            for (at, _) in line.match_indices("{{arg:") {
                assert!(
                    fenced.contains(&(i + 1)),
                    "{path}:{}: outside a shell fence",
                    i + 1
                );
                let before = &line[..at];
                let quotes = before
                    .char_indices()
                    .filter(|&(j, c)| c == '"' && !before[..j].ends_with('\\'))
                    .count();
                assert!(
                    quotes % 2 == 1,
                    "{path}:{}: not in double quotes: {line}",
                    i + 1
                );
            }
        }
    }
}

/// `reference` fills no placeholder: only a file a prompt inlines may hold one (ADR 0340 D4, D5).
#[test]
fn no_file_read_by_reference_holds_an_arg_token() {
    let rx = Regex::new(r#"<file path="([^"]+)">"#).expect("a regex");
    let mut inlined: BTreeSet<String> = BTreeSet::new();
    for (command, _, _, text) in sweep().iter().filter(|(_, h, _, _)| *h == "claude-code") {
        inlined.insert(format!("commands/gsd/{command}.md"));
        inlined.extend(rx.captures_iter(text).map(|c| c[1].to_string()));
    }
    for (path, text) in asset_files() {
        if text.contains("{{arg:") {
            assert!(
                inlined.contains(&path),
                "{path} is read through reference and holds an arg token"
            );
        }
    }
}

/// The files whose shell fences still read the argument text (`$ARGUMENTS`, `$GSD_ARGS`) the render splices: each is
/// rewritten to read its typed values through `{{arg:}}` (ADR 0340 D6).
const ARGS_PENDING: &[&str] = &[
    "gsd-core/workflows/add-tests.md",
    "gsd-core/workflows/audit-milestone.md",
    "gsd-core/workflows/complete-milestone.md",
    "gsd-core/workflows/complete-milestone/detail/elaboration.md",
    "gsd-core/workflows/diagnose-issues.md",
    "gsd-core/workflows/discuss-phase-assumptions.md",
    "gsd-core/workflows/discuss-phase.md",
    "gsd-core/workflows/discuss-phase/modes/chain.md",
    "gsd-core/workflows/execute-phase.md",
    "gsd-core/workflows/execute-phase/steps/codebase-drift-gate.md",
    "gsd-core/workflows/execute-phase/steps/verify-phase-goal.md",
    "gsd-core/workflows/execute-plan.md",
    "gsd-core/workflows/extract-learnings.md",
    "gsd-core/workflows/manager.md",
    "gsd-core/workflows/milestone-summary.md",
    "gsd-core/workflows/new-project.md",
    "gsd-core/workflows/plan-phase.md",
    "gsd-core/workflows/plan-phase/steps/auto-advance-check.md",
    "gsd-core/workflows/plan-phase/steps/chunked-planning-mode.md",
    "gsd-core/workflows/plan-phase/steps/closed-phase-gate.md",
    "gsd-core/workflows/progress.md",
    "gsd-core/workflows/quick.md",
    "gsd-core/workflows/resume-project.md",
    "gsd-core/workflows/review.md",
    "gsd-core/workflows/secure-phase.md",
    "gsd-core/workflows/ship.md",
    "gsd-core/workflows/transition.md",
    "gsd-core/workflows/ultraplan-phase.md",
    "gsd-core/workflows/validate-phase.md",
    "gsd-core/workflows/verify-work.md",
    "gsd-core/workflows/verify-work/steps/mvp-uat-framing.md",
];

const TERMINATORS: [&str; 2] = ["GSD_ARGS_END", "GSD_QUICK_BATCH_ARGS_END"];

/// The ratchet of ADR 0340 D6: a shell line that reads the argument text is in a file of `ARGS_PENDING`, every listed
/// file still has one, and the capture heredocs left open and close only in the form the render stamps (#5637).
#[test]
fn no_shell_block_reads_the_raw_argument_text() {
    let reads = Regex::new(r"\$\{?(ARGUMENTS|GSD_ARGS)\b").expect("a regex");
    let opener =
        Regex::new(r#"<<[-~]?\s*["']?(GSD_(?:QUICK_BATCH_)?ARGS_END)\b"#).expect("a regex");
    let mut reading = BTreeSet::new();
    for (path, text) in asset_files() {
        for (n, line) in shell_lines(&text) {
            if reads.is_match(line) {
                assert!(
                    ARGS_PENDING.contains(&path.as_str()),
                    "{path}:{n} reads the argument text: {line}"
                );
                reading.insert(path.clone());
            }
        }
        let (mut opened, mut closed) = (0, 0);
        for (i, line) in text.lines().enumerate() {
            if let Some(token) = opener.captures(line).map(|c| c[1].to_string()) {
                assert!(
                    line.contains(&format!("<<'{token}'")),
                    "{path}:{} opens {token} in a form the render does not stamp",
                    i + 1
                );
                opened += 1;
            }
            if TERMINATORS.contains(&line.trim()) {
                assert!(
                    TERMINATORS.contains(&line),
                    "{path}:{} closes with an indented or padded line",
                    i + 1
                );
                closed += 1;
            }
        }
        assert_eq!(opened, closed, "{path}: every opener has its closing line");
    }
    let listed: BTreeSet<String> = ARGS_PENDING.iter().map(|p| p.to_string()).collect();
    assert_eq!(
        reading, listed,
        "ARGS_PENDING lists exactly the files that still read"
    );
}

/// The host refuses an option its prompt does not declare, so a dispatch (`Skill(skill="gsd-<x>", args="…")`, also
/// escaped inside an `Agent(prompt=…)`) forwards only `<x>`'s options, and `CONVERGENCE_ARGS` only
/// `plan_review_convergence`'s (ADR 0340 D7). `gsd-code-review` belongs to an extension that is not built.
#[test]
fn every_dispatch_forwards_only_options_its_prompt_declares() {
    let declaration = gsd_core::build().declaration();
    let options = |id: &str| -> Option<BTreeSet<String>> {
        let prompt = declaration.surfaces.prompts.iter().find(|p| p.id == id)?;
        Some(
            prompt
                .args
                .iter()
                .map(|a| a.name.replace('_', "-"))
                .collect(),
        )
    };
    let dispatch =
        Regex::new(r#"Skill\(skill=\\?["']gsd-([a-z-]+)\\?["'],\s*args=\\?["'](.*?)\\?["']\)"#)
            .expect("a regex");
    let appended =
        Regex::new(r#"CONVERGENCE_ARGS="\$\{CONVERGENCE_ARGS\} ([^"]*)""#).expect("a regex");
    let option = Regex::new(r"--([a-z][a-z0-9-]*)|GSD_WS").expect("a regex");
    let mut offenders = BTreeSet::new();
    let mut seen = 0;
    for (path, text) in asset_files() {
        for (i, line) in text.split('\n').enumerate() {
            let mut check = |target: &str, args: &str| {
                let declared = options(target)
                    .unwrap_or_else(|| panic!("{path}:{}: no prompt {target}", i + 1));
                for word in option.find_iter(args) {
                    let name = word.as_str().trim_start_matches("--");
                    if name == "GSD_WS" || !declared.contains(name) {
                        offenders.insert(format!("{path}:{} {name}", i + 1));
                    }
                }
            };
            for cap in dispatch.captures_iter(line) {
                seen += 1;
                if &cap[1] != "code-review" {
                    check(&cap[1].replace('-', "_"), &cap[2]);
                }
            }
            for cap in appended.captures_iter(line) {
                check("plan_review_convergence", &cap[1]);
            }
        }
    }
    assert!(seen > 30, "{seen} dispatches");
    assert!(offenders.is_empty(), "{offenders:#?}");
}
