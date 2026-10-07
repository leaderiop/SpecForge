use serde_json::{Value, json};

use specforge_common::inference::anchors::{self, SourceAnchor};
use specforge_graph::Graph;
use specforge_ops::navigate::{FileAnchors, FileMatch, anchors_of_file};

use crate::target::Call;
use crate::tool::{Handled, ToolOutcome};

#[derive(Debug, serde::Deserialize)]
pub struct Args {
    file_path: String,
}

/// How a source-file query matched its anchors, as MCP spells it: the
/// file itself, a directory the files are under, or a trailing part of
/// their paths.
pub(crate) fn file_match_name(mode: FileMatch) -> &'static str {
    match mode {
        FileMatch::Exact => "exact",
        FileMatch::Under => "directory",
        FileMatch::Suffix => "suffix_path",
        FileMatch::None => "none",
    }
}

/// One entity anchored to a source file, as MCP spells it: the anchor,
/// and the entity's kind when the graph has it.
pub(crate) fn anchor_json(anchor: &SourceAnchor, graph: &Graph) -> Value {
    json!({
        "entity_id": anchor.entity_id,
        "kind": graph.node(&anchor.entity_id).map(|n| n.kind.raw.as_str()),
        "file": anchor.file,
        "line": anchor.line,
        "symbol_name": anchor.symbol_name,
        "item_kind": anchor.item_kind,
        "confidence": anchor.confidence,
    })
}

/// The entities anchored to the source files `file_path` names, under the
/// one file rule (`specforge_ops::navigate::anchors_of_file`).
pub fn call(call: &mut Call<'_>, args: Args) -> Handled {
    let file_path = args.file_path.as_str();

    let project = call.project()?;
    let manifest = match anchors::load_anchor_manifest(project.root) {
        Ok(m) => m,
        Err(e) => return Ok(super::manifest_error(e)),
    };
    let FileAnchors { mode, anchors } = anchors_of_file(&manifest, file_path);
    let entities: Vec<Value> = anchors
        .iter()
        .map(|anchor| anchor_json(anchor, project.graph()))
        .collect();

    Ok(ToolOutcome::ok(json!({
        "file_path": file_path,
        "match_mode": file_match_name(mode),
        "entities": entities,
        "count": entities.len(),
    })))
}
