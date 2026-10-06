use serde::Serialize;
use serde_json::Value;
use specforge_diagnostics::{Code, codes};
use specforge_graph::{Graph, Node};
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::error::EmitterError;
use crate::json::{JsonEdge, SCHEMA_VERSION, field_map_to_json, sorted_edges};
use crate::schema::{GraphProtocolSchema, SchemaAttachment, SchemaRefBlock};

/// The diagnostic a budget too small for any export fails with.
const BUDGET_TOO_SMALL: Code = codes::E062;

/// What wraps the entities of a graph export: `format_version`,
/// `schema_version`, and for a V2 export the schema, embedded or referenced.
/// A budgeted export always carries it whole; only entities are truncated.
pub(crate) struct Envelope<'a> {
    format_version: &'static str,
    schema_version: String,
    schema: Option<&'a GraphProtocolSchema>,
    schema_ref: Option<SchemaRefBlock>,
}

impl<'a> Envelope<'a> {
    /// The schemaless V1 envelope, as [`crate::json::emit_json`] writes it.
    pub(crate) fn schemaless() -> Self {
        Self {
            format_version: "1.0",
            schema_version: SCHEMA_VERSION.to_string(),
            schema: None,
            schema_ref: None,
        }
    }

    /// The V2 envelope, as [`crate::schema::emit_json_attached`] writes it.
    pub(crate) fn attached(schema: &'a GraphProtocolSchema, attach: SchemaAttachment) -> Self {
        let (embedded, reference) = match attach {
            SchemaAttachment::Embedded => (Some(schema), None),
            SchemaAttachment::Referenced => (None, Some(SchemaRefBlock::for_schema(schema))),
        };
        Self {
            format_version: "2.0",
            schema_version: schema.schema_version.to_string(),
            schema: embedded,
            schema_ref: reference,
        }
    }
}

#[derive(Serialize)]
struct BudgetedGraph<'a> {
    format_version: &'static str,
    schema_version: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema: Option<&'a GraphProtocolSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_ref: Option<&'a SchemaRefBlock>,
    nodes: Vec<&'a BudgetedNode>,
    edges: Vec<&'a JsonEdge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_budget: Option<TokenBudgetResult<'a>>,
}

#[derive(Serialize)]
struct BudgetedNode {
    id: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    file: String,
    line: usize,
    fields: BTreeMap<String, Value>,
}

#[derive(Serialize)]
struct TokenBudgetResult<'a> {
    strategy: &'static str,
    budget_tokens: usize,
    estimated_tokens: usize,
    truncated_entities: Vec<&'a str>,
}

/// The token cost a `--max-tokens` budget measures: each word-like run
/// counts one token, each JSON structural character (`{ } [ ] : , "`) half.
pub fn estimate_tokens(s: &str) -> usize {
    // Count word-like sequences (alphanumeric runs) as ~1 token each,
    // plus JSON structural characters ({, }, [, ], :, ,) as ~0.5 tokens each.
    let mut words = 0usize;
    let mut structural = 0usize;
    let mut in_word = false;

    for ch in s.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            if !in_word {
                words += 1;
                in_word = true;
            }
        } else {
            in_word = false;
            if matches!(ch, '{' | '}' | '[' | ']' | ':' | ',' | '"') {
                structural += 1;
            }
        }
    }

    // Each word ≈ 1 token, structural chars ≈ 0.5 tokens each (round up)
    words + structural.div_ceil(2)
}

fn degree_centrality(graph: &Graph) -> HashMap<String, usize> {
    let mut degrees: HashMap<String, usize> = HashMap::new();
    for node in graph.nodes() {
        degrees.entry(node.id.raw.to_string()).or_insert(0);
    }
    for edge in graph.edges() {
        *degrees.entry(edge.source.to_string()).or_insert(0) += 1;
        *degrees.entry(edge.target.to_string()).or_insert(0) += 1;
    }
    degrees
}

/// The graph's nodes in truncation order: least connected first, ties by ID.
fn nodes_by_priority(graph: &Graph) -> Vec<&Node> {
    let degrees = degree_centrality(graph);
    let mut nodes: Vec<_> = graph.nodes().into_iter().collect();
    nodes.sort_by(|a, b| {
        let da = degrees.get(a.id.raw.as_str()).copied().unwrap_or(0);
        let db = degrees.get(b.id.raw.as_str()).copied().unwrap_or(0);
        da.cmp(&db).then(a.id.raw.cmp(&b.id.raw))
    });
    nodes
}

fn budgeted_node(n: &Node) -> BudgetedNode {
    BudgetedNode {
        id: n.id.raw.to_string(),
        kind: n.kind.raw.to_string(),
        title: n.title.clone(),
        file: n.source_span.file.to_string(),
        line: n.source_span.start_line,
        fields: field_map_to_json(&n.fields),
    }
}

/// The schemaless graph export fitted to `max_tokens`: the whole export when
/// it fits, otherwise the most central entities that fit with a
/// `token_budget` block naming the rest. See [`emit_graph_within_budget`].
pub fn emit_json_with_budget(graph: &Graph, max_tokens: usize) -> Result<String, EmitterError> {
    emit_graph_within_budget(graph, max_tokens, &Envelope::schemaless())
}

