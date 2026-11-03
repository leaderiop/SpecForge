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
pub use format::{FACTS_KEY, HEADER, LEDGER_FILE, Ledger, Line, Parsed, VERDICT_KEY, digest};
pub use io::{Overlay, lock};
pub use overlay::LedgerOverlay;
pub use rewrite::{LedgerChange, View};

/// What the ledger records for the loaded extensions: the harness fields, by kind; whether an extension declares a
/// judged rule, whose verdicts the ledger records too (ADR 0290); whether a kind is collector-written, whose fact
/// stores it records (ADR 0327); and the collectors whose `facts:` lines it keeps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Harness {
    fields: BTreeMap<String, BTreeSet<String>>,
    judged: bool,
    collected: bool,
    collectors: BTreeSet<String>,
}

impl Harness {
    /// Every field the registry build holds with `writer: harness`, its judged rules, collected kinds and collectors.
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
        Self {
            fields: kinds,
            judged: registries.judged_rules().next().is_some(),
            collected: registries.kinds().any_collected(),
            collectors: registries
                .declarations()
                .iter()
                .flat_map(|d| d.collectors.iter().map(|c| c.name.clone()))
                .collect(),
        }
    }

    /// No extension declares a harness field, a judged rule or a collector-written kind: nothing reads the ledger.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty() && !self.judged && !self.collected
    }

    /// Whether a loaded extension declares the collector `name`: its `facts:` line is kept (ADR 0327 D4).
    pub fn has_collector(&self, name: &str) -> bool {
        self.collectors.contains(name)
    }

    /// The harness fields of `kind`.
    pub fn fields(&self, kind: &str) -> impl Iterator<Item = &str> {
        self.fields
            .get(kind)
            .into_iter()
            .flatten()
            .map(String::as_str)
    }

    pub fn has(&self, kind: &str, field: &str) -> bool {
        self.fields
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
