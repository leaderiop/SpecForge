//! The render pipeline (D-r42-6): one pure function from a prompt's command file, the host and the argument text to
//! the text a model reads. The export and the native tests call the same function.
//!
//! In order: the preamble (ours), the command file with its `<execution_context>` includes inlined, the settings its
//! lines read filled in (`setting`), the prompt's own tail, then the references re-pointed (D-r42-5) and the whole
//! converted for the host (D-r42-12). Re-pointing comes first: Codex's `.claude` rewrite would otherwise hide the paths.
//! Then the heredoc terminators are stamped and `$ARGUMENTS` replaced, last: the typed text is data no rewrite runs on
//! (#5637). The host's adapter header goes in front, unconverted.
//!
//! Also ports gsd-core bin/lib modules: args-terminator.cjs (the collision-free terminator, stamped per render).

use super::{compose, host, preamble, setting};

pub struct Input<'a> {
    /// The upstream command file's stem: `plan-phase` for `commands/gsd/plan-phase.md`.
    pub command: &'a str,
    /// `claude-code`, `codex` or `cursor`.
    pub host: &'a str,
    /// What `$ARGUMENTS` stands for (`args::text`).
    pub arguments: &'a str,
    /// The settings block of the preamble (`settings::block()`).
    pub settings: &'a str,
    /// The extension the prompt belongs to (`@gsd/core`): the preamble names it as the owner of the settings and of the
    /// harness fields' commands.
    pub extension: &'a str,
    /// The rows of the preamble's table of where GSD's files went (`preamble::FILES` for core).
    pub files: &'a [(&'a str, &'a str)],
    /// The prompt's own closing section, when it has one.
    pub tail: &'a str,
    /// The embedded text of an asset path (`gsd-core/…`, `commands/gsd/…`), attribution line included.
    pub asset: fn(&str) -> Option<&'static str>,
    /// Re-points the install paths of a text for a host (`text`, `host`).
    pub repoint: fn(&str, &str) -> String,
}

/// The attribution line of an asset is a notice about the file, not text a model reads.
pub fn without_notice(text: &str) -> &str {
    match text.strip_prefix("<!-- Ported from") {
        Some(rest) => rest.split_once('\n').map_or("", |(_, body)| body),
        None => text,
    }
}

/// The path of an `@~/.claude/gsd-core/<path>` include alone on a line.
fn include(line: &str) -> Option<String> {
    let at = line.trim().strip_prefix('@')?;
    let path = at
        .strip_prefix("~/.claude/")
        .or_else(|| at.strip_prefix("$HOME/.claude/"))?;
    path.starts_with("gsd-core/").then(|| path.to_string())
}

/// The command file with each include of its `<execution_context>` replaced by the file it names, once per render.
fn inline_includes(command: &str, asset: fn(&str) -> Option<&'static str>) -> String {
    let mut out = String::with_capacity(command.len());
    let mut seen: Vec<String> = Vec::new();
    let mut inside = false;
    for line in command.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed == "<execution_context>" {
            inside = true;
        } else if trimmed == "</execution_context>" {
            inside = false;
        }
        let file = include(line)
            .filter(|_| inside)
            .filter(|path| !seen.contains(path))
            .and_then(|path| asset(&path).map(|text| (path, text)));
        match file {
            Some((path, text)) => {
                out.push_str(&format!(
                    "<file path=\"{path}\">\n{}\n</file>\n",
                    compose::composed(&path, without_notice(text)).trim_end()
                ));
                seen.push(path);
            }
            None => out.push_str(line),
        }
    }
    out
}

/// `$ARGUMENTS` (not `$ARGUMENTS_X`) replaced by `text`.
fn substitute_arguments(text: &str, arguments: &str) -> String {
    let replacement = if arguments.is_empty() {
        "(no arguments)"
    } else {
        arguments
    };
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("$ARGUMENTS") {
        let tail = &rest[at + "$ARGUMENTS".len()..];
        out.push_str(&rest[..at]);
        let word = tail
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        out.push_str(if word { "$ARGUMENTS" } else { replacement });
        rest = tail;
    }
    out.push_str(rest);
    out
}

