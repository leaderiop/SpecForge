use serde_json::{Value, json};

use specforge_graph::Graph;
use specforge_ops::navigate::{
    FileAnchors, FileMatch, SourceAnchor, anchors_of_file, source_anchors,
};

use crate::args::Arguments;
use crate::target::ProjectRef;
use crate::tool::{McpError, ToolOutcome};

/// `specforge.find_spec_for_source`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Path of the source file, relative to the project root (a directory lists the files under it; a trailing part of a path matches it)
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
pub fn call(project: &ProjectRef<'_>, args: Args) -> ToolOutcome {
    let file_path = args.file_path.as_str();

    let manifest = match source_anchors(&project.view()) {
        Ok(m) => m,
        Err(e) => return McpError::from(e).into(),
    };
    let FileAnchors { mode, anchors } = anchors_of_file(&manifest, file_path);
    let entities: Vec<Value> = anchors
        .iter()
        .map(|anchor| anchor_json(anchor, project.graph()))
        .collect();

    ToolOutcome::ok(json!({
        "file_path": file_path,
        "match_mode": file_match_name(mode),
        "entities": entities,
        "count": entities.len(),
    }))
}
