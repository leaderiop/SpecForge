use specforge_graph::Graph;
use specforge_registry::{FieldRegistry, KindRegistry};

use crate::error::EmitterError;
use crate::schema::GraphProtocolSchema;

/// Output format for graph emission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitFormat {
    /// Full JSON with all fields, file locations, and line numbers.
    Json,
    /// Agent-optimized: contract, status, verify only.
    Context,
    /// Minimal: id, kind, title only.
    Brief,
    /// Graphviz DOT format.
    Dot,
}

/// Options controlling graph emission.
///
/// Use `EmitOptions::default()` for full JSON, then customize:
/// ```ignore
/// let options = EmitOptions { format: EmitFormat::Context, ..Default::default() };
/// let output = emit(&graph, &options)?;
/// ```
#[derive(Debug, Clone)]
pub struct EmitOptions<'a> {
    /// Output format (default: Json).
    pub format: EmitFormat,
    /// Scope to a subgraph rooted at this entity (default: None = full graph).
    pub scope: Option<&'a str>,
    /// Embed schema in output for V2 format (default: None = V1 format).
    pub schema: Option<&'a GraphProtocolSchema>,
    /// Token budget for truncated output (default: None = no limit).
    pub token_budget: Option<usize>,
    /// Maximum traversal depth from the scoped entity (default: None = unlimited).
    /// Only meaningful when `scope` is set.
    pub depth: Option<usize>,
    /// Filter output to only include nodes of these kinds (default: empty = all kinds).
    /// The scoped root entity is always included regardless of this filter.
    pub kind_filter: Vec<&'a str>,
    /// Kind registry for style lookups (DOT shape/color/fillcolor declared by
    /// extensions; C13-00). Default: None — nodes use built-in defaults.
    pub kind_registry: Option<&'a KindRegistry>,
    /// Field registry, so the context format keeps each entity's normative
    /// fields. Default: None — context carries only contract/status/verify.
    pub field_registry: Option<&'a FieldRegistry>,
}

impl Default for EmitOptions<'_> {
    fn default() -> Self {
        Self {
            format: EmitFormat::Json,
            scope: None,
            schema: None,
            token_budget: None,
            depth: None,
            kind_filter: Vec::new(),
            kind_registry: None,
            field_registry: None,
        }
    }
}

/// Unified graph emission — single entry point for all formats.
///
/// Replaces the combinatorial `emit_{json,context,brief}[_scoped][_with_schema]` API.
/// Returns `Err` only when `scope` references a nonexistent entity.
pub fn emit(graph: &Graph, options: &EmitOptions<'_>) -> Result<String, EmitterError> {
    // Resolve scope: use depth-limited subgraph when depth is set, otherwise full subgraph
    let scoped_graph;
    let budgeted_graph;
    let g = if let Some(scope_id) = options.scope {
        scoped_graph = if let Some(depth) = options.depth {
            graph.subgraph_depth(scope_id, depth)
        } else {
            graph.subgraph(scope_id)
        }
        .ok_or_else(|| {
            EmitterError::EntityNotFound(format!(
                "E003: unresolved scope entity '{}' — entity not found in graph",
                scope_id
            ))
        })?;
        &scoped_graph
    } else {
        graph
    };

    // Apply kind filter if specified
    let filtered_graph;
    let g = if !options.kind_filter.is_empty() {
        filtered_graph = {
            let mut fg = Graph::new();
            let root_id = options.scope;
            for node in g.nodes() {
                if Some(node.id.raw.as_str()) == root_id
                    || options.kind_filter.contains(&node.kind.raw.as_str())
                {
                    fg.add_node(node.clone());
                }
            }
            for edge in g.edges() {
                if fg.node(edge.source.as_str()).is_some()
                    && fg.node(edge.target.as_str()).is_some()
                {
                    fg.add_edge(*edge);
                }
            }
            fg
        };
        &filtered_graph
    } else {
        g
    };

    // Scoped exports reference the published schema instead of embedding it
    // (C6-07); full exports embed.
    let attach = if options.scope.is_some() {
        crate::schema::SchemaAttachment::Referenced
    } else {
        crate::schema::SchemaAttachment::Embedded
    };

    // Token budget: agent-consumed formats (Json/Context/Brief) honor it by
    // truncating to the most central subgraph that fits (C1-10). DOT is not
    // budgeted.
    let g = match options.token_budget {
        // The graph export embeds token_budget metadata (budget/estimate/
        // truncated entities); an attached schema counts toward the budget
        // and is never truncated.
        Some(budget) if options.format == EmitFormat::Json => {
            let envelope = match options.schema {
                Some(schema) => crate::budget::Envelope::attached(schema, attach),
                None => crate::budget::Envelope::schemaless(),
            };
            return crate::budget::emit_graph_within_budget(g, budget, &envelope);
        }
        // Context/brief truncate to the most central subgraph that fits.
        Some(budget) if matches!(options.format, EmitFormat::Context | EmitFormat::Brief) => {
            budgeted_graph = crate::budget::filter_graph_within_budget(g, budget, |sub| {
                match (options.format, options.schema) {
                    (EmitFormat::Context, Some(schema)) => crate::schema::emit_context_attached(
                        sub,
                        schema,
                        attach,
                        options.field_registry,
                    ),
                    (EmitFormat::Context, None) => Ok(crate::context::emit_context_with_fields(
                        sub,
                        options.field_registry,
                    )),
                    (EmitFormat::Brief, Some(schema)) => {
                        crate::schema::emit_brief_attached(sub, schema, attach)
                    }
                    (EmitFormat::Brief, None) => Ok(crate::brief::emit_brief(sub)),
                    _ => Ok(crate::json::emit_json(sub)),
                }
            })?;
            &budgeted_graph
        }
        _ => g,
    };

    // Dispatch to format
    let output = match (options.format, options.schema) {
        (EmitFormat::Json, Some(schema)) => crate::schema::emit_json_attached(g, schema, attach)?,
        (EmitFormat::Json, None) => crate::json::emit_json(g),
        (EmitFormat::Context, Some(schema)) => {
            crate::schema::emit_context_attached(g, schema, attach, options.field_registry)?
        }
        (EmitFormat::Context, None) => {
            crate::context::emit_context_with_fields(g, options.field_registry)
        }
        (EmitFormat::Brief, Some(schema)) => crate::schema::emit_brief_attached(g, schema, attach)?,
        (EmitFormat::Brief, None) => crate::brief::emit_brief(g),
        (EmitFormat::Dot, _) => crate::dot::emit_dot(
            g,
            &crate::dot::DotOptions {
                kind_registry: options.kind_registry,
                ..Default::default()
            },
        ),
    };

    Ok(output)
}
