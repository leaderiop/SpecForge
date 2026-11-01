//! The lines every one of the 44 prompts shows (REVIEW-r43): r42's regression tests ran the lines of the lifecycle
//! prompts only, so r43's new lines slipped past them. Each prompt is rendered once for `claude-code` on its golden
//! project, and the checks run over all of the text: the shell the Rust side writes parses, no prose is joined to a path,
//! and every `specforge settings` line is one the host accepts.

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

/// The bash and sh fences of `text`, as the code between the marks.
pub fn bash_fences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut open: Option<Vec<&str>> = None;
    for line in text.lines() {
        let mark = line.trim_start();
        match &mut open {
            Some(code) if mark.starts_with("```") => {
                out.push(code.join("\n"));
                open = None;
            }
            Some(code) => code.push(line),
            None if mark == "```bash" || mark == "```sh" => open = Some(Vec::new()),
            None => {}
        }
    }
    out
}

/// The text a prompt's Rust side appends (`In the shell blocks above, ...` to the end).
fn tail(text: &str) -> &str {
    text.rfind("In the shell blocks above,")
        .map_or("", |at| &text[at..])
}

/// A single-word placeholder (`<paths>`) is a value the agent fills: it stands for one word in the fence. A
/// placeholder with spaces in it is prose, and bash reads it as a redirection (REVIEW-r43 Minor 2).
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

/// The lines REVIEW-r43 M2 found: a project is linked to none of its milestones, phases, pauses or lessons, so each
/// of these lists by kind.
#[test]
fn the_project_rooted_lines_list_by_kind() {
    has(
        "onboard",
        "for k in project milestone phase requirement; do specforge export --kinds \"$k\" --pick '.nodes[].id'",
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

/// REVIEW-r43 M3: five line shapes that the host refuses or stores wrongly (a block as a bare word, `null` for an
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

/// REVIEW-r43 M9 and M10: no prompt writes upstream GSD's `~/.gsd/defaults.json`, and none takes an API key in the
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

/// REVIEW-r43 M11: a "Parse JSON for:" line that follows `specforge gsd progress-report --format json` may name only
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

/// REVIEW-r43 post-fix N5: the leftovers of the Minor 22 and M10 text mismatches.
#[test]
fn the_post_fix_text_leftovers_stay_fixed() {
    // progress asks for the next phase of the milestone, padded as phase ids are
    assert!(
        workflow("progress.md").contains("NEXT_PHASE=$(printf %02d $((Z+1))); NEXT_PHASE_SECTION=$(specforge query \"$MILESTONE/$NEXT_PHASE\"")
    );
    // the ingest staging directory holds the documents' text and is removed at the end
    assert!(workflow("ingest-docs.md").contains("rm -rf \"$INTEL_DIR\""));
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

/// G4 (plan 05): no rendered line runs jq, neither an asset's (`port_tools` scans those) nor one the Rust side writes
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
