//! What the tests of the lifecycle prompts share (D-r42): rendering a prompt through the real `specforge` binary for
//! each host, and the checks every rendered text must pass.
#![allow(dead_code, reason = "each prompt test file uses some of these helpers")]

use serde_json::Value;

use super::harness::{Project, Run};

pub const HOSTS: [&str; 3] = ["claude-code", "codex", "cursor"];

/// What no rendered text may name: GSD's planning folder, its tools script, the paths of its install and its
/// generated state files (`spec/gsd/…​.md`).
const FORBIDDEN: [&str; 7] = [
    ".planning",
    "gsd-tools",
    "gsd_run",
    "gsd-sdk",
    "gsd.cjs",
    "~/.claude/gsd-core",
    "$HOME/.claude/gsd-core",
];

/// `specforge prompt gsd.<id> <args> --host <host> --format json`.
pub fn prompt(p: &Project, id: &str, args: &[&str], host: &str) -> Run {
    let name = format!("gsd.{id}");
    let mut all = vec!["prompt", name.as_str()];
    all.extend_from_slice(args);
    all.extend_from_slice(&["--host", host, "--format", "json"]);
    p.run(&all)
}

/// The JSON a successful render printed.
pub fn payload(run: &Run) -> Value {
    assert_eq!(run.code, 0, "{}", run.all());
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("not JSON ({e}): {}", run.stdout))
}

/// The text of the first message.
pub fn text_of(rendered: &Value) -> String {
    rendered["messages"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// A `/gsd-<command>` mention (not a path) that Codex's conversion should have turned into `$gsd-<command>`.
fn codex_mention(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find("/gsd-") {
        let at = from + at;
        let left = at == 0
            || matches!(
                bytes[at - 1],
                b' ' | b'\t' | b'\n' | b'`' | b'"' | b'\'' | b'(' | b'['
            );
        let name_end = text[at + 5..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .map_or(text.len(), |n| at + 5 + n);
        if left && name_end > at + 5 && bytes.get(name_end) != Some(&b'/') {
            return Some(&text[at..name_end]);
        }
        from = at + 5;
    }
    None
}

/// No upstream path or tool name survives in `text` (D-r42 black-box invariants).
pub fn assert_clean(text: &str) {
    for forbidden in FORBIDDEN {
        assert!(!text.contains(forbidden), "the text names {forbidden}");
    }
    let mut rest = text;
    while let Some(at) = rest.find("spec/gsd/") {
        let token: String = rest[at..]
            .chars()
            .take_while(|c| !c.is_whitespace())
            .collect();
        assert!(!token.contains(".md"), "the text names {token}");
        rest = &rest[at + 9..];
    }
}

/// Renders `id` for each host and checks what every ported prompt owes: exit 0, the qualified id, the preamble, the
/// argument text, the workflow's heading, a clean text, and the host's own conversion. Returns `(host, payload)`.
pub fn render_three_hosts(
    p: &Project,
    id: &str,
    args: &[&str],
    argument_text: &str,
    heading: &str,
) -> Vec<(&'static str, Value)> {
    let mut out = Vec::new();
    for host in HOSTS {
        let rendered = payload(&prompt(p, id, args, host));
        let text = text_of(&rendered);
        assert_eq!(
            rendered["payload"]["prompt"],
            format!("@gsd/core@0.1.0:{id}"),
            "{host}"
        );
        assert!(
            text.contains("<specforge_gsd_preamble>"),
            "{host}: no preamble"
        );
        assert!(text.contains(argument_text), "{host}: no '{argument_text}'");
        assert!(text.contains(heading), "{host}: no '{heading}'");
        assert_clean(&text);
        match host {
            "codex" => {
                assert!(
                    text.starts_with("<codex_skill_adapter>"),
                    "codex: no header"
                );
                assert_eq!(codex_mention(&text), None, "codex: a /gsd- mention is left");
            }
            "cursor" => {
                assert!(
                    text.starts_with("<cursor_skill_adapter>"),
                    "cursor: no header"
                );
                assert!(!text.contains("Bash("), "cursor: Bash( is left");
            }
            _ => assert!(
                !text.contains("_skill_adapter>"),
                "claude-code has no header"
            ),
        }
        out.push((host, rendered));
    }
    out
}

#[test]
fn the_existing_judge_prompt_renders_through_the_helpers() {
    let p = super::harness::project("discuss");
    let rendered = payload(&prompt(&p, "spec_phase", &["v1.0/02/02-SPEC"], "codex"));
    let text = text_of(&rendered);
    assert!(text.starts_with("Judge the phase spec"), "{text}");
    assert_clean(&text);
    assert_eq!(rendered["payload"]["subject"], "v1.0/02/02-SPEC");
}

#[test]
fn a_codex_mention_is_found_and_a_path_is_not() {
    assert_eq!(
        codex_mention("run /gsd-plan-phase 3"),
        Some("/gsd-plan-phase")
    );
    assert_eq!(codex_mention("see /gsd-core/bin/x and a/gsd-help"), None);
    assert_eq!(codex_mention("run `$gsd-help`"), None);
}
