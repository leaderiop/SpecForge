use serde_json::{Value, json};
use specforge_ops::navigate::{Fix, FixQuery};

use crate::target::Call;
use crate::tool::{Handled, ToolOutcome};

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    #[serde(default, deserialize_with = "crate::args::lenient")]
    entity_id: Option<String>,
    #[serde(default, deserialize_with = "crate::args::lenient")]
    file_path: Option<String>,
    #[serde(default, deserialize_with = "crate::args::lenient")]
    diagnostic_code: Option<String>,
}

/// `specforge.suggest_fixes`: the fixes the LSP offers as code actions for
/// the same diagnostics and entities, each with its edits (ADR 0016). A
/// diagnostic whose data names no fix contributes none: its suggestion
/// text stays on the diagnostic (validate, inspect).
pub fn call(call: &mut Call<'_>, args: Args) -> Handled {
    if let Some(entity_id) = args.entity_id.as_deref()
        && call.view().graph().node(entity_id).is_none()
    {
        return Err(crate::tool::entity_not_found(entity_id).into());
    }
    let diagnostics = call.view().reported();
    let query = FixQuery {
        entity: args.entity_id.as_deref(),
        file: args.file_path.as_deref(),
        code: args.diagnostic_code.as_deref(),
        within: None,
    };
    let fixes = super::navigator(call).fixes(&diagnostics, &query);
    Ok(ToolOutcome::ok(Value::Array(
        fixes.iter().map(suggestion).collect(),
    )))
}

/// A fix as an `McpFixSuggestion`: its edits are the spec's `TextEdit`
/// (`file_path`, `range` a `SourceSpan`, `new_text`).
fn suggestion(fix: &Fix) -> Value {
    json!({
        "title": fix.title,
        "kind": fix.kind.as_str(),
        "diagnostic_code": fix.diagnostic_code,
        "entity_id": fix.subject,
        "edits": fix.edits.iter().map(|edit| json!({
            "file_path": edit.span.file,
            "range": super::span_json(&edit.span),
            "new_text": edit.new_text,
        })).collect::<Vec<Value>>(),
    })
}
