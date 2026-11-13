//! The lines every one of the 44 prompts shows: the earlier regression tests ran the lines of the lifecycle prompts
//! only, so new lines in the other prompts slipped past them. Each prompt is rendered once for `claude-code` on its
//! golden project, and the checks run over all of the text: the shell the Rust side writes parses, no prose is joined
//! to a path, and every `specforge settings` line is one the host accepts.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use regex::Regex;

use super::commands_support::twin;
use super::harness::Project;
use super::prompts_support::{payload, prompt, text_of};
use super::render_prompts::ROWS;

/// `(prompt id, rendered text)` of the 44 prompts.
pub fn rendered() -> &'static [(&'static str, String)] {
    static ALL: OnceLock<Vec<(&'static str, String)>> = OnceLock::new();
    ALL.get_or_init(|| {
        ROWS.iter()
            .map(|row| {
                let p = (row.setup)();
                let text = text_of(&payload(&prompt(&p, row.id, row.args, "claude-code")));
                (row.id, text)
            })
            .collect()
    })
}

/// The rendered text of one prompt.
pub fn text(id: &str) -> &'static str {
    rendered()
        .iter()
        .find(|(i, _)| *i == id)
        .map(|(_, t)| t.as_str())
        .unwrap_or_else(|| panic!("no prompt {id}"))
}

/// `code` parses as bash (`bash -n`).
pub fn parses(code: &str) -> bool {
    let mut child = Command::new("bash")
        .arg("-n")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("bash");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(code.as_bytes())
        .expect("write");
    child.wait().expect("bash -n").success()
}

/// The bash and sh fences of `text`, as the code between the marks, as gsd-kit's Markdown reader reads them.
pub fn bash_fences(text: &str) -> Vec<String> {
    gsd_kit::markdown::fences(text)
        .into_iter()
        .filter(|f| matches!(f.language.as_str(), "bash" | "sh"))
        .map(|f| match f.text.strip_suffix('\n') {
            Some(code) => code.to_string(),
            None => f.text,
        })
        .collect()
}

/// The text a prompt's Rust side appends (`In the shell blocks above, ...` to the end).
fn tail(text: &str) -> &str {
    text.rfind("In the shell blocks above,")
        .map_or("", |at| &text[at..])
}

/// A single-word placeholder (`<paths>`) is a value the agent fills: it stands for one word in the fence. A
/// placeholder with spaces in it is prose, and bash reads it as a redirection.
fn filled(code: &str) -> String {
    Regex::new(r"<[A-Za-z0-9_.-]+>")
        .expect("a regex")
        .replace_all(code, "X")
        .into_owned()
}

#[test]
fn the_shell_a_prompt_writes_itself_parses() {
    let mut checked = 0;
    let mut broken = Vec::new();
    for (id, text) in rendered() {
        for code in bash_fences(tail(text)) {
            checked += 1;
            if !parses(&filled(&code)) {
                broken.push(format!("{id}: {code}"));
            }
        }
    }
    assert!(checked >= 2, "{checked} fences of the Rust side");
    assert!(
        broken.is_empty(),
        "fences that do not parse:\n{}",
        broken.join("\n")
    );
}

/// `the spec` joined to the word or path that follows it (`the specquick/`, `the spec.continue-here.md`), and an
/// entity kind joined to a slug (`the `phase` entities03-auth`): what a blind path-to-prose swap leaves behind.
fn joined() -> Regex {
    Regex::new(r"\bthe spec(?:[A-Za-z]+/|[A-Za-z-]*\.md|\.[A-Za-z])|\)the spec|entit(?:y|ies)[0-9]")
        .expect("a regex")
}

#[test]
fn no_rendered_text_joins_the_spec_to_a_path() {
    let rx = joined();
    let mut found = Vec::new();
    for (id, text) in rendered() {
        for (n, line) in text.lines().enumerate() {
            if rx.is_match(line) {
                found.push(format!("{id}:{n}: {line}"));
            }
        }
    }
    assert!(found.is_empty(), "joined words:\n{}", found.join("\n"));
}

#[test]
fn every_prompt_rendered_once() {
    assert_eq!(rendered().len(), 44);
    let _: Project = twin();
}

fn has(id: &str, needle: &str) {
    assert!(text(id).contains(needle), "{id} lacks `{needle}`");
}

fn lacks(id: &str, needle: &str) {
    assert!(!text(id).contains(needle), "{id} still has `{needle}`");
}

