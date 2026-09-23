use specforge_graph::Graph;

use crate::emit::{EmitFormat, EmitOptions, emit};
use crate::error::EmitterError;

pub fn query(
    graph: &Graph,
    entity_id: &str,
    depth: usize,
    kind_filter: &[&str],
) -> Result<String, EmitterError> {
    emit(
        graph,
        &EmitOptions {
            format: EmitFormat::Json,
            scope: Some(entity_id),
            depth: Some(depth),
            kind_filter: kind_filter.to_vec(),
            ..Default::default()
        },
    )
}
