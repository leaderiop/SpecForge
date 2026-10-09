use serde::Serialize;
use specforge_common::SourceSpan;
use specforge_common::shape::Shape;
use specforge_ops::navigate::Precision;

use crate::args::Arguments;
use crate::reply::Answered;
use crate::tool::McpError;
use specforge_ops::view::ProjectView;

/// `specforge.find_definition`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Entity ID
    entity_id: String,
}

/// `specforge.find_definition`'s reply (`McpDefinitionResult`).
#[derive(Debug, Serialize, Shape)]
pub struct Reply {
    entity_id: String,
    file_path: String,
    /// The position of the entity's name.
    line: usize,
    column: usize,
    /// The entity's block.
    source_span: SourceSpan,
    /// The entity's name as written; its block when the name could not be
    /// read.
    name_span: SourceSpan,
    /// `token` when `name_span` is the name as written, else `entity`.
    precision: Precision,
}

/// `specforge.find_definition`: where the entity is declared. `line` and
/// `column` are its name's (where a cursor goes); `source_span` is its
/// block, `name_span` its name (the block when the name could not be
/// read: `precision` says which).
pub fn call(view: ProjectView<'_>, args: Args) -> Answered<Reply> {
    let definition = super::navigator(view)
        .definition(&args.entity_id)
        .map_err(McpError::from)?;
    Ok(Reply {
        entity_id: definition.id.to_string(),
        file_path: definition.name.file.to_string(),
        line: definition.name.start_line,
        column: definition.name.start_col,
        source_span: definition.block,
        name_span: definition.name,
        precision: definition.precision,
    }
    .into())
}
