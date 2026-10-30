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
        "specforge export --kinds project,milestone,phase,requirement",
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
