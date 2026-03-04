use std::collections::HashMap;

use specforge_common::{CompilerConfig, Diagnostic, KindRegistry, SourceSpan, ValidationCode};
use specforge_graph::SpecGraph;

use crate::WarmInstancePool;

/// Serialize a `SpecGraph` to JSON for the Wasm `query_graph` host function.
pub fn serialize_graph_for_wasm(graph: &SpecGraph) -> String {
    let nodes: Vec<serde_json::Value> = graph
        .nodes()
        .map(|n| {
            serde_json::json!({
                "id": n.id.raw(),
                "kind": format!("{}", n.kind),
                "title": n.title,
                "file": n.file,
            })
        })
        .collect();

    let edges: Vec<serde_json::Value> = graph
        .edges()
        .map(|(src, tgt, edge)| {
            serde_json::json!({
                "from": src.id.raw(),
                "to": tgt.id.raw(),
                "edge_type": format!("{}", edge.edge_type),
                "field_name": edge.field_name,
            })
        })
        .collect();

    serde_json::json!({
        "nodes": nodes,
        "edges": edges,
    })
    .to_string()
}

/// Run Wasm plugin validation against the built graph.
///
/// Serializes the graph to JSON, calls `validate_all` on the runtime,
/// and converts errors into `E019` diagnostics.
pub fn run_wasm_validation(pool: &mut WarmInstancePool, graph: &SpecGraph) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if let Some(runtime) = pool.runtime_mut() {
        let graph_json = serialize_graph_for_wasm(graph);
        let result = runtime.validate_all(&graph_json);
        diagnostics.extend(result.diagnostics);
        for err in result.errors {
            diagnostics.push(Diagnostic::new(
                ValidationCode::E019,
                SourceSpan::file_start("specforge.json"),
                err.to_string(),
            ));
        }
    }
    diagnostics
}

/// Register entity kinds from Wasm plugin initialization into the kind registry.
///
/// Groups registered entities by source plugin and calls `register_plugin`
/// on the kind registry for each plugin.
pub fn register_wasm_entity_kinds(
    pool: &WarmInstancePool,
    config: &CompilerConfig,
    kind_registry: &mut KindRegistry,
) {
    let mut by_plugin: HashMap<String, Vec<(String, bool)>> = HashMap::new();
    for entity in pool.registered_entities() {
        by_plugin
            .entry(entity.source_plugin.clone())
            .or_default()
            .push((entity.name.clone(), entity.testable));
    }
    for (plugin_package, kinds) in &by_plugin {
        if !plugin_package.is_empty() {
            kind_registry.register_plugin(
                plugin_package,
                kinds,
                &config.entity_kind_policy,
                &config.entity_kind_overrides,
            );
        }
    }
}
