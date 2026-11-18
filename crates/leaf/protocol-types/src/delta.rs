//! What changed in the project's graph since a base (ADR 0376 D6, D7): the nodes added, removed or modified, each
//! named by its kind and id with the digests of its content, and the edges added or removed. What `specforge diff`
//! names and what a pass that declares the `delta` input is sent. The full content of an entity is not here: it stays
//! in the pass's `entities`.

use serde::{Deserialize, Serialize};

/// The graph's change since [`DeltaPayload::base`]. An empty change still names its base, so a pass that declared
/// the delta tells "nothing changed" from "not sent".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeltaPayload {
    /// What the change is taken against.
    pub base: DeltaBase,
    /// Each changed node, sorted by kind then id, compared as bytes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<DeltaNode>,
    /// The edges the graph gained, sorted by source, target then label.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub added_edges: Vec<DeltaEdge>,
    /// The edges the graph lost, sorted by source, target then label.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removed_edges: Vec<DeltaEdge>,
}

/// What a delta is taken against.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeltaBase {
    /// No base: the empty graph, so every node is added.
    #[default]
    None,
    /// The project at a commit: its full sha.
    Revision { id: String },
    /// The graph of the session's last completed check.
    Previous,
}

/// One changed node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeltaNode {
    /// Its kind; a node whose kind changed is its old kind removed and its new kind added.
    pub kind: String,
    /// Its full path.
    pub id: String,
    pub change: DeltaChange,
    /// A modified node only: the fields whose value changed, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<String>,
    /// A modified node only: what else about it changed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aspects: Vec<DeltaAspect>,
    /// A modified node only: the digest of its content at the base, `sha256:<64 hex>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    /// An added or modified node: the digest of its content now, `sha256:<64 hex>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

/// How a node changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaChange {
    Added,
    Removed,
    Modified,
}

/// What about a modified node changed besides its fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaAspect {
    /// Its title.
    Title,
    /// Its declared methods.
    Methods,
    /// Its verify obligations.
    Obligations,
    /// The judgements recorded of it.
    Verdicts,
    /// The edges from it.
    Edges,
}

/// One changed edge.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeltaEdge {
    /// The full path of the node it leaves.
    pub source: String,
    /// The full path of the node it reaches.
    pub target: String,
    /// What it is.
    pub label: String,
}

#[cfg(test)]
mod tests;
