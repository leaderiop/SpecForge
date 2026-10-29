//! What the writing commands share (D-r41-2): one way to refuse, the clock, a person's text and the ids a command
//! creates. Every refusal is `invalid_input` (exit 1) unless a gate or the host says otherwise.

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
        Err(why) => call.fail(&CommandError::new(INVALID, why), 1),
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

/// A control character of `text` (C0 but tab and line feed, DEL, C1) as `U+00XX`.
fn control(text: &str) -> Option<String> {
    text.chars()
        .find(|c| c.is_control() && *c != '\t' && *c != '\n')
        .map(|c| format!("U+{:04X}", c as u32))
}

/// The arg `arg` as a person wrote it (D-r41-4): refused when it holds a control character, never stripped.
pub fn text<'a>(call: &'a CommandCall<'_>, arg: &str) -> Result<Option<&'a str>, String> {
    let value = call.str(arg);
    if let Some(code) = value.and_then(control) {
        return Err(format!("'{arg}' holds a control character ({code})"));
    }
    Ok(value)
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

#[cfg(test)]
mod tests {
    use super::control;

    #[test]
    fn a_control_character_is_found_but_a_line_feed_and_a_tab_are_not() {
        assert_eq!(control("a\u{1b}[31mb").as_deref(), Some("U+001B"));
        assert_eq!(control("a\u{85}").as_deref(), Some("U+0085"));
        assert_eq!(control("a\u{7f}").as_deref(), Some("U+007F"));
        assert_eq!(control("two\nlines\tand tab"), None);
    }
}