/// The lines a review found: a project is linked to none of its milestones, phases, pauses or lessons, so each
/// of these lists by kind.
#[test]
fn the_project_rooted_lines_list_by_kind() {
    has(
        "onboard",
        "specforge export --kinds project,milestone,phase,requirement --pick '.nodes[]' --fields kind,id",
    );
    has(
        "ingest_docs",
        "specforge export --kinds project,requirement,decision",
    );
    has(
        "ingest_docs",
        "specforge export --kinds milestone,phase,context",
    );
    has("progress", "specforge export --kinds pause");
    has(
        "resume_work",
        "specforge export --kinds pause,handoff,blocker",
    );
    has("milestone_summary", "specforge export --kinds milestone");
    for id in [
        "onboard",
        "ingest_docs",
        "next",
        "progress",
        "resume_work",
        "milestone_summary",
    ] {
        lacks(id, "specforge query \"$PROJECT\" --depth 4");
        lacks(id, "specforge query \"$PROJECT\" --depth 2 --kinds");
    }
}

/// The words of the shell command that starts at `rest`, up to the end of the line or the first `` ` ``, `)`, `;`, `|`,
/// `&` or `#` outside quotes. Quotes are honoured and dropped.
fn shell_words(rest: &str) -> Vec<String> {
    let (mut out, mut word, mut any) = (Vec::new(), String::new(), false);
    let mut quote: Option<char> = None;
    for c in rest.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.push(c),
            (None, '"' | '\'') => {
                quote = Some(c);
                any = true;
            }
            (None, '>') => {
                // `2>/dev/null`: the fd belongs to the redirection
                if word.chars().all(|d| d.is_ascii_digit()) {
                    word.clear();
                }
                break;
            }
            (None, '`' | ')' | ';' | '|' | '&' | '#') => break,
            (None, c) if c.is_whitespace() => {
                if any || !word.is_empty() {
                    out.push(std::mem::take(&mut word));
                    any = false;
                }
            }
            (None, c) => word.push(c),
        }
    }
    if any || !word.is_empty() {
        out.push(word);
    }
    out
}

/// `(prompt, line)` -> the words after `specforge settings ` of every such command a rendered prompt shows.
fn settings_commands() -> Vec<(&'static str, String, Vec<String>)> {
    let mut out = Vec::new();
    for (id, text) in rendered() {
        for line in text.lines() {
            let mut from = 0;
            while let Some(at) = line[from..].find("specforge settings ") {
                let start = from + at + "specforge settings ".len();
                out.push((*id, line.trim().to_string(), shell_words(&line[start..])));
                from = start;
            }
        }
    }
    out
}

