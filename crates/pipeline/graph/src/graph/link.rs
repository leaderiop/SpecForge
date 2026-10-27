//! The graph build's linking step: the one place references resolve, so a
//! compile and every update agree (`GraphBuild`). What a field's value
//! references is its [`Reading`](crate::Reading)'s answer, never the
//! parser's shape (ADR 0093).

use specforge_common::{Diagnostic, DiagnosticData, Sym, codes};
use std::collections::{BTreeMap, BTreeSet};

use crate::{Edge, FieldForms, Graph, Node};

mod reference;

/// An edge as written, and whether a declared single reference wrote it.
struct WrittenEdge {
    edge: Edge,
    single: bool,
}

impl Graph {
    /// Resolve every node's references into edges and report what does not
    /// resolve (`Graph::resolve` decides each, ADR 0117): E003 (with a
    /// fuzzy-match suggestion when there is one), or I004 for a field whose
    /// target kind no loaded extension declares, naming the extension in
    /// `owners` that does. Then W061 over the edges but the declared single
    /// references': a single reference is never part of a reference cycle.
    ///
    /// Clears all existing edges first, then adds each once, in written order.
    /// Diagnostics come in node order (the graph's nodes are sorted), then
    /// the cycles'.
    pub(crate) fn link(
        &mut self,
        forms: &FieldForms,
        owners: &BTreeMap<String, String>,
        scoped: &BTreeMap<String, String>,
    ) -> Vec<Diagnostic> {
        self.clear_edges();
        let mut diagnostics = Vec::new();
        // Every edge in written order, and whether it is a single reference's.
        let mut edges: Vec<WrittenEdge> = Vec::new();

        for node in self.nodes.values() {
            let kind = node.kind.raw.as_str();
            for top in node.fields.entries() {
                // A reference inside a typed block is held by this node, read from it and labelled by its declared
                // member path (ADR 0285); diagnostics name the written path.
                for (path, reading, entry) in forms.readings(kind, top) {
                    let single = reading.is_single_reference();
                    let declared = path.declared();
                    let target = forms.target(kind, &declared);
                    let from = held_from(node, scoped, &declared);
                    let label = Sym::new(&declared);
                    for reference in reading.references(entry) {
                        let resolution = self.resolve(target.clone(), owners, from, reference.id);
                        if let Some(path) = resolution.path() {
                            edges.push(WrittenEdge {
                                edge: Edge {
                                    source: node.id.raw,
                                    target: path,
                                    label,
                                },
                                single,
                            });
                        }
                        diagnostics.extend(resolution.diagnostic(
                            node,
                            Sym::new(path.as_str()),
                            &reference,
                        ));
                    }
                }
            }
        }

        // W061 reads the written references but a declared single
        // reference's (ADR 0093 D7). A derived edge joins later and is never
        // one: a recursive type is a dependency, not a reference cycle (ADR 0204).
        let references = super::successors(edges.iter().filter(|w| !w.single).map(|w| &w.edge));
        for written in edges {
            self.add_edge(written.edge);
        }
        diagnostics.extend(cycle_warnings(&references));
        diagnostics
    }

