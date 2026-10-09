use serde_json::{Value, json};
use specforge_ops::navigate::{Fix, FixQuery};

use crate::args::Arguments;
use crate::tool::{Handled, ToolOutcome};
use specforge_ops::view::ProjectView;

/// `specforge.suggest_fixes`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Entity ID (optional, all if omitted)
    entity_id: Option<String>,
    /// Only diagnostics in this spec file
    file_path: Option<String>,
    /// Only diagnostics with this code, e.g. W001
    diagnostic_code: Option<String>,
}

/// `specforge.suggest_fixes`: the fixes the LSP offers as code actions for
/// the same diagnostics and entities, each with its edits (ADR 0016). A
/// diagnostic whose data names no fix contributes none: its suggestion
/// text stays on the diagnostic (validate, inspect).
pub fn call(view: ProjectView<'_>, args: Args) -> Handled {
    if let Some(entity_id) = args.entity_id.as_deref()
        && view.graph().node(entity_id).is_none()
    {
        return Err(crate::tool::entity_not_found(view.graph(), entity_id).into());
    }
    let diagnostics = view.reported();
    let query = FixQuery {
        entity: args.entity_id.as_deref(),
        file: args.file_path.as_deref(),
        code: args.diagnostic_code.as_deref(),
        within: None,
    };
    let fixes = super::navigator(view).fixes(&diagnostics, &query);
    Ok(ToolOutcome::ok(Value::Array(
        fixes.iter().map(suggestion).collect(),
    )))
}

/// A fix as an `McpFixSuggestion`: its edits are the spec's `TextEdit`
/// (`file_path`, `range` a `SourceSpan`, `new_text`).
fn suggestion(fix: &Fix) -> Value {
    json!({
        "title": fix.title,
        "kind": fix.kind,
        "diagnostic_code": fix.diagnostic_code,
        "entity_id": fix.subject,
        "edits": fix.edits.iter().map(|edit| json!({
            "file_path": edit.span.file,
            "range": super::span_json(&edit.span),
            "new_text": edit.new_text,
        })).collect::<Vec<Value>>(),
    })
}