/// The graph export in `envelope`, fitted to `max_tokens` by the
/// `prioritize` strategy.
///
/// The envelope always counts and is never truncated: when it carries an
/// embedded schema, a schema over the budget fails with E062 rather than
/// shipping part of it. Entities fill what the envelope leaves, least
/// connected dropped first along with their edges; the dropped IDs go in
/// the `token_budget` block. When no entity fits, the export is the envelope
/// with no entities and that block; when even that is over the budget, E062.
pub(crate) fn emit_graph_within_budget(
    graph: &Graph,
    max_tokens: usize,
    envelope: &Envelope<'_>,
) -> Result<String, EmitterError> {
    let edges = sorted_edges(graph);
    let render = |nodes: Vec<&BudgetedNode>, token_budget: Option<TokenBudgetResult<'_>>| {
        let kept: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
        let output = BudgetedGraph {
            format_version: envelope.format_version,
            schema_version: &envelope.schema_version,
            schema: envelope.schema,
            schema_ref: envelope.schema_ref.as_ref(),
            nodes,
            edges: edges
                .iter()
                .filter(|e| kept.contains(e.source.as_str()) && kept.contains(e.target.as_str()))
                .collect(),
            token_budget,
        };
        serde_json::to_string(&output).map_err(|e| EmitterError::SerializationError(e.to_string()))
    };

    // Everything, in graph order, when it fits.
    let all: Vec<BudgetedNode> = graph.nodes().into_iter().map(budgeted_node).collect();
    let full = render(all.iter().collect(), None)?;
    if estimate_tokens(&full) <= max_tokens {
        return Ok(full);
    }

    if let Some(schema) = envelope.schema {
        let schema_json = serde_json::to_string(schema)
            .map_err(|e| EmitterError::SerializationError(e.to_string()))?;
        let schema_tokens = estimate_tokens(&schema_json);
        if schema_tokens > max_tokens {
            return Err(EmitterError::Other(format!(
                "{BUDGET_TOO_SMALL}: the embedded schema alone costs {schema_tokens} tokens, over \
                 the token budget of {max_tokens}; raise the budget or export without the schema"
            )));
        }
    }

    // The export with the `cut` least central entities dropped, and its cost.
    let priority: Vec<BudgetedNode> = nodes_by_priority(graph)
        .into_iter()
        .map(budgeted_node)
        .collect();
    let with_cut = |cut: usize| -> Result<(String, usize), EmitterError> {
        let truncated: Vec<&str> = priority[..cut].iter().map(|n| n.id.as_str()).collect();
        let kept: Vec<&BudgetedNode> = priority[cut..].iter().collect();
        let marker = |estimated_tokens| TokenBudgetResult {
            strategy: "prioritize",
            budget_tokens: max_tokens,
            estimated_tokens,
            truncated_entities: truncated.clone(),
        };
        // The estimate is one number, one token whatever its value, so the
        // placeholder costs what the real figure does.
        let estimated = estimate_tokens(&render(kept.clone(), Some(marker(0)))?);
        Ok((render(kept, Some(marker(estimated)))?, estimated))
    };

    // Cutting an entity always lowers the cost (its ID is cheaper in the
    // truncated list than its whole entry), so the fewest cuts that fit are
    // found by bisection. (An empty graph never gets here: its full export
    // is the cheapest there is, so it fits or the empty export fails below.)
    let n = priority.len();
    let (_, empty_cost) = with_cut(n)?;
    if empty_cost > max_tokens {
        return Err(EmitterError::Other(format!(
            "{BUDGET_TOO_SMALL}: the token budget of {max_tokens} cannot hold even an export \
             with no entities ({empty_cost} tokens: the envelope and the truncated entity IDs); \
             raise the budget"
        )));
    }
    let (mut lo, mut hi) = (1, n);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if with_cut(mid)?.1 <= max_tokens {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    Ok(with_cut(lo)?.0)
}

/// Build the sub-graph that fits `max_tokens` when rendered by `render`,
/// dropping least-connected nodes first (same prioritize strategy as
/// [`emit_json_with_budget`]). Returns the filtered graph.
pub fn filter_graph_within_budget<F>(
    graph: &Graph,
    max_tokens: usize,
    render: F,
) -> Result<Graph, crate::error::EmitterError>
where
    F: Fn(&Graph) -> Result<String, crate::error::EmitterError>,
{
    let full = render(graph)?;
    if estimate_tokens(&full) <= max_tokens {
        return Ok(graph.clone());
    }

    let mut kept: Vec<_> = nodes_by_priority(graph);
    loop {
        let kept_ids: HashSet<&str> = kept.iter().map(|n| n.id.raw.as_str()).collect();

        let mut filtered = Graph::new();
        for n in &kept {
            filtered.add_node(Node {
                id: n.id,
                kind: n.kind,
                title: n.title.clone(),
                fields: n.fields.clone(),
                source_span: n.source_span.clone(),
                methods: n.methods.clone(),
            });
        }
        for e in graph.edges() {
            if kept_ids.contains(e.source.as_str()) && kept_ids.contains(e.target.as_str()) {
                filtered.add_edge(specforge_graph::Edge {
                    source: e.source,
                    target: e.target,
                    label: e.label,
                });
            }
        }

        let rendered = render(&filtered)?;
        if estimate_tokens(&rendered) <= max_tokens || kept.len() <= 1 {
            return Ok(filtered);
        }
        kept.remove(0);
    }
}

pub fn emit_json_with_budget_strategy(
    graph: &Graph,
    max_tokens: usize,
    strategy: &str,
) -> Result<String, EmitterError> {
    let full = crate::json::emit_json(graph);
    let est = estimate_tokens(&full);

    if est <= max_tokens {
        return Ok(full);
    }

    match strategy {
        "error" => Err(EmitterError::Other(format!(
            "token budget exceeded: estimated {} tokens, budget is {}",
            est, max_tokens
        ))),
        _ => emit_json_with_budget(graph, max_tokens),
    }
}
