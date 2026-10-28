//! The digests (ADR 0290): `sha256:<64 hex>` over canonical JSON, or over a file's bytes.

use specforge_graph::wire::entity_bytes;
use specforge_graph::{FieldForms, Graph, Node};
use specforge_package::Sha256;

/// An entity's digest: its resolved authored values, its `verify` statements and its method signatures. Layout,
/// comments, the entity's own verdicts, computed values and a respelling of a reference are not in it.
pub fn entity_digest(graph: &Graph, forms: &FieldForms, node: &Node) -> String {
    format!("sha256:{}", Sha256::of(&entity_bytes(graph, forms, node)))
}

/// A file's digest: its bytes with `\r\n` read as `\n`, so a checkout on another platform digests alike.
pub fn file_digest(bytes: &[u8]) -> String {
    let mut normalized = Vec::with_capacity(bytes.len());
    let mut bytes = bytes.iter().copied().peekable();
    while let Some(byte) = bytes.next() {
        if byte == b'\r' && bytes.peek() == Some(&b'\n') {
            continue;
        }
        normalized.push(byte);
    }
    format!("sha256:{}", Sha256::of(&normalized))
}
