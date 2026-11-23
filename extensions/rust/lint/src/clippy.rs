//! cargo's JSON messages (`--message-format=json`) as SARIF results (ADR 0389 D8): clippy writes no SARIF, so the pack
//! converts its diagnostics into [`SarifResult`] values for `specforge-code-facts`' `finding_facts`. Text in, values
//! out: nothing here opens a file.

use std::collections::BTreeSet;

use serde_json::Value;
use specforge_code_facts::{relative, shown, Level, SarifResult, UNKNOWN};

/// The tool every result names.
const TOOL: &str = "clippy";

/// The results of cargo's JSON lines `text`, their files relative to the project root `cwd`, and the paths outside the
/// project their primary spans named, each once and [`shown`].
///
/// A line whose `reason` is `compiler-message` and whose message has a code or a primary span is one result: the rule
/// is `message.code.code` (a rustc lint keeps its bare name, `unused_variables`), else `unknown`; the level is
/// `message.level` (`error`, `warning`, `note` and `help` as note, anything else `unknown`); the message is
/// `message.message`; the file and lines are the span marked `is_primary` (`file_name`, `line_start`, `line_end`),
/// only that one. Other reasons, summaries with neither code nor span ("2 warnings emitted") and blank lines are
/// ignored. Results equal on rule, file, lines and message appear once, in first-seen order: `--all-targets` checks a
/// library twice (lib and lib test) and repeats each of its diagnostics. A primary span outside the project (a
/// registry crate's macro) is counted, not kept.
///
/// A capture cut off inside a line or between two is an error, so it fails instead of dropping findings: a line that is
/// no JSON, named by its 1-based number, or no `build-finished` record, which cargo writes last on every run that ends,
/// failed builds included (a run killed by a signal stops before it).
pub(crate) fn results(text: &str, cwd: &str) -> Result<(Vec<SarifResult>, Vec<String>), String> {
    let mut results = Vec::new();
    let mut outside = Vec::new();
    let mut seen = BTreeSet::new();
    let mut seen_outside = BTreeSet::new();
    let mut finished = false;
    for (at, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value =
            serde_json::from_str(line).map_err(|e| format!("line {}: no JSON: {e}", at + 1))?;
        match value.get("reason").and_then(Value::as_str) {
            Some("compiler-message") => {}
            Some("build-finished") => {
                finished = true;
                continue;
            }
            _ => continue,
        }
        let Some(message) = value.get("message") else {
            continue;
        };
        let rule = message.pointer("/code/code").and_then(Value::as_str);
        let primary = message
            .get("spans")
            .and_then(Value::as_array)
            .and_then(|spans| {
                spans
                    .iter()
                    .find(|span| span.get("is_primary").and_then(Value::as_bool) == Some(true))
            });
        if rule.is_none() && primary.is_none() {
            continue;
        }
        let (file, line, end_line) = match primary {
            None => (None, None, None),
            Some(span) => {
                let file = match span.get("file_name").and_then(Value::as_str) {
                    None => None,
                    Some(name) => match relative(name, cwd) {
                        Some(file) => Some(file),
                        None => {
                            let path = shown(name);
                            if seen_outside.insert(path.clone()) {
                                outside.push(path);
                            }
                            continue;
                        }
                    },
                };
                (file, number(span, "line_start"), number(span, "line_end"))
            }
        };
        let result = SarifResult {
            tool: TOOL.to_string(),
            rule: rule.unwrap_or(UNKNOWN).to_string(),
            level: level(message.get("level").and_then(Value::as_str)),
            message: message
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            file,
            line,
            end_line,
        };
        let key = (
            result.rule.clone(),
            result.file.clone(),
            result.line,
            result.end_line,
            result.message.clone(),
        );
        if seen.insert(key) {
            results.push(result);
        }
    }
    if !finished {
        return Err(
            "no `build-finished` record: the capture stops before cargo's last line, so the run was cut off (killed \
             or crashed) and its findings may be partial"
                .to_string(),
        );
    }
    Ok((results, outside))
}

/// cargo's diagnostic level as a SARIF level.
fn level(level: Option<&str>) -> Level {
    match level {
        Some("error") => Level::Error,
        Some("warning") => Level::Warning,
        Some("note" | "help") => Level::Note,
        _ => Level::Unknown,
    }
}

/// The 1-based line number at `name` of `span`, if it is one.
fn number(span: &Value, name: &str) -> Option<usize> {
    span.get(name)
        .and_then(Value::as_u64)
        .and_then(|line| usize::try_from(line).ok())
        .filter(|&line| line >= 1)
}