/// The terminators of the heredocs the assets capture the typed text with: opened `<<'T'`, closed by a line `T`.
pub(crate) const ARGS_TERMINATORS: [&str; 2] = ["GSD_ARGS_END", "GSD_QUICK_BATCH_ARGS_END"];

/// `token` when no line of `arguments` starts with it, else `token_<n>` for the smallest `n` no line starts with.
fn terminator_for(token: &str, arguments: &str) -> String {
    // a prefix test also sees through a line's trailing `\r`
    let taken = |t: &str| arguments.lines().any(|line| line.starts_with(t));
    if !taken(token) {
        return token.to_string();
    }
    (1..)
        .map(|n| format!("{token}_{n}"))
        .find(|t| !taken(t))
        .expect("finitely many lines block finitely many suffixes")
}

/// `command` with each terminator an argument line could equal renamed: its quoted opener and its closing lines.
fn stamp_terminators(command: &str, arguments: &str) -> String {
    let mut text = command.to_string();
    for token in ARGS_TERMINATORS {
        let stamp = terminator_for(token, arguments);
        if stamp == token {
            continue;
        }
        text = text
            .replace(&format!("<<'{token}'"), &format!("<<'{stamp}'"))
            .split_inclusive('\n')
            .map(|line| {
                let body = line.strip_suffix('\n').unwrap_or(line);
                if body.strip_suffix('\r').unwrap_or(body) == token {
                    format!("{stamp}{}", &line[token.len()..])
                } else {
                    line.to_string()
                }
            })
            .collect();
    }
    text
}

/// The text of the prompt, or why it cannot be built.
pub fn render(input: &Input<'_>) -> Result<String, String> {
    let path = format!("commands/gsd/{}.md", input.command);
    if let Some(command) = (input.asset)(&path) {
        return Ok(assemble(without_notice(command), input));
    }
    // A workflow upstream runs only as a sub-workflow (`execute-plan`) has no command file: the prompt wraps it.
    let workflow = format!("gsd-core/workflows/{}.md", input.command);
    (input.asset)(&workflow).ok_or_else(|| format!("no command file '{path}'"))?;
    Ok(assemble(&wrapper(&workflow), input))
}

/// The command body of a prompt that is only a workflow.
fn wrapper(workflow: &str) -> String {
    format!(
        "<objective>\nRun the workflow below end-to-end.\n</objective>\n\n<execution_context>\n@~/.claude/{workflow}\n</execution_context>\n\n<context>\nArguments: $ARGUMENTS\n</context>\n\n<process>\nExecute the workflow end-to-end. Preserve all its gates.\n</process>\n"
    )
}

/// [`render`] over the text of the command file.
pub fn assemble(command: &str, input: &Input<'_>) -> String {
    let mut text = preamble::text(input.host, input.settings, input.extension, input.files);
    text.push_str(&setting::fill(
        &inline_includes(command, input.asset),
        input.settings,
        runtime(input.host),
    ));
    if !input.tail.is_empty() {
        text.push_str("\n\n");
        text.push_str(input.tail);
    }
    let converted = host::convert(input.host, &(input.repoint)(&text, input.host));
    // the render knows the arguments, so the terminator is chosen to be one they cannot contain (#5637; upstream stamps
    // a per-install nonce because its installed files hold the heredoc); spliced after every rewrite, so the text
    // the terminator was chosen against is the text the heredoc holds
    let body = substitute_arguments(
        &stamp_terminators(&converted, input.arguments),
        input.arguments,
    );
    format!("{}{body}", header(input.host))
}

/// The runtime a host is called in the settings: `claude-code` is `claude`.
pub fn runtime(host: &str) -> &str {
    host.strip_suffix("-code").unwrap_or(host)
}

fn header(host: &str) -> String {
    match host::header(host) {
        "" => String::new(),
        header => format!("{header}\n\n"),
    }
}

#[cfg(test)]
mod tests;
