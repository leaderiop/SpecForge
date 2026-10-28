//! The harness ledger (ADR 0287): `specforge-harness`, one line per harness value a host write recorded. A field
//! whose extension declares `writer: harness` holds a value the author did not type; the ledger is how the host
//! tells the two apart (E083). [`format`] is the file, [`check`] its diagnostics, [`rewrite`] what every host write
//! and `specforge harness gc` do to it.

pub mod check;
pub mod format;
pub mod io;
pub mod overlay;
pub mod rewrite;

use std::collections::{BTreeMap, BTreeSet};

use specforge_graph::wire::{bytes, resolved, verdict_bytes};
use specforge_graph::{FieldForms, Graph};
use specforge_protocol_types::FieldWriter;
use specforge_registry::RegistryBuild;

pub use check::check;
pub use format::{HEADER, LEDGER_FILE, Ledger, Line, Parsed, VERDICT_KEY, digest};
pub use io::{Overlay, lock};
pub use overlay::LedgerOverlay;
pub use rewrite::{LedgerChange, View};

/// The harness fields of the loaded extensions, by kind: the fields whose values the ledger records, and whether a
/// loaded extension declares a judged rule, whose verdicts the ledger records too (ADR 0290).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Harness(BTreeMap<String, BTreeSet<String>>, bool);

impl Harness {
    /// Every field the registry build holds with `writer: harness`.
    pub fn of(registries: &RegistryBuild) -> Self {
        let mut kinds: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (kind, field, entry) in registries.fields().iter() {
            if entry.writer() == Some(FieldWriter::Harness) {
                kinds
                    .entry(kind.to_string())
                    .or_default()
                    .insert(field.to_string());
            }
        }
        Self(kinds, registries.judged_rules().next().is_some())
    }

    /// No extension declares a harness field or a judged rule: nothing reads the ledger.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty() && !self.1
    }

    /// The harness fields of `kind`.
    pub fn fields(&self, kind: &str) -> impl Iterator<Item = &str> {
        self.0.get(kind).into_iter().flatten().map(String::as_str)
    }

    pub fn has(&self, kind: &str, field: &str) -> bool {
        self.0
            .get(kind)
            .is_some_and(|fields| fields.contains(field))
    }

    /// The bytes a ledger digest is taken over for `field` of the entity `path` in `graph`, when it is a harness
    /// field the entity writes, or `verdict:<rule>` and the entity writes a verdict of that rule (its first).
    pub fn value(
        &self,
        graph: &Graph,
        forms: &FieldForms,
        path: &str,
        field: &str,
    ) -> Option<Vec<u8>> {
        let node = graph.node(path)?;
        if let Some(rule) = field.strip_prefix(VERDICT_KEY) {
            let verdict = node.verdicts.iter().find(|v| v.rule == rule)?;
            return Some(verdict_bytes(graph, node, verdict));
        }
        if !self.has(node.kind.raw.as_str(), field) {
            return None;
        }
        Some(bytes(&resolved(graph, forms, node, field)?))
    }
}
