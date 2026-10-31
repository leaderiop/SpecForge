//! Ported from gsd-core v1.18.0 src/runtime-artifact-conversion.cts, MIT, Copyright (c) 2026 Open GSD; see gsd/core/LICENSE-GSD
//!
//! Upstream's runtime adapters (D-r42-12): the text rewrites `convertClaudeToCodexMarkdown` and
//! `convertClaudeToCursorMarkdown` apply to a Claude Code workflow so it reads right on Codex and Cursor, and the two
//! adapter headers. Claude Code gets what upstream's install does to a Claude file (`to_claude`): the runtime notes of
//! another runtime dropped and `/gsd:x` written `/gsd-x`. Left out, by name: the frontmatter and file
//! naming of a skill (the host's install writes those), `rewriteBareGsdToolsCommandsForCodex` (a rendered text names no
//! `gsd-tools` command, the port rule and `assert_clean` see to it) and the stamp of `_applyRuntimeRewrites` (the
//! workflow's `RUNTIME` line takes the host as its fallback instead, `setting::fill`). Cursor's path rewrite is done.

mod headers;
#[cfg(test)]
mod tests;

/// The adapter header a host's text starts with; none on Claude Code.
pub fn header(host: &str) -> &'static str {
    match host {
        "codex" => headers::CODEX,
        "cursor" => headers::CURSOR,
        _ => "",
    }
}

/// `text` as `host` reads it. The host is `claude-code`, `codex` or `cursor`; an unknown host gets Claude's text.
pub fn convert(host: &str, text: &str) -> String {
    match host {
        "codex" => to_codex(text),
        "cursor" => to_cursor(text),
        _ => to_claude(text),
    }
}

/// The names of upstream's `commands/gsd/*.md` at v1.18.0 (`readGsdCommandNames`): only these are rewritten from the
/// colon form, as `transformContentToHyphen` does; `/gsd:transition` and the like name no command and stay.
const COMMAND_NAMES: &[&str] = &[
    "add-tests",
    "ai-integration-phase",
    "audit-fix",
    "audit-milestone",
    "audit-uat",
    "autonomous",
    "capture",
    "cleanup",
    "code-review",
    "complete-milestone",
    "config",
    "debug",
    "discuss-phase",
    "docs-update",
    "eval-review",
    "execute-phase",
    "explore",
    "extract-learnings",
    "fast",
    "forensics",
    "graphify",
    "health",
    "help",
    "import",
    "inbox",
    "ingest-docs",
    "manager",
    "map-codebase",
    "mempalace-capture",
    "mempalace-recall",
    "milestone-summary",
    "mvp-phase",
    "new-milestone",
    "new-project",
    "next",
    "ns-context",
    "ns-ideate",
    "ns-manage",
    "ns-project",
    "ns-review",
    "ns-workflow",
    "onboard",
    "pause-work",
    "phase",
    "plan-phase",
    "plan-review-convergence",
    "pr-branch",
    "profile-user",
    "progress",
    "quick-batch",
    "quick",
    "resume-work",
    "review-backlog",
    "review",
    "secure-phase",
    "settings",
    "ship",
    "sketch",
    "spec-phase",
    "spike",
    "stats",
    "surface",
    "thread",
    "ui-phase",
    "ui-review",
    "ultraplan-phase",
    "undo",
    "update",
    "validate-phase",
    "verify-work",
    "workspace",
    "workstreams",
];

