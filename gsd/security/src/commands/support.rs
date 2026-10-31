//! What the commands share: one way to refuse (`invalid_input`, exit 1) and the answer in both forms. A trimmed copy
//! of `@gsd/core`'s `commands::support`, which `@gsd/security` cannot link (ADR gsd/0002).

use serde_json::Value;
use specforge_extension_sdk::prelude::*;

const INVALID: &str = "invalid_input";

/// Runs a command's body: its `Err` is the refusal, reported as `invalid_input`.
pub fn run(
    call: &CommandCall<'_>,
    body: impl FnOnce(&CommandCall<'_>) -> Result<CommandOutput, String>,
) -> CommandOutput {
    match body(call) {
        Ok(out) => out,
        Err(why) => match why.strip_prefix("human_only: ") {
            Some(step) => call.fail(&CommandError::new("human_only", step), 1),
            None => call.fail(&CommandError::new(INVALID, why), 1),
        },
    }
}

/// `payload` through `render`, with `human` as the text form.
pub fn answer(call: &CommandCall<'_>, payload: Value, human: String) -> CommandOutput {
    call.render(&payload, |text| text.push_str(&human))
}

/// The arg `arg`, which must be given and not blank.
pub fn required<'a>(call: &'a CommandCall<'_>, arg: &str) -> Result<&'a str, String> {
    match call.str(arg) {
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

/// The last `/`-segment of an id: its name.
pub fn name_of(id: &str) -> &str {
    id.rsplit('/').next().unwrap_or(id)
}
