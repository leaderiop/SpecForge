//! What the writing commands share (D-r41-2): one way to refuse, the clock, a person's text and the ids a command
//! creates. Every refusal is `invalid_input` (exit 1) unless a gate or the host says otherwise.

use std::collections::BTreeMap;

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

pub const INVALID: &str = "invalid_input";

/// Runs a command's body: its `Err` is the refusal, reported as `invalid_input`.
pub fn run(
    call: &CommandCall<'_>,
    body: impl FnOnce(&CommandCall<'_>) -> Result<CommandOutput, String>,
) -> CommandOutput {
    match body(call) {
        Ok(out) => out,
        // A refusal that starts `human_only: ` is the host's own code for what a person wrote (ADR 0298).
        Err(why) => match why.strip_prefix("human_only: ") {
            Some(step) => call.fail(&CommandError::new("human_only", step), 1),
            None => call.fail(&CommandError::new(INVALID, why), 1),
        },
    }
}

/// `{"wrote": [..], ..}` through `render`, with `human` as the text form, and `plan` as the edit plan.
pub fn answer(
    call: &CommandCall<'_>,
    payload: Value,
    human: String,
    plan: Option<&PlanBuilder>,
) -> CommandOutput {
    let out = call.render(&payload, |text| text.push_str(&human));
    match plan {
        Some(plan) => out.with_plan(plan.finish()),
        None => out,
    }
}

/// The optional JSON arg `name` as `T`, `default` when it is not given.
pub fn json<T>(
    call: &CommandCall<'_>,
    name: &str,
    default: T,
    parse: impl FnOnce(&str) -> serde_json::Result<T>,
) -> Result<T, String> {
    match call.str(name) {
        None => Ok(default),
        Some(text) => parse(text).map_err(|e| format!("{name}: not the JSON it takes: {e}")),
    }
}

/// The host's clock; a call the host gave none is refused, never written as `""`.
pub fn now<'a>(call: &'a CommandCall<'_>) -> Result<&'a str, String> {
    match call.now() {
        "" => Err("the host sent no clock; run the command through `specforge`".to_string()),
        now => Ok(now),
    }
}

/// A control character of `text` (C0 but tab and line feed, DEL, C1, the line and paragraph separators and the bidi
/// overrides, which reorder what a reader sees) as `U+00XX`.
fn control(text: &str) -> Option<String> {
    text.chars()
        .find(|c| {
            (c.is_control() && *c != '\t' && *c != '\n')
                || matches!(c, '\u{2028}' | '\u{2029}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
        })
        .map(|c| format!("U+{:04X}", c as u32))
}

/// An id ordered as a person reads it: `A9` before `A10`.
pub fn natural(id: &str) -> (String, u64) {
    let digits = id.chars().rev().take_while(char::is_ascii_digit).count();
    let (head, number) = id.split_at(id.len() - digits);
    (head.to_string(), number.parse().unwrap_or(0))
}

/// The arg `arg` as a person wrote it (D-r41-4): refused when it holds a control character, never stripped.
pub fn text<'a>(call: &'a CommandCall<'_>, arg: &str) -> Result<Option<&'a str>, String> {
    let value = call.str(arg);
    if let Some(code) = value.and_then(control) {
        return Err(format!("'{arg}' holds a control character ({code})"));
    }
    Ok(value)
}

/// [`text`] for an arg that, when given, is not blank (`--name ""` sets nothing).
pub fn given<'a>(call: &'a CommandCall<'_>, arg: &str) -> Result<Option<&'a str>, String> {
    match text(call, arg)? {
        Some(value) if value.trim().is_empty() => Err(format!("'{arg}' is empty")),
        other => Ok(other),
    }
}

/// The optional JSON list of strings `name`, each item as a person wrote it ([`text`]'s rule).
pub fn texts(call: &CommandCall<'_>, name: &str) -> Result<Option<Vec<String>>, String> {
    let Some(raw) = call.str(name) else {
        return Ok(None);
    };
    let items: Vec<String> =
        serde_json::from_str(raw).map_err(|e| format!("{name}: not the JSON it takes: {e}"))?;
    for item in &items {
        if let Some(code) = control(item) {
            return Err(format!("'{name}' holds a control character ({code})"));
        }
    }
    Ok(Some(items))
}

/// [`text`] for an arg that must be given and not empty.
pub fn required<'a>(call: &'a CommandCall<'_>, arg: &str) -> Result<&'a str, String> {
    match text(call, arg)? {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(format!("'{arg}' is required")),
    }
}

/// The entity of `kind` that `id` names: its full path, or the end of one that no other entity of the kind shares.
pub fn node<'a>(call: &CommandCall<'a>, kind: &str, id: &str) -> Result<&'a GraphNode, String> {
    let graph = call.graph();
    if let Some(found) = graph.node(id).filter(|n| n.kind == kind) {
        return Ok(found);
    }
    let tail = format!("/{id}");
    let found: Vec<&GraphNode> = graph
        .nodes()
        .iter()
        .filter(|n| n.kind == kind && n.id.ends_with(&tail))
        .collect();
    match found.as_slice() {
        [one] => Ok(one),
        [] => Err(format!("no {kind} '{id}'")),
        many => Err(format!(
            "{kind} '{id}' is ambiguous: {}",
            many.iter()
                .map(|n| n.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// The next free number of `prefix` among the entities of `kind` under `scope` (a path prefix, `""` for none):
/// `PS` with `PS1` and `PS3` there gives `PS4`.
pub fn next_id(graph: &CommandGraph, kind: &str, scope: &str, prefix: &str) -> String {
    let highest = graph
        .nodes()
        .iter()
        .filter(|n| n.kind == kind)
        .filter_map(|n| n.id.strip_prefix(scope))
        .filter_map(|rest| rest.strip_prefix(prefix))
        .filter_map(|digits| digits.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    format!("{prefix}{}", highest + 1)
}

/// The file a new ops entity (a pause, a blocker, a window) goes to: the `ops.spec` beside the file that holds
/// `anchor` when the project has one, else that file itself. A new file would not round-trip: the host leaves an
/// emptied file behind when its last entity is deleted.
pub fn ops_file(graph: &CommandGraph, anchor: Option<&str>) -> String {
    let file = anchor
        .and_then(|id| graph.node(id))
        .and_then(|n| n.file.as_deref())
        .or_else(|| graph.nodes().iter().find_map(|n| n.file.as_deref()))
        .unwrap_or("spec/ops.spec");
    let ops = match file.rsplit_once('/') {
        Some((dir, _)) => format!("{dir}/ops.spec"),
        None => "ops.spec".to_string(),
    };
    if graph
        .nodes()
        .iter()
        .any(|n| n.file.as_deref() == Some(&ops))
    {
        ops
    } else {
        file.to_string()
    }
}

/// A one-line title for an entity made from a person's text: its first line, cut at 72 characters.
pub fn title(text: &str) -> String {
    let line = text.lines().next().unwrap_or_default().trim();
    match line.char_indices().nth(72) {
        Some((cut, _)) => format!("{}...", line[..cut].trim_end()),
        None => line.to_string(),
    }
}

/// The fields of a `create`: the pairs whose value is not null.
pub fn fields<const N: usize>(pairs: [(&str, Value); N]) -> BTreeMap<String, Value> {
    pairs
        .into_iter()
        .filter(|(_, value)| !value.is_null())
        .map(|(name, value)| (name.to_string(), value))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::control;

    #[test]
    fn a_control_character_is_found_but_a_line_feed_and_a_tab_are_not() {
        assert_eq!(control("a\u{1b}[31mb").as_deref(), Some("U+001B"));
        assert_eq!(control("a\u{85}").as_deref(), Some("U+0085"));
        assert_eq!(control("a\u{7f}").as_deref(), Some("U+007F"));
        assert_eq!(control("two\nlines\tand tab"), None);
        assert_eq!(control("a\u{202e}b").as_deref(), Some("U+202E"));
        assert_eq!(control("a\u{2028}b").as_deref(), Some("U+2028"));
    }
}