/// `gsd:<command>` -> `gsd-<command>` for a known command at a token boundary (`transformContentToHyphen`).
pub fn hyphenate_commands(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    let mut before: Option<char> = None;
    while let Some(at) = rest.find("gsd:") {
        let prev = rest[..at].chars().next_back().or(before);
        let tail = &rest[at + 4..];
        let name = COMMAND_NAMES
            .iter()
            .filter(|n| tail.starts_with(**n))
            .filter(|n| {
                !tail[n.len()..]
                    .starts_with(|c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
            })
            .max_by_key(|n| n.len());
        let left = !prev.is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'));
        out.push_str(&rest[..at]);
        out.push_str(if name.is_some() && left {
            "gsd-"
        } else {
            "gsd:"
        });
        before = Some(':');
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// What upstream's install does to a Claude file: `filterRuntimeNotesForTarget(claude)` and, for a runtime whose
/// descriptor says `hyphenNameAgentBody` (Claude does, #3683, #3583), `transformContentToHyphen`.
pub fn to_claude(content: &str) -> String {
    hyphenate_commands(&filter_runtime_notes(content, "claude"))
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `from` -> `to`, where `left` / `right` ask for a word boundary at that end (JavaScript's `\b`).
fn replace_bounded(text: &str, from: &str, to: &str, left: bool, right: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut before: Option<char> = None;
    while let Some(at) = rest.find(from) {
        let prev = rest[..at].chars().next_back().or(before);
        let next = rest[at + from.len()..].chars().next();
        let ok = (!left || !prev.is_some_and(is_word)) && (!right || !next.is_some_and(is_word));
        out.push_str(&rest[..at]);
        out.push_str(if ok { to } else { from });
        before = from.chars().next_back();
        rest = &rest[at + from.len()..];
    }
    out.push_str(rest);
    out
}

/// `<runtime_note>` blocks for another runtime are dropped, and so are the sections of a note aimed at another runtime
/// (`filterRuntimeNotesForTarget`).
pub fn filter_runtime_notes(content: &str, target: &str) -> String {
    const OPEN: &str = "<runtime_note";
    const CLOSE: &str = "</runtime_note>";
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(at) = rest.find(OPEN) {
        out.push_str(&rest[..at]);
        let after = &rest[at + OPEN.len()..];
        let Some((audience, inner_start)) = note_head(after) else {
            out.push_str(OPEN);
            rest = after;
            continue;
        };
        let Some(end) = after[inner_start..].find(CLOSE) else {
            out.push_str(OPEN);
            rest = after;
            continue;
        };
        let inner = &after[inner_start..inner_start + end];
        rest = &after[inner_start + end + CLOSE.len()..];
        if audience.is_some_and(|a| !a.eq_ignore_ascii_case(target)) {
            continue;
        }
        let kept = drop_other_sections(inner, target);
        let body = kept.trim();
        if !body.is_empty() {
            out.push_str(&format!("<runtime_note>\n{body}\n</runtime_note>"));
        }
    }
    out.push_str(rest);
    out
}

/// After `<runtime_note`: the declared `runtime="x"` and where the note's text starts, when the tag is one.
fn note_head(after: &str) -> Option<(Option<&str>, usize)> {
    if after.starts_with('>') {
        return Some((None, 1));
    }
    let trimmed = after.trim_start();
    if trimmed.len() == after.len() {
        return None;
    }
    let attr = trimmed.strip_prefix("runtime=")?;
    let quote = attr.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    let value_end = attr[1..].find(quote)?;
    let value = &attr[1..1 + value_end];
    let tail = &attr[value_end + 2..];
    tail.starts_with('>').then(|| {
        let consumed = after.len() - tail.len() + 1;
        (Some(value), consumed)
    })
}

/// The note's sections (`**Heading:** …` and the lines that follow up to a blank one) that name another audience.
fn drop_other_sections(inner: &str, target: &str) -> String {
    let lines: Vec<&str> = inner.split('\n').collect();
    let mut kept: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let heading = section_heading(lines[i]);
        let mut j = i + 1;
        if heading.is_some() {
            while j < lines.len() && !(lines[j].trim().is_empty() && j + 1 < lines.len()) {
                j += 1;
            }
        }
        let audience = heading
            .and_then(|h| (h.trim().to_lowercase() == "copilot (vs code)").then_some("copilot"));
        if audience.is_none_or(|a| a == target) {
            kept.extend(&lines[i..j]);
        }
        i = j;
    }
    kept.join("\n")
}

/// The heading of a line that starts `**Heading:**`.
fn section_heading(line: &str) -> Option<&str> {
    let rest = line.trim_start_matches([' ', '\t']).strip_prefix("**")?;
    let end = rest.find(":**")?;
    let heading = &rest[..end];
    (!heading.is_empty() && !heading.contains('*')).then_some(heading)
}

// ---- Codex ---------------------------------------------------------------------------------------------------------

/// `/gsd:name` and the `/gsd-name` mentions of workflow output become `$gsd-name`; a path (`/gsd-core/bin`) stays.
pub fn codex_skill_mentions(content: &str) -> String {
    let bytes = content.as_bytes();
    let mut out = String::with_capacity(content.len());
    let mut i = 0;
    while i < content.len() {
        if let Some(name_start) = mention_at(content, i) {
            let mut end = name_start;
            while end < content.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'-')
            {
                end += 1;
            }
            if end > name_start {
                let colon = bytes[i + 4] == b':';
                let boundary_ok = colon
                    || (i == 0 || is_left_delimiter(content[..i].chars().next_back()))
                        && bytes.get(end) != Some(&b'/');
                if boundary_ok {
                    out.push_str("$gsd-");
                    out.push_str(&content[name_start..end].to_ascii_lowercase());
                    i = end;
                    continue;
                }
            }
        }
        let ch = content[i..].chars().next().unwrap_or(' ');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Where the name starts when `/gsd:` or `/gsd-` (any case) opens at `i`.
fn mention_at(content: &str, i: usize) -> Option<usize> {
    let head = content.as_bytes().get(i..i + 5)?;
    (head[0] == b'/' && head[1..4].eq_ignore_ascii_case(b"gsd") && matches!(head[4], b':' | b'-'))
        .then_some(i + 5)
}

fn is_left_delimiter(c: Option<char>) -> bool {
    c.is_none_or(|c| c.is_whitespace() || matches!(c, '`' | '"' | '\'' | '(' | '['))
}

/// Drops `/clear` instructions, which Codex has no command for.
fn drop_clear(content: &str) -> String {
    let lower = content.to_ascii_lowercase();
    let mut out = String::with_capacity(content.len());
    let mut at = 0;
    while let Some(found) = lower[at..].find("/clear") {
        let found = at + found;
        let ticked = content[..found].ends_with('`');
        let head_end = if ticked { found - 1 } else { found };
        let mut tail = &content[found + "/clear".len()..];
        if ticked && tail.starts_with('`') {
            tail = &tail[1..];
        }
        let after = tail.trim_start();
        let after = after.strip_prefix(',').unwrap_or(after).trim_start();
        if after.len() >= 4 && after.as_bytes()[..4].eq_ignore_ascii_case(b"then") {
            let cut = after[4..]
                .strip_prefix(':')
                .unwrap_or(&after[4..])
                .trim_start();
            out.push_str(&content[at..head_end]);
            at = content.len() - cut.len();
        } else if content[at..head_end]
            .rsplit('\n')
            .next()
            .is_some_and(|l| l.trim().is_empty())
            && tail.split('\n').next().is_some_and(|l| l.trim().is_empty())
        {
            // `/clear` alone on its line: the line becomes empty.
            let line_start = content[..head_end].rfind('\n').map_or(0, |n| n + 1).max(at);
            out.push_str(&content[at..line_start]);
            at = content.len() - tail.len() + tail.find('\n').unwrap_or(tail.len());
        } else {
            out.push_str(&content[at..found + "/clear".len()]);
            at = found + "/clear".len();
        }
    }
    out.push_str(&content[at..]);
    out
}

/// Claude's paths and file names, as Codex has them.
fn codex_paths(content: &str) -> String {
    let mut c = content.to_string();
    for (from, to) in [
        ("$HOME/.claude/", "$HOME/.codex/"),
        ("~/.claude/", "~/.codex/"),
        ("./.claude/", "./.codex/"),
    ] {
        c = c.replace(from, to);
    }
    c = replace_bounded(&c, "$HOME/.claude", "$HOME/.codex", false, true);
    c = replace_bounded(&c, "~/.claude", "~/.codex", false, true);
    c = bare_dot_claude(&c);
    replace_bounded(&c, ".claudeignore", ".codexignore", false, true)
}

/// `.claude/` that no path, `~` or `$` leads to.
fn bare_dot_claude(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    let mut prev: Option<char> = None;
    while let Some(at) = rest.find(".claude/") {
        let before = rest[..at].chars().next_back().or(prev);
        let anchored =
            before.is_some_and(|c| is_word(c) || matches!(c, '-' | '.' | '/' | '~' | '$'));
        out.push_str(&rest[..at]);
        out.push_str(if anchored { ".claude/" } else { ".codex/" });
        prev = Some('/');
        rest = &rest[at + ".claude/".len()..];
    }
    out.push_str(rest);
    out
}

/// `Claude` (the agent) becomes "the agent"; the product and model names stay (`neutralizeAgentReferences`).
pub fn neutralize_agent_references(content: &str, instruction_file: &str) -> String {
    const KEEP: [&str; 7] = [
        " Code", " Opus", " Sonnet", " Haiku", " native", " based", "-",
    ];
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    let mut prev: Option<char> = None;
    while let Some(at) = rest.find("Claude") {
        let before = rest[..at].chars().next_back().or(prev);
        let tail = &rest[at + "Claude".len()..];
        let replace = !before.is_some_and(is_word)
            && !KEEP.iter().any(|k| tail.starts_with(k))
            && !tail.chars().next().is_some_and(is_word)
            && !tail.starts_with(".md");
        out.push_str(&rest[..at]);
        out.push_str(if replace { "the agent" } else { "Claude" });
        prev = Some('e');
        rest = tail;
    }
    out.push_str(rest);
    let renamed = out.replace("CLAUDE.md", instruction_file);
    drop_agents_md_instruction(&renamed)
}

/// Removes the rest of each line that begins "Do NOT load full `AGENTS.md` files".
fn drop_agents_md_instruction(content: &str) -> String {
    const START: &str = "Do NOT load full `AGENTS.md` files";
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(at) = rest.find(START) {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        rest = &rest[rest.find('\n').unwrap_or(rest.len())..];
    }
    out.push_str(rest);
    out
}

/// `convertClaudeToCodexMarkdown`, but for `$ARGUMENTS` (literal here, substituted after, #5637).
pub fn to_codex(content: &str) -> String {
    let mut c = codex_skill_mentions(&filter_runtime_notes(content, "codex"));
    c = drop_clear(&c);
    c = codex_paths(&c);
    neutralize_agent_references(&c, "AGENTS.md")
}

// ---- Cursor --------------------------------------------------------------------------------------------------------

/// "Claude Code" becomes the runtime's name, except inside a `<runtime_compatibility>` table.
fn brand_swap(content: &str, brand: &str) -> String {
    const OPEN: &str = "<runtime_compatibility>";
    const CLOSE: &str = "</runtime_compatibility>";
    let swap = |gap: &str| replace_bounded(gap, "Claude Code", brand, true, true);
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(at) = rest.find(OPEN) {
        let Some(len) = rest[at..].find(CLOSE) else {
            break;
        };
        out.push_str(&swap(&rest[..at]));
        out.push_str(&rest[at..at + len + CLOSE.len()]);
        rest = &rest[at + len + CLOSE.len()..];
    }
    out.push_str(&swap(rest));
    out
}

/// Removes every line that starts with `marker`, with its line break.
fn drop_lines_starting(content: &str, marker: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(at) = rest.find(marker) {
        let Some(end) = rest[at..].find('\n') else {
            break;
        };
        out.push_str(&rest[..at]);
        rest = &rest[at + end + 1..];
    }
    out.push_str(rest);
    out
}

/// `convertClaudeToCursorMarkdown`, but for `$ARGUMENTS` (literal here, substituted after, #5637).
pub fn to_cursor(content: &str) -> String {
    let filtered = filter_runtime_notes(content, "cursor");
    // `gsd:` -> `gsd-` in any case, keeping a leading slash.
    let lower = filtered.to_ascii_lowercase();
    let mut c = String::with_capacity(filtered.len());
    let mut at = 0;
    while let Some(found) = lower[at..].find("gsd:") {
        c.push_str(&filtered[at..at + found]);
        c.push_str("gsd-");
        at += found + 4;
    }
    c.push_str(&filtered[at..]);
    c = replace_bounded(&c, "Bash(", "Shell(", true, false);
    c = replace_bounded(&c, "Edit(", "StrReplace(", true, false);
    c = replace_bounded(
        &c,
        "AskUserQuestion",
        "conversational prompting",
        true,
        true,
    );
    c = c.replace(
        "subagent_type=\"general-purpose\"",
        "subagent_type=\"generalPurpose\"",
    );
    c = c.replace("./CLAUDE.md", ".cursor/rules/");
    c = replace_bounded(&c, "CLAUDE.md", ".cursor/rules/", true, true);
    c = c.replace(".claude/skills/", ".cursor/skills/");
    c = drop_lines_starting(&c, "**Known Claude Code bug (classifyHandoffIfNeeded):**");
    c = drop_lines_starting(&c, "- **classifyHandoffIfNeeded false failure:**");
    cursor_paths(&brand_swap(&c, "Cursor"))
}

/// `from` -> `to` where no word character or `-` follows (JavaScript's `(?![\w-])`).
fn replace_not_followed(text: &str, from: &str, to: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(from) {
        let next = rest[at + from.len()..].chars().next();
        let ok = !next.is_some_and(|c| is_word(c) || c == '-');
        out.push_str(&rest[..at]);
        out.push_str(if ok { to } else { from });
        rest = &rest[at + from.len()..];
    }
    out.push_str(rest);
    out
}

/// `_applyRuntimeRewrites(cursor)`, profile `slash-lookahead-own-tilde`: the install paths Claude's text names are
/// Cursor's (`~/.cursor/` for the home forms). References were re-pointed before this runs (D-r42-5).
fn cursor_paths(content: &str) -> String {
    let mut c = content.to_string();
    for (from, to) in [
        ("~/.claude/", "~/.cursor/"),
        ("$HOME/.claude/", "~/.cursor/"),
        ("./.claude/", "./.cursor/"),
    ] {
        c = c.replace(from, to);
    }
    for (from, to) in [
        ("~/.claude", "~/.cursor"),
        ("$HOME/.claude", "~/.cursor"),
        ("./.claude", "./.cursor"),
    ] {
        c = replace_not_followed(&c, from, to);
    }
    c
}
