//! Upstream's own vectors through the port. The Cursor cases are `tests/cursor-conversion.test.cjs`'; the filter and
//! neutralising cases are `tests/runtime-converters.test.cjs`' (`#4482`, `#766`). Both files are vendored beside the
//! source. A case the port does not do is named and skipped: the frontmatter and file name of a skill or agent
//! (`convertClaudeCommandToCursorSkill`'s `name:`, `convertClaudeAgentToCursorAgent`), and the install-side path rewrite
//! `_applyRuntimeRewrites(cursor)`, which runs after references are re-pointed (D-r42-5).

use super::*;

const SOURCE: &str =
    include_str!("../../../../parity/upstream/v1.18.0/src/runtime-artifact-conversion.cts");

/// Cursor's `preserves slash for slash commands in markdown body`.
#[test]
fn cursor_conversion_matches_upstreams_vectors() {
    let out = to_cursor("Next:\n/gsd:execute-phase 17\n/gsd-help\ngsd:progress\n");
    assert!(out.contains("/gsd-execute-phase 17"), "{out}");
    assert!(out.contains("/gsd-help"), "{out}");
    assert!(out.contains("gsd-progress"), "{out}");
    assert!(!out.contains("/gsd:execute-phase"), "{out}");
    // The rewrites of `convertClaudeToCursorMarkdown`, one line each.
    let converted = to_cursor(
        "Bash(ls) and Edit(f) and AskUserQuestion\nTask(subagent_type=\"general-purpose\")\nSee ./CLAUDE.md, `CLAUDE.md` and .claude/skills/x\nThis is Claude Code.\n",
    );
    assert_eq!(
        converted,
        "Shell(ls) and StrReplace(f) and conversational prompting\nTask(subagent_type=\"generalPurpose\")\nSee .cursor/rules/, `.cursor/rules/` and .cursor/skills/x\nThis is Cursor.\n"
    );
}

#[test]
fn cursor_keeps_claude_code_inside_a_runtime_compatibility_table() {
    let text = "Claude Code runs it.\n<runtime_compatibility>\n| Claude Code | yes |\n</runtime_compatibility>\nClaude Code again.";
    assert_eq!(
        to_cursor(text),
        "Cursor runs it.\n<runtime_compatibility>\n| Claude Code | yes |\n</runtime_compatibility>\nCursor again."
    );
}

#[test]
fn cursor_drops_the_claude_code_bug_workarounds() {
    let text = "keep\n**Known Claude Code bug (classifyHandoffIfNeeded):** text\n- **classifyHandoffIfNeeded false failure:** more\nkeep too\n";
    assert_eq!(to_cursor(text), "keep\nkeep too\n");
}

/// `neutralizeAgentReferences` (#766).
#[test]
fn codex_conversion_matches_upstreams_vectors() {
    let n = |text: &str| neutralize_agent_references(text, "AGENTS.md");
    assert_eq!(
        n("Claude handles these decisions. Claude should read the file."),
        "the agent handles these decisions. the agent should read the file."
    );
    let kept = "This is a Claude Code bug. Use Claude Code settings. Use Claude Opus for planning. Claude Sonnet for execution. Claude Haiku for research.";
    assert_eq!(n(kept), kept);
    let out = n("Read CLAUDE.md for project instructions. Check ./CLAUDE.md if exists.");
    assert!(
        out.contains("AGENTS.md") && !out.contains("CLAUDE.md"),
        "{out}"
    );
    assert!(neutralize_agent_references("Read CLAUDE.md.", "GEMINI.md").contains("GEMINI.md"));
    let out = n("Do NOT load full `AGENTS.md` files — they contain agent definitions.");
    assert!(!out.contains("Do NOT load full"), "{out}");
    let kept = "The claude-ctx session and claude-code package.";
    assert_eq!(n(kept), kept);
    assert_eq!(n("Claude.md stays"), "Claude.md stays");
}

/// The skill mentions of `convertSlashCommandsToCodexSkillMentions` (#712).
#[test]
fn codex_skill_mentions_leave_paths_alone() {
    assert_eq!(
        codex_skill_mentions("run /gsd:plan-phase 3"),
        "run $gsd-plan-phase 3"
    );
    assert_eq!(
        codex_skill_mentions("then `/gsd-execute-phase` ok"),
        "then `$gsd-execute-phase` ok"
    );
    assert_eq!(codex_skill_mentions("(/gsd-help)"), "($gsd-help)");
    let path = "see `/gsd-core/workflows/update.md` and /gsd-core/bin/x";
    assert_eq!(codex_skill_mentions(path), path);
    assert_eq!(codex_skill_mentions("a/gsd-help"), "a/gsd-help");
}

#[test]
fn codex_drops_clear_and_moves_paths() {
    assert_eq!(to_codex("run `/clear` then:\nnext"), "run next");
    assert_eq!(to_codex("a\n/clear\nb"), "a\n\nb");
    let out =
        to_codex("see ~/.claude/skills and $HOME/.claude and .claude/commands and x.claude/y");
    assert_eq!(
        out,
        "see ~/.codex/skills and $HOME/.codex and .codex/commands and x.claude/y"
    );
}

