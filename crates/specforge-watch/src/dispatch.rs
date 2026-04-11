use crate::delta::GraphDelta;
use specforge_graph::Graph;
use std::collections::HashSet;

/// How an extension validator should be dispatched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatorInput {
    /// Extension supports incremental validation — receives only the delta.
    Delta,
    /// Extension does not support incremental — receives the full graph.
    FullGraph,
}

/// Describes an entity kind's incremental support within an extension.
#[derive(Debug, Clone)]
pub struct KindDescriptor {
    pub kind_name: String,
    pub incremental: bool,
}

/// Describes an extension validator to dispatch.
#[derive(Debug, Clone)]
pub struct ValidatorDescriptor {
    pub extension_name: String,
    pub kinds: Vec<KindDescriptor>,
}

/// Result of planning which validators to dispatch and with what input.
#[derive(Debug, Clone)]
pub struct DispatchPlan {
    pub entries: Vec<DispatchEntry>,
}

#[derive(Debug, Clone)]
pub struct DispatchEntry {
    pub extension_name: String,
    pub input: ValidatorInput,
}

/// Plan how to dispatch validators based on extension manifest declarations.
/// Per-kind granularity: if ANY kind in the delta is marked incremental=false,
/// the extension receives the full graph for that dispatch.
pub fn plan_incremental_dispatch(
    validators: &[ValidatorDescriptor],
    delta: &GraphDelta,
    _graph: &Graph,
) -> DispatchPlan {
    // Collect kinds present in the delta
    let delta_kinds: HashSet<&str> = delta
        .added_nodes
        .iter()
        .map(|n| n.kind.as_str())
        .chain(delta.removed_nodes.iter().map(|n| n.kind.as_str()))
        .chain(delta.modified_nodes.iter().map(|n| n.id.as_str()))
        .collect();

    let entries = validators
        .iter()
        .map(|v| {
            // Check if any of this extension's non-incremental kinds appear in the delta
            let needs_full_graph = v.kinds.iter().any(|k| {
                !k.incremental && delta_kinds.contains(k.kind_name.as_str())
            });

            // If all kinds are incremental, use delta; otherwise full graph
            let all_incremental = v.kinds.iter().all(|k| k.incremental);

            let input = if needs_full_graph || !all_incremental && !v.kinds.is_empty() {
                // If extension has mixed kinds and non-incremental kind is in delta
                if needs_full_graph {
                    ValidatorInput::FullGraph
                } else if all_incremental {
                    ValidatorInput::Delta
                } else {
                    // Mixed kinds but non-incremental kinds NOT in delta — delta is fine
                    ValidatorInput::Delta
                }
            } else if v.kinds.is_empty() {
                // No kinds declared — treat as non-incremental
                ValidatorInput::FullGraph
            } else {
                ValidatorInput::Delta
            };

            DispatchEntry {
                extension_name: v.extension_name.clone(),
                input,
            }
        })
        .collect();

    DispatchPlan { entries }
}
