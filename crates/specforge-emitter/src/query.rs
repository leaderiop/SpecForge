use specforge_graph::Graph;

use crate::emit::{emit, EmitFormat, EmitOptions};

pub fn query(graph: &Graph, entity_id: &str, depth: usize, kind_filter: &[&str]) -> Result<String, String> {
    emit(graph, &EmitOptions {
        format: EmitFormat::Json,
        scope: Some(entity_id),
        depth: Some(depth),
        kind_filter: kind_filter.to_vec(),
        ..Default::default()
    })
}
