use serde::Serialize;
use specforge_common::shape::Shape;
use specforge_ops::navigate;

use crate::args::Arguments;
use crate::reply::Answered;
use crate::target::ProjectRef;
use crate::tool::McpError;

/// `specforge.find_implementation`'s arguments.
#[derive(Debug, Arguments)]
pub struct Args {
    /// Entity ID to find implementations for
    entity_id: String,
}

/// `specforge.find_implementation`'s reply (`McpImplementationResult`).
#[derive(Debug, Serialize, Shape)]
pub struct Reply {
    entity_id: String,
    implementations: Vec<Implementation>,
    count: usize,
}

/// One source item the anchors manifest anchors the entity to.
#[derive(Debug, Serialize, Shape)]
pub struct Implementation {
    file: String,
    line: usize,
    symbol_name: String,
    item_kind: String,
    scanner: String,
}

/// The source items the anchors manifest anchors the entity to
/// (`specforge_ops::navigate::anchors_of_entity`), in manifest order.
pub fn call(project: &ProjectRef<'_>, args: Args) -> Answered<Reply> {
    let anchors = navigate::source_anchors(&project.view()).map_err(McpError::from)?;
    let implementations: Vec<Implementation> =
        navigate::anchors_of_entity(&anchors, &args.entity_id)
            .into_iter()
            .map(|a| Implementation {
                file: a.file.clone(),
                line: a.line,
                symbol_name: a.symbol_name.clone(),
                item_kind: a.item_kind.clone(),
                scanner: a.scanner.clone(),
            })
            .collect();
    Ok(Reply {
        entity_id: args.entity_id,
        count: implementations.len(),
        implementations,
    }
    .into())
}