    /// The references that name an existing entity but have no edge
    /// (source, target, field). Empty after every link.
    #[cfg_attr(
        not(debug_assertions),
        allow(dead_code, reason = "only debug assertions call it")
    )]
    pub(crate) fn unlinked_references(
        &self,
        forms: &FieldForms,
        scoped: &BTreeMap<String, String>,
    ) -> Vec<(Sym, Sym, Sym)> {
        let mut unlinked = Vec::new();
        for node in self.nodes.values() {
            let source = node.id.raw;
            let edges = self.source_index.get(&source);
            let kind = node.kind.raw.as_str();
            for top in node.fields.entries() {
                for (path, reading, entry) in forms.readings(kind, top) {
                    let declared = path.declared();
                    let label = Sym::new(&declared);
                    let field_target = forms.target(kind, &declared);
                    let from = held_from(node, scoped, &declared);
                    for reference in reading.references(entry) {
                        let Some(target) = self
                            .resolve(field_target.clone(), &BTreeMap::new(), from, reference.id)
                            .path()
                        else {
                            continue;
                        };
                        let linked = edges.is_some_and(|indices| {
                            indices.iter().any(|&i| {
                                let edge = &self.edges[i];
                                edge.target == target && edge.label == label
                            })
                        });
                        if !linked {
                            unlinked.push((source, target, label));
                        }
                    }
                }
            }
        }
        unlinked
    }

    /// Panic, naming them, when [`Self::unlinked_references`] is not empty: a
    /// SpecForge bug, never a spec error.
    #[cfg_attr(
        not(debug_assertions),
        allow(dead_code, reason = "only debug assertions call it")
    )]
    pub(crate) fn assert_linked(&self, forms: &FieldForms, scoped: &BTreeMap<String, String>) {
        let unlinked = self.unlinked_references(forms, scoped);
        assert!(
            unlinked.is_empty(),
            "SpecForge bug: references to existing entities without their edge (source, target, field): {}",
            unlinked
                .iter()
                .map(|(source, target, field)| format!("({source}, {target}, {field})"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

impl Graph {
    /// Panic, naming it, when a node's scope field links to an entity other than the node's scope: the build
    /// resolves the field once for the path and once for the edge, from the project both times (ADR 0284). A
    /// SpecForge bug, never a spec error.
    #[cfg(debug_assertions)]
    pub(crate) fn assert_scoped(&self, inputs: &crate::GraphInputs) {
        for node in self.nodes.values() {
            let kind = node.kind.raw.as_str();
            let Some(field) = inputs.scopes.get(kind) else {
                continue;
            };
            let target = inputs.field_forms.target(kind, field);
            for edge in self.edges_from(node.id.raw.as_str()) {
                let admitted = self
                    .scopes
                    .kind(edge.target.as_str())
                    .is_some_and(|k| target.as_ref().is_none_or(|t| t.admits(k.as_str())));
                assert!(
                    edge.label != field.as_str() || !admitted || Some(edge.target) == node.scope,
                    "SpecForge bug: '{}' is scoped by '{}', but its '{field}' links to '{}'",
                    node.id.raw,
                    node.scope.map_or("the project", |s| s.as_str()),
                    edge.target
                );
            }
        }
    }
}

/// Where a reference `node` writes in `field` is read from: the node itself, whose scope chain answers; `None` for
/// its scope field, which decides the chain and so reads from the project (ADR 0284).
fn held_from(node: &Node, scoped: &BTreeMap<String, String>, field: &str) -> Option<Sym> {
    let is_scope = scoped
        .get(node.kind.raw.as_str())
        .is_some_and(|f| f == field);
    (!is_scope).then_some(node.id.raw)
}

/// W061 for every cycle among `references`: one per strongly connected set,
/// its path walking every entity of it.
fn cycle_warnings(references: &BTreeMap<Sym, BTreeSet<Sym>>) -> Vec<Diagnostic> {
    specforge_cycles::cycles(references)
        .into_iter()
        .map(|cycle| {
            let path: Vec<String> = cycle.path.iter().map(|s| s.to_string()).collect();
            Diagnostic::new(
                codes::W061,
                format!("reference cycle detected: {}", path.join(" -> ")),
            )
            .with_suggestion("break the cycle by removing or inverting one reference")
            .with_data(DiagnosticData::ReferenceCycle { path })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_test_macros::test as specforge_test;

    const REFERENCING: &str = r#"
behavior alpha "A" {
  contract "first"
  invariants [inv_one]
}
invariant inv_one "Invariant One" {
  contract "must hold"
}
"#;

    /// The graph and diagnostics of `source`, built with no extension loaded.
    fn built(source: &str) -> (Graph, Vec<Diagnostic>) {
        crate::build_graph(&[specforge_parser::parse(source, "main.spec")])
    }

    #[specforge_test(
        behavior = "detect_dangling_references",
        verify = "a reference to an existing entity without its edge fails the linker's assertion"
    )]
    #[should_panic(expected = "without their edge")]
    fn a_reference_without_its_edge_fails_the_assertion() {
        let (mut graph, _) = built(REFERENCING);
        // What a linker that forgot an edge leaves behind.
        graph.clear_edges();

        graph.assert_linked(&FieldForms::default(), &BTreeMap::new());
    }

    #[test]
    fn unlinked_references_names_the_missing_edge() {
        let (mut graph, _) = built(REFERENCING);
        graph.clear_edges();

        let unlinked: Vec<(&str, &str, &str)> = graph
            .unlinked_references(&FieldForms::default(), &BTreeMap::new())
            .iter()
            .map(|(s, t, f)| (s.as_str(), t.as_str(), f.as_str()))
            .collect();

        assert_eq!(unlinked, [("alpha", "inv_one", "invariants")]);
    }

    #[specforge_test(
        behavior = "detect_dangling_references",
        verify = "reference with corresponding graph edge passes"
    )]
    fn a_linked_reference_passes() {
        let (graph, _) = built(REFERENCING);
        graph.assert_linked(&FieldForms::default(), &BTreeMap::new());
        assert!(
            graph
                .unlinked_references(&FieldForms::default(), &BTreeMap::new())
                .is_empty()
        );
    }

    #[specforge_test(
        behavior = "detect_dangling_references",
        verify = "an empty graph passes the linker's assertion"
    )]
    fn an_empty_graph_passes() {
        let (graph, _) = built("");
        assert_eq!(graph.edge_count(), 0);
        graph.assert_linked(&FieldForms::default(), &BTreeMap::new());
    }

    #[specforge_test(
        behavior = "detect_dangling_references",
        verify = "Detect Dangling References: dangling reference detection holds — graph_built_fired, resolver_integrity_verified, no_duplicate_diagnostics"
    )]
    fn a_missing_target_is_e003_once() {
        // An unresolved id is the linker's E003, once; it names no existing
        // entity, so the assertion has nothing to say.
        let source = "behavior beta \"B\" { contract \"second\" invariants [missing] }\n";
        let (graph, diagnostics) = built(source);
        assert_eq!(diagnostics.iter().filter(|d| d.code == "E003").count(), 1);
        assert!(
            graph
                .unlinked_references(&FieldForms::default(), &BTreeMap::new())
                .is_empty()
        );
        graph.assert_linked(&FieldForms::default(), &BTreeMap::new());
    }
}