/// `#4482`: a Copilot-only note leaves nothing, and a mixed one keeps its neutral part.
#[test]
fn a_runtime_note_for_another_host_is_dropped() {
    let note = "<runtime_note runtime=\"opencode\">\nOnly OpenCode.\n</runtime_note>\n<objective>Keep me.</objective>";
    assert_eq!(
        filter_runtime_notes(note, "codex"),
        "\n<objective>Keep me.</objective>"
    );
    assert_eq!(
        filter_runtime_notes(
            "<runtime_note runtime=\"codex\">\nFor Codex.\n</runtime_note>",
            "codex"
        ),
        "<runtime_note>\nFor Codex.\n</runtime_note>"
    );
}

#[test]
fn copilot_paragraphs_are_dropped_off_copilot() {
    let note = "<runtime_note>\n**Copilot (VS Code):** Use `vscode_askquestions`.\n\n**TEXT_MODE fallback:** Keep me.\n</runtime_note>";
    for host in ["codex", "cursor", "opencode", "claude"] {
        let out = filter_runtime_notes(note, host);
        assert!(!out.contains("vscode_askquestions"), "{host}: {out}");
        assert!(out.contains("TEXT_MODE fallback"), "{host}: {out}");
    }
    assert_eq!(filter_runtime_notes(note, "copilot"), note);
    let only =
        "<runtime_note>\n**Copilot (VS Code):** Use `vscode_askquestions`.\n</runtime_note>\nafter";
    assert_eq!(filter_runtime_notes(only, "codex"), "\nafter");
    let other =
        "<runtime_note>\n**OpenCode:** Keep this runtime-specific guidance.\n</runtime_note>";
    assert_eq!(filter_runtime_notes(other, "codex"), other);
}

#[test]
fn claude_code_gets_the_text_raw() {
    let text = "Claude and ~/.claude/x /gsd:y $ARGUMENTS";
    assert_eq!(convert("claude-code", text), text);
    assert_eq!(header("claude-code"), "");
}

#[test]
fn the_codex_header_has_no_invocation_section_and_no_resolve_model() {
    let header = header("codex");
    assert!(header.starts_with("<codex_skill_adapter>"), "{header}");
    assert!(
        header.trim_end().ends_with("</codex_skill_adapter>"),
        "{header}"
    );
    assert!(!header.contains("## A."), "section A is the trampoline's");
    assert!(
        !header.contains("resolve-model") && !header.contains("gsd_run"),
        "{header}"
    );
    assert!(
        !header.contains("{{GSD_ARGS}}"),
        "the arguments are already in the text"
    );
    assert!(header.contains("## B. AskUserQuestion") && header.contains("## C. Task()"));
    let cursor = super::header("cursor");
    assert!(
        cursor.contains("## D. Subagent Spawning") && !cursor.contains("## A."),
        "{cursor}"
    );
}

/// The lines of the template literal that returns `adapter` in the vendored source, with `\`` unescaped.
fn upstream_lines(adapter: &str) -> Vec<String> {
    let start = SOURCE
        .find(&format!("return `<{adapter}>"))
        .expect("the adapter in the source");
    let end = SOURCE[start..]
        .find(&format!("</{adapter}>`"))
        .expect("its end")
        + start;
    SOURCE[start + "return `".len()..end + adapter.len() + 3]
        .replace("\\`", "`")
        .split('\n')
        .map(String::from)
        .collect()
}

/// The header lines this port rewrote (D-r42-7, D-r42-12): the arguments are text already, and the model resolution is
/// gone because the installed agent carries its model and effort.
const REWRITTEN: [&str; 4] = [
    "- When `request_user_input` is rejected or unavailable, activate TEXT_MODE: behave as if `--text` were among the arguments, so the workflow's built-in text-mode branching takes over. Present every `AskUserQuestion` call as a plain-text numbered list, then stop and wait for the user's reply. Do NOT pick a default and continue (#3018 / #3808).",
    "- The installed agent carries its own model and effort: do not resolve them. Pass `reasoning_effort` only when the",
    "  workflow names one and the visible `spawn_agent` schema advertises it; do not invent one-off effort literals in",
    "  workflow prose.",
];

#[test]
fn adapter_header_lines_are_upstreams() {
    for (adapter, header) in [
        ("codex_skill_adapter", headers::CODEX),
        ("cursor_skill_adapter", headers::CURSOR),
    ] {
        let theirs = upstream_lines(adapter);
        for line in header.split('\n') {
            assert!(
                theirs.iter().any(|t| t == line) || REWRITTEN.contains(&line),
                "{adapter}: a line that is not upstream's and not listed: {line}"
            );
        }
    }
    // Every rewritten line is used by the header it belongs to.
    for line in REWRITTEN {
        assert!(headers::CODEX.contains(line), "{line}");
    }
}

#[test]
fn codex_conversion_survives_multibyte_text_before_a_mention() {
    // `—ab` is five bytes, so a five-byte window at the dash is a valid slice that does not start with `/`.
    let text = "—ab then — run /gsd-help, “quoted” /gsd:next";
    assert_eq!(
        codex_skill_mentions(text),
        "—ab then — run $gsd-help, “quoted” $gsd-next"
    );
}
