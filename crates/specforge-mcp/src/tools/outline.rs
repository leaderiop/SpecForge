use serde_json::{Value, json};
use specforge_ops::navigate::{OutlineEntry, outline};

use crate::args::Arguments;
use crate::target::Call;
use crate::tool::{Handled, ToolOutcome, file_not_found};

/// `specforge.outline`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// File path
    file: String,
}

/// `specforge.outline`: the entities a file declares, in line order, each
/// with its method members, each selecting its name: the tree the LSP's
/// document symbols are (`specforge_ops::navigate::outline`). `file` is a
/// spec file of the project, relative to its spec root, as spans name it.
pub fn call(call: &mut Call<'_>, args: Args) -> Handled {
    let file = args.file.as_str();
    let entries = outline(&super::navigator(call), file);
    // A file the graph has no entity from is either empty or not there:
    // under the project's spec root. With nothing served no file is the
    // project's (the dispatcher makes the refusal the no-project one,
    // ADR 0025).
    let on_disk = call
        .spec_root()
        .is_some_and(|spec_root| spec_root.join(file).exists());
    if entries.is_empty() && !on_disk {
        return Err(file_not_found(file).into());
    }
    Ok(ToolOutcome::ok(Value::Array(
        entries.iter().map(entry).collect(),
    )))
}

/// One entry as an `McpOutlineEntry`: `range` is its block, `name_range`
/// its name; a method child is `<entity>.<method>`, titled by its
/// signature.
fn entry(entry: &OutlineEntry) -> Value {
    let mut value = json!({
        "entity_id": entry.id,
        "kind": entry.kind,
        "title": entry.title,
        "range": super::span_json(&entry.block),
        "name_range": super::span_json(&entry.name),
    });
    if !entry.children.is_empty() {
        value["children"] = entry
            .children
            .iter()
            .map(|m| {
                json!({
                    "entity_id": format!("{}.{}", entry.id, m.name),
                    "kind": "method",
                    "title": m.signature,
                    "range": super::span_json(&m.block),
                    "name_range": super::span_json(&m.name_span),
                })
            })
            .collect();
    }
    value
}
