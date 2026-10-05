use serde_json::Value;

use crate::target::Call;
use crate::tool::{ErrorCode, Handled, McpError, ToolOutcome};

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    file: String,
}

/// `specforge.outline`: the entities a file declares. `file` is a spec
/// file of the project, relative to its spec root, as spans name it.
pub fn call(call: &mut Call<'_>, args: Args) -> Handled {
    let file = args.file.as_str();

    let nodes = call.state.graph().nodes_in_file(file);
    // A file the graph has no entity from is either empty or not there:
    // under the project's spec root (a graph built in memory with no
    // project names files as given).
    let on_disk = match call.spec_root() {
        Some(spec_root) => spec_root.join(file).exists(),
        None => std::path::Path::new(file).exists(),
    };
    if nodes.is_empty() && !on_disk {
        return Err(
            McpError::new(ErrorCode::FileNotFound, format!("File not found: {file}"))
                .with_argument("file")
                .into(),
        );
    }

    let mut entries: Vec<Value> = nodes
        .iter()
        .map(|n| {
            let mut entry = serde_json::json!({
                "entity_id": n.id.raw,
                "kind": n.kind.raw,
                "title": n.title,
                "range": range(&n.source_span),
            });
            if !n.methods.is_empty() {
                entry["children"] = n
                    .methods
                    .iter()
                    .map(|m| {
                        // `name(param: Type, opt?: Type) -> Ret`, as declared.
                        let params: Vec<String> = m
                            .params
                            .iter()
                            .map(|p| {
                                let optional = if p.optional { "?" } else { "" };
                                format!("{}{optional}: {}", p.name, p.ty)
                            })
                            .collect();
                        let returns = m
                            .returns
                            .as_ref()
                            .map(|r| format!(" -> {r}"))
                            .unwrap_or_default();
                        serde_json::json!({
                            "entity_id": format!("{}.{}", n.id.raw, m.name),
                            "kind": "method",
                            "title": format!("{}({}){returns}", m.name, params.join(", ")),
                            "range": range(&m.span),
                        })
                    })
                    .collect();
            }
            entry
        })
        .collect();

    // Sort by line number
    entries.sort_by_key(|e| e["range"]["start_line"].as_u64().unwrap_or(0));

    Ok(ToolOutcome::ok(Value::Array(entries)))
}

fn range(span: &specforge_common::SourceSpan) -> Value {
    serde_json::json!({
        "file": span.file,
        "start_line": span.start_line,
        "start_col": span.start_col,
        "end_line": span.end_line,
        "end_col": span.end_col,
    })
}