/// A placeholder in a key or value the agent fills, as a concrete word the host accepts.
fn concrete(word: &str) -> Option<String> {
    let fill = |w: &str| match w {
        "<runtime>" => "claude",
        "<tier>" => "opus",
        "<cli>" => "codex",
        "<slug>" => "alpha",
        "<provider>" => "anthropic",
        "<budget>" => "high",
        _ => "x",
    };
    let rx = Regex::new(r"<[A-Za-z0-9_-]+>").expect("a regex");
    let word = rx
        .replace_all(word, |c: &regex::Captures<'_>| fill(&c[0]))
        .into_owned();
    (!word.contains('<') && !word.contains('$')).then_some(word)
}

/// Five line shapes that the host refuses or stores wrongly (a block as a bare word, `null` for an
/// enum, a JSON list as one word, a value that starts with `--`). Every `specforge settings set|unset @gsd/core ...`
/// line of the 44 prompts is run with `--dry-run` on the twin.
#[test]
fn every_settings_line_is_accepted_by_the_host() {
    let p = twin();
    let (mut ran, mut bad) = (0, Vec::new());
    let (mut with_json, mut unsets) = (0, 0);
    for (id, line, words) in settings_commands() {
        // `set --value-json <json> @gsd/core <key>` carries its value as the flag's, not as a word after the key
        let (json, words) = match words.as_slice() {
            [verb, flag, json, rest @ ..] if flag == "--value-json" => {
                (concrete(json), [std::slice::from_ref(verb), rest].concat())
            }
            _ => (None, words),
        };
        let [verb, ext, key, values @ ..] = words.as_slice() else {
            continue;
        };
        if !matches!(verb.as_str(), "set" | "unset") || ext != "@gsd/core" {
            continue;
        }
        if verb == "set" && values.is_empty() && json.is_none() {
            continue; // a table row or a mention of the command
        }
        let Some(key) = concrete(key) else { continue };
        let values: Option<Vec<String>> = values.iter().map(|v| concrete(v)).collect();
        let Some(values) = values else { continue };
        if verb == "set" && values == ["null"] {
            bad.push(format!(
                "{id}: `null` for {key} does not clear it, `unset` does: {line}"
            ));
            continue;
        }
        let mut args = vec!["settings", verb.as_str(), "--dry-run"];
        if let Some(json) = &json {
            args.extend(["--value-json", json.as_str()]);
            with_json += 1;
        }
        args.extend(["@gsd/core", key.as_str()]);
        args.extend(values.iter().map(String::as_str));
        unsets += usize::from(verb == "unset");
        let run = p.run(&args);
        ran += 1;
        let text = run.all();
        // a list is written as the words it holds, not as one JSON word
        let stored_as_json_text = text.contains("[\"[") || text.contains("= [\"{");
        if run.code != 0 || stored_as_json_text {
            bad.push(format!("{id}: {line}\n    -> {}", text.trim()));
        }
    }
    assert!(
        with_json >= 2 && unsets >= 3,
        "{with_json} --value-json lines and {unsets} unset lines ran"
    );
    assert!(ran >= 15, "{ran} lines ran");
    assert!(
        bad.is_empty(),
        "lines the host refuses or mis-stores:\n{}",
        bad.join("\n")
    );
}

/// No prompt writes upstream GSD's `~/.gsd/defaults.json`, and none takes an API key in the
/// conversation or `export`s one.
#[test]
fn no_prompt_writes_the_global_defaults_or_takes_a_key() {
    let export = Regex::new(r"^\s*export [A-Z_]*(KEY|TOKEN|SECRET)=").expect("a regex");
    for (id, text) in rendered() {
        for line in text.lines() {
            assert!(!export.is_match(line), "{id}: exports a secret: {line}");
            assert!(
                !(line.contains(".gsd") && line.contains("mkdir")),
                "{id}: makes ~/.gsd: {line}"
            );
        }
    }
    has("settings", "There are no global defaults here");
    lacks("settings", "Save these as default settings");
    has(
        "config",
        "do not ask for the key and do not take it in this conversation",
    );
}

/// A "Parse JSON for:" line that follows `specforge gsd progress-report --format json` may name only
/// fields that command prints (`roadmap_exists` or `manager_flags` decided routing and were never there). The fields
/// are the top-level keys of the real output on the twin.
#[test]
fn parse_json_lines_name_fields_the_command_prints() {
    let p = twin();
    let out = p.run(&["gsd", "progress-report", "--format", "json"]);
    let real: serde_json::Value = serde_json::from_str(&out.stdout).expect("progress-report JSON");
    let keys: Vec<&str> = real
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    let name = Regex::new(r"`([a-z_]+)(?:\[\])?`").expect("a regex");
    let (mut checked, mut bad) = (0, Vec::new());
    for (id, text) in rendered() {
        let lines: Vec<&str> = text.lines().collect();
        for (n, l) in lines.iter().enumerate() {
            if !l.starts_with("Parse JSON for:") {
                continue;
            }
            let from = n.saturating_sub(60);
            if !lines[from..n]
                .iter()
                .any(|c| c.contains("specforge gsd progress-report --format json"))
            {
                continue;
            }
            let fields = l.split('(').next().unwrap_or(l); // the parenthesis explains
            for cap in name.captures_iter(fields) {
                checked += 1;
                if !keys.contains(&&cap[1]) {
                    bad.push(format!("{id}:{n}: `{}` is not in {keys:?}", &cap[1]));
                }
            }
        }
    }
    assert!(checked >= 10, "{checked} fields checked");
    assert!(
        bad.is_empty(),
        "fields the command does not print:\n{}",
        bad.join("\n")
    );
}

fn workflow(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/assets/gsd-core/workflows/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The text mismatches a first fix left behind.
#[test]
fn the_post_fix_text_leftovers_stay_fixed() {
    // progress asks for the next phase of the milestone, padded as phase ids are
    assert!(
        workflow("progress.md").contains("NEXT_PHASE=$(printf %02d $((Z+1))); NEXT_HAS_UI=$(specforge query \"$MILESTONE/$NEXT_PHASE\"")
    );
    // the ingest staging directory holds the documents' text and is removed at the end
    assert!(
        workflow("ingest-docs.md").contains("specforge files remove --recursive \"$INTEL_DIR\"")
    );
    // `[path]` is where the documents are scanned
    assert!(!text("onboard").contains("The project is read from the directory"));
    // the integrations confirmation shows what is written: booleans, and no key at all
    let integrations = workflow("settings-integrations.md");
    assert!(
        !integrations.contains("****<last-4>"),
        "a masked key is shown"
    );
    assert!(!integrations.contains("written plaintext only to `config.json`"));
}

/// The milestone close defers open items the one way the port has: a person accepts the latest audit's tech debt
/// (`debt_accepted`, which gsd/E106 reads); no `audit-open` writer is named and no loop pretends to acknowledge.
#[test]
fn the_milestone_close_defers_through_debt_accepted() {
    let close = workflow("complete-milestone.md");
    let elaboration = workflow("complete-milestone/detail/elaboration.md");
    for text in [&close, &elaboration] {
        assert!(!text.contains("audit-open"));
        assert!(text.contains("debt_accepted"));
    }
    assert!(!elaboration.contains("if ! true"));
}

/// [A] says what it does in all three places: the choice the person sees, the step that follows it and the
/// elaboration. Accepting the audit's tech debt clears gsd/E106 alone, so each names the close's other gate codes as
/// still blocking, and none says "Acknowledge all" or "document as deferred".
#[test]
fn the_milestone_acknowledge_clears_e106_alone() {
    let declared = std::fs::read_to_string(format!(
        "{}/src/commands/milestone.rs",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("milestone.rs");
    let gates: Vec<String> = Regex::new(r#"\.gate\("(E\d+)"\)"#)
        .expect("a regex")
        .captures_iter(&declared)
        .map(|c| format!("gsd/{}", &c[1]))
        .collect();
    assert!(
        gates.contains(&"gsd/E106".to_string()) && gates.len() > 1,
        "{gates:?}"
    );
    let close = workflow("complete-milestone.md");
    let elaboration = workflow("complete-milestone/detail/elaboration.md");
    let choice = close
        .lines()
        .find(|l| l.starts_with("[A] "))
        .expect("the [A] choice");
    let step = close
        .lines()
        .find(|l| l.starts_with("**If user chooses [A]"))
        .expect("the [A] step");
    let section = elaboration
        .split("## § 1")
        .nth(1)
        .and_then(|s| s.split("\n## ").next())
        .expect("§ 1");
    assert!(choice.contains("gsd/E106"), "{choice}");
    for text in [step, section] {
        for gate in &gates {
            assert!(text.contains(gate.as_str()), "names no {gate}: {text}");
        }
    }
    for text in [&close, &elaboration] {
        assert!(!text.contains("Acknowledge all") && !text.contains("document as deferred"));
    }
}

/// No rendered line runs jq, neither an asset's (`port_tools` scans those) nor one the Rust side writes
/// (`src/prompts/`). gh's own `--jq` filter is not jq.
#[test]
fn no_rendered_line_runs_jq() {
    let found: Vec<String> = rendered()
        .iter()
        .flat_map(|(id, text)| {
            text.lines()
                .enumerate()
                .filter(|(_, l)| super::command_word::runs_in_prose(l, "jq"))
                .map(move |(n, l)| format!("{id}:{}: {}", n + 1, l.trim()))
        })
        .collect();
    assert!(found.is_empty(), "lines that run jq:\n{}", found.join("\n"));
}

/// R0 of "SpecForge serves itself": a rendered prompt's shell fences and inline runs run only `specforge` too, as an
/// asset's do (`port_tools`); what still runs another tool is a `prompt:<id>` row of `pending_tools.txt`.
#[test]
fn a_rendered_prompt_calls_only_specforge() {
    let texts: Vec<(String, Vec<String>)> = rendered()
        .iter()
        .map(|(id, text)| {
            (
                format!("prompt:{id}"),
                text.lines().map(String::from).collect(),
            )
        })
        .collect();
    // a function an asset defines may be called from a prompt that does not render its definition
    let assets = super::specforge_only::shell_assets();
    let functions =
        super::specforge_only::functions(texts.iter().chain(&assets).map(|(_, lines)| &lines[..]));

    let hits: Vec<super::specforge_only::Row> = texts
        .iter()
        .flat_map(|(file, lines)| super::specforge_only::hits(file, lines, &functions))
        .collect();
    super::specforge_only::assert_pending(
        &hits,
        super::specforge_only::pending(|f| f.starts_with("prompt:")),
    );
}

/// The distinct graph tools of the engine-tool catalogue (every group but setup) that the 44 claude-code renders and
/// the installed claude-code agent bodies name, by `tool_names::named`: the port's tool measure (ADR 0332). The count
/// may not fall below it, and a rise raises it in the commit that names the new tool.
const AT_LEAST: usize = 12;

#[test]
fn the_graph_tools_the_prompts_name_are_a_ratchet() {
    let p = super::harness::project("empty");
    super::agents::installed(&p, "claude-code");
    // the generated tools section is the catalogue rendered, not a body using it
    let agents = gsd_core::agents::roster::AGENTS.iter().map(|agent| {
        let (_, body) = super::agents::split(&super::agents::claude_agent(&p, agent.id));
        let cut = body.find("\n## SpecForge tools").unwrap_or(body.len());
        (format!("agent:{}", agent.id), body[..cut].to_string())
    });
    let corpus = rendered()
        .iter()
        .map(|(id, text)| (format!("prompt:{id}"), text.clone()))
        .chain(agents);
    let mut counted: Vec<(&'static str, String)> = Vec::new();
    for (source, text) in corpus {
        for tool in super::tool_names::named(&text) {
            if tool.group.graph() && !counted.iter().any(|(name, _)| *name == tool.name) {
                counted.push((tool.name, source.clone()));
            }
        }
    }
    let catalogue = specforge_extension_sdk::engine_tools::ENGINE_TOOLS;
    counted.sort_by_key(|(name, _)| catalogue.iter().position(|t| t.name == *name));
    let set: Vec<String> = counted
        .iter()
        .map(|(name, source)| format!("  {name} (first in {source})"))
        .collect();
    let n = counted.len();
    assert!(
        n >= AT_LEAST,
        "{n} graph tools named, below AT_LEAST = {AT_LEAST}: a tool was lost. Named now:\n{}",
        set.join("\n")
    );
    assert!(
        n <= AT_LEAST,
        "{n} graph tools named: raise AT_LEAST to {n} in this commit. Named now:\n{}",
        set.join("\n")
    );
}

/// The counter's rule: an MCP name counts anywhere, a CLI line only as a command in code.
#[test]
fn a_tool_is_named_by_its_mcp_name_or_a_cli_line_in_code() {
    let names = |text: &str| -> Vec<&str> {
        super::tool_names::named(text)
            .iter()
            .map(|t| t.name)
            .collect()
    };
    assert_eq!(names("Call `specforge.trace` on it."), ["trace"]);
    assert_eq!(names("Then call specforge.edit."), ["edit"]);
    assert!(names("Then run specforge check on the project.").is_empty());
    assert_eq!(names("Then run `specforge check`."), ["validate"]);
    assert_eq!(
        names("```bash\nspecforge query X --inputs gsd/E501\n```\n"),
        ["verdict_inputs"]
    );
    assert_eq!(
        names("```bash\nP=$(specforge query X \\\n  --depth 1)\n# specforge check\n```\n"),
        ["query"]
    );
    assert!(names("Edit `specforge.json` by hand.").is_empty());
}

/// Each line of every asset as the claude-code render shows it: re-pointed and converted, trimmed. The names ledger
/// judges asset lines by their spans; a rendered line outside this set is the prompt's own Rust-authored text.
// shortcut: an asset line with a {{setting:}} or {{arg:}} token renders filled and is not matched; strip the tokens
// from both sides if one starts naming a GSD file
fn asset_lines() -> std::collections::HashSet<String> {
    gsd_core::prompts::asset_paths()
        .flat_map(|path| {
            let text = gsd_core::prompts::asset(path).unwrap_or_default();
            gsd_core::prompts::without_notice(text)
                .lines()
                .map(|line| {
                    gsd_kit::prompt::host::convert("claude-code", &gsd_core::prompts::repoint(line))
                        .trim()
                        .to_string()
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The names ledger over what a prompt's Rust side writes (ADR 0332 D8): a rendered line that is no asset line, no row
/// of the preamble's file table (held equal to the README) and names a GSD file is a `prompt:<id>` row of
/// `planning_names.txt`.
#[test]
fn no_rendered_tail_names_a_gsd_file() {
    let assets = asset_lines();
    let mut hits = Vec::new();
    for (id, text) in rendered() {
        let preamble = text.find("</specforge_gsd_preamble>").unwrap_or(0);
        let lines: Vec<&str> = text.split('\n').collect();
        let mut at = 0;
        let starts: Vec<usize> = lines
            .iter()
            .map(|l| {
                let start = at;
                at += l.len() + 1;
                start
            })
            .collect();
        let source = format!("prompt:{id}");
        hits.extend(super::planning_names::hits(&source, text, |n| {
            let line = lines[n].trim();
            !(line.starts_with('|') && starts[n] < preamble) && !assets.contains(line)
        }));
    }
    super::planning_names::assert_listed(&hits, |source| source.starts_with("prompt:"));
}
