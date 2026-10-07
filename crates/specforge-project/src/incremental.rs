//! The incremental rebuild a [`crate::ProjectSession`] runs on each change.
//!
//! A parse depends only on its own file's text, and references resolve
//! across the whole project without `use` (ADR 0004 D1-a), so a change
//! re-parses exactly the files that changed: an importer of a changed
//! file parses the same as before. The graph is then patched red-green
//! (the changed files' entities stripped and re-added, references
//! re-linked over the whole graph) and the graph-build diagnostics
//! recomputed from the cached parses, which is what a cold build of the
//! same sources yields.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use specforge_common::{Diagnostic, Sym};
use specforge_graph::{Graph, GraphConfig, GraphDelta, Node, build_graph_with_config};
use specforge_parser::{SpecFile, parse_incremental};

use specforge_graph as delta;

/// What one rebuild did.
pub(crate) struct Rebuild {
    pub delta: GraphDelta,
    /// The files re-parsed or dropped (sorted).
    pub rebuilt_files: Vec<String>,
    /// Files whose graph-build diagnostics changed (sorted).
    pub changed_diagnostic_files: Vec<String>,
    /// The comparison with a cold rebuild, when verifying.
    pub verification: Option<Result<(), String>>,
}

/// The cached parses, retained trees, graph and graph-build diagnostics
/// of a project's sources.
pub(crate) struct IncrementalBuild {
    graph: Graph,
    /// The text each cached parse was made from: what a reader resolves a
    /// span of the graph against (a span is a position in this text), and
    /// what a whole-file replacement is diffed against to edit the
    /// retained tree.
    sources: HashMap<String, Arc<str>>,
    parsed_files: HashMap<String, SpecFile>,
    /// Retained tree-sitter trees, fed back into `parse_incremental` so
    /// keystroke-sized edits reuse unchanged subtrees.
    trees: HashMap<String, tree_sitter::Tree>,
    /// Graph-build diagnostics by file (`""`: no span).
    file_diagnostics: HashMap<String, Vec<Diagnostic>>,
    graph_config: GraphConfig,
    verify: bool,
}

impl IncrementalBuild {
    /// Seeded by a cold build of `files`, parsed from `sources`, which
    /// produced `graph` and `diagnostics`.
    pub fn from_cold_build(
        files: Vec<(String, SpecFile)>,
        sources: HashMap<String, Arc<str>>,
        graph: Graph,
        diagnostics: &[Diagnostic],
        graph_config: GraphConfig,
    ) -> Self {
        IncrementalBuild {
            graph,
            sources,
            parsed_files: files.into_iter().collect(),
            trees: HashMap::new(),
            file_diagnostics: partition_by_file(diagnostics),
            graph_config,
            verify: false,
        }
    }

    pub fn empty() -> Self {
        Self::from_cold_build(
            Vec::new(),
            HashMap::new(),
            Graph::new(),
            &[],
            GraphConfig::default(),
        )
    }

    /// Compare every rebuild with a cold one (costly).
    pub fn set_verify(&mut self, enabled: bool) {
        self.verify = enabled;
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// The text `path`'s cached parse was made from.
    pub fn source_text(&self, path: &str) -> Option<&Arc<str>> {
        self.sources.get(path)
    }

    /// Every cached parse's text, by path (the texts are shared, not
    /// copied).
    pub fn source_texts(&self) -> HashMap<String, Arc<str>> {
        self.sources.clone()
    }

    pub fn file_diagnostics(&self, path: &str) -> &[Diagnostic] {
        self.file_diagnostics
            .get(path)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Every file that has graph-build diagnostics (sorted).
    pub fn diagnostic_files(&self) -> Vec<String> {
        let mut files: Vec<String> = self.file_diagnostics.keys().cloned().collect();
        files.sort();
        files
    }

    /// The graph-build diagnostics, by file then code.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut all: Vec<Diagnostic> = self.file_diagnostics.values().flatten().cloned().collect();
        all.sort_by(|a, b| {
            let file = |d: &Diagnostic| d.span.as_ref().map(|s| s.file);
            file(a).cmp(&file(b)).then_with(|| a.code.cmp(&b.code))
        });
        all
    }

    /// Every cached parse with its path, sorted by path.
    pub fn parsed_files(&self) -> Vec<(&str, &SpecFile)> {
        let mut files: Vec<(&str, &SpecFile)> = self
            .parsed_files
            .iter()
            .map(|(path, file)| (path.as_str(), file))
            .collect();
        files.sort_by(|a, b| a.0.cmp(b.0));
        files
    }

    /// Apply new texts for `changes` (`None`: the file is gone).
    pub fn rebuild(&mut self, changes: Vec<(String, Option<String>)>) -> Rebuild {
        let previous = self.verify.then(|| self.graph.clone());
        let old_file_diagnostics = std::mem::take(&mut self.file_diagnostics);
        let mut changes: BTreeMap<String, Option<String>> = changes.into_iter().collect();
        // A file that was never known and is not there now changes nothing.
        changes.retain(|path, text| text.is_some() || self.parsed_files.contains_key(path));

        // Every ID a changed file held in the graph or declares now.
        let mut ids: BTreeSet<Sym> = BTreeSet::new();
        for (path, text) in &changes {
            let file = Sym::new(path);
            ids.extend(
                self.graph
                    .nodes()
                    .iter()
                    .filter(|n| n.source_span.file == file)
                    .map(|n| n.id.raw),
            );
            match text {
                Some(text) => {
                    let spec_file = self.parse(path, text);
                    ids.extend(spec_file.entities.iter().map(|e| e.id.raw));
                    self.parsed_files.insert(path.clone(), spec_file);
                }
                None => {
                    self.trees.remove(path);
                    self.sources.remove(path);
                    self.parsed_files.remove(path);
                }
            }
        }

        // What the graph had under those IDs, and all its edges, for the delta.
        let old_nodes: BTreeMap<Sym, Node> = ids
            .iter()
            .filter_map(|id| Some((*id, self.graph.node(id.as_str())?.clone())))
            .collect();
        let old_edges = delta::edge_keys(&self.graph);

        // Red: strip the changed files' entities.
        for path in changes.keys() {
            self.graph.remove_entities_of_file(Sym::new(path));
        }
        // Green: each ID goes to its first declaration in path order (the
        // cold build's first-writer-wins rule), so a duplicate in an
        // unchanged file can win or come back.
        let mut paths: Vec<&String> = self.parsed_files.keys().collect();
        paths.sort();
        let mut placed: HashSet<Sym> = HashSet::new();
        for path in &paths {
            for entity in &self.parsed_files[*path].entities {
                if !specforge_graph::is_define_block(entity)
                    && ids.contains(&entity.id.raw)
                    && placed.insert(entity.id.raw)
                {
                    self.graph
                        .add_node(specforge_graph::node_from_entity(entity));
                }
            }
        }

        // The graph-build diagnostics, over every cached parse.
        let cached: Vec<&SpecFile> = paths.iter().map(|p| &self.parsed_files[*p]).collect();
        let mut diagnostics =
            specforge_graph::entity_pass(cached.iter().copied(), &self.graph_config, None);
        diagnostics.extend(specforge_graph::link_and_diagnose(
            &mut self.graph,
            &self.graph_config,
        ));
        self.file_diagnostics = partition_by_file(&diagnostics);

        let mut changed_diagnostic_files: Vec<String> = self
            .file_diagnostics
            .iter()
            .filter(|(file, diags)| old_file_diagnostics.get(*file) != Some(*diags))
            .map(|(file, _)| file.clone())
            .chain(
                old_file_diagnostics
                    .keys()
                    .filter(|file| !self.file_diagnostics.contains_key(*file))
                    .cloned(),
            )
            .collect();
        changed_diagnostic_files.sort();
        changed_diagnostic_files.dedup();

        let old_nodes: BTreeMap<Sym, &Node> = old_nodes.iter().map(|(id, n)| (*id, n)).collect();
        let delta = delta::diff(&ids, &old_nodes, &old_edges, &self.graph);
        let verification = previous.map(|previous| {
            let cold: Vec<SpecFile> = cached.iter().map(|f| (*f).clone()).collect();
            let (cold, _) = build_graph_with_config(&cold, &self.graph_config);
            verify(&previous, &self.graph, &cold, &delta)
        });

        Rebuild {
            delta,
            rebuilt_files: changes.into_keys().collect(),
            changed_diagnostic_files,
            verification,
        }
    }

    /// Parse `text` as `path`, reusing the retained tree.
    fn parse(&mut self, path: &str, text: &str) -> SpecFile {
        let old_tree = match (self.trees.get(path), self.sources.get(path)) {
            (Some(tree), Some(old)) if &**old != text => {
                Some(edited_tree_for_replacement(tree, old, text))
            }
            (Some(tree), Some(_)) => Some(tree.clone()),
            _ => None,
        };
        let (spec_file, tree) = parse_incremental(text, path, old_tree.as_ref());
        match tree {
            Some(tree) => self.trees.insert(path.to_string(), tree),
            None => self.trees.remove(path),
        };
        self.sources.insert(path.to_string(), Arc::from(text));
        spec_file
    }
}

/// `--verify-incremental`: the patched graph is the cold rebuild's (every
/// node's kind, file, title and fields, every edge), and the delta is the
/// one a full comparison of the two graphs gives and applies to the
/// previous graph.
fn verify(
    previous: &Graph,
    patched: &Graph,
    cold: &Graph,
    delta: &GraphDelta,
) -> Result<(), String> {
    compare_graph_contents(patched, cold)?;
    let full = delta::compute_graph_delta(previous, patched);
    if &full != delta {
        return Err(format!(
            "incremental delta differs from the full comparison: incremental {delta:?}, full {full:?}"
        ));
    }
    delta.applies(previous, patched)
}

fn compare_graph_contents(incremental: &Graph, cold: &Graph) -> Result<(), String> {
    let node_sigs = |g: &Graph| -> BTreeMap<String, String> {
        g.nodes()
            .iter()
            .map(|n| {
                let sig = format!(
                    "{} in {} title={:?} fields={}",
                    n.kind.raw,
                    n.source_span.file,
                    n.title,
                    serde_json::to_string(&n.fields).unwrap_or_default()
                );
                (n.id.raw.to_string(), sig)
            })
            .collect()
    };
    let (inc, cold_nodes) = (node_sigs(incremental), node_sigs(cold));
    if let Some(id) = inc
        .keys()
        .chain(cold_nodes.keys())
        .find(|id| inc.get(*id) != cold_nodes.get(*id))
    {
        return Err(format!(
            "incremental/cold mismatch on node '{}': incremental {:?}, cold {:?}",
            id,
            inc.get(id),
            cold_nodes.get(id)
        ));
    }
    let (inc, cold) = (delta::edge_keys(incremental), delta::edge_keys(cold));
    if let Some((s, t, l)) = inc.symmetric_difference(&cold).next() {
        let side = if inc.contains(&(*s, *t, *l)) {
            "only in incremental"
        } else {
            "only in cold"
        };
        return Err(format!(
            "incremental/cold mismatch on edge {s} -{l}-> {t} ({side})"
        ));
    }
    Ok(())
}

fn partition_by_file(diagnostics: &[Diagnostic]) -> HashMap<String, Vec<Diagnostic>> {
    let mut map: HashMap<String, Vec<Diagnostic>> = HashMap::new();
    for diag in diagnostics {
        let file = diag
            .span
            .as_ref()
            .map(|s| s.file.to_string())
            .unwrap_or_default();
        map.entry(file).or_default().push(diag.clone());
    }
    map
}

/// The [`tree_sitter::InputEdit`] for a whole-file replacement, applied to
/// a copy of `old_tree`, so `parse_incremental` can reuse unchanged
/// subtrees: the longest common byte prefix and suffix of the two texts.
fn edited_tree_for_replacement(
    old_tree: &tree_sitter::Tree,
    old_src: &str,
    new_src: &str,
) -> tree_sitter::Tree {
    let mut prefix = 0;
    let max_prefix = old_src.len().min(new_src.len());
    while prefix < max_prefix && old_src.as_bytes()[prefix] == new_src.as_bytes()[prefix] {
        prefix += 1;
    }
    while prefix > 0 && !old_src.is_char_boundary(prefix) {
        prefix -= 1;
    }

    let mut suffix = 0;
    let max_suffix = max_prefix - prefix;
    while suffix < max_suffix
        && old_src.as_bytes()[old_src.len() - 1 - suffix]
            == new_src.as_bytes()[new_src.len() - 1 - suffix]
    {
        suffix += 1;
    }
    while suffix > 0 && !new_src.is_char_boundary(new_src.len() - suffix) {
        suffix -= 1;
    }

    let start_byte = prefix;
    let old_end_byte = old_src.len() - suffix;
    let new_end_byte = new_src.len() - suffix;

    let point = |src: &str, byte: usize| {
        let before = &src[..byte];
        let row = before.matches('\n').count();
        let column = byte - before.rfind('\n').map(|i| i + 1).unwrap_or(0);
        tree_sitter::Point { row, column }
    };

    let mut tree = old_tree.clone();
    tree.edit(&tree_sitter::InputEdit {
        start_byte,
        old_end_byte,
        new_end_byte,
        start_position: point(old_src, start_byte),
        old_end_position: point(old_src, old_end_byte),
        new_end_position: point(new_src, new_end_byte),
    });
    tree
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_parser::parse;
    use specforge_test::prelude::*;

    fn cold(files: &[(&str, &str)]) -> IncrementalBuild {
        let parsed: Vec<(String, SpecFile)> = files
            .iter()
            .map(|(path, text)| (path.to_string(), parse(text, path)))
            .collect();
        let specs: Vec<SpecFile> = parsed.iter().map(|(_, f)| f.clone()).collect();
        let config = GraphConfig::default();
        let (graph, diagnostics) = build_graph_with_config(&specs, &config);
        let sources = files
            .iter()
            .map(|(path, text)| (path.to_string(), Arc::from(*text)))
            .collect();
        IncrementalBuild::from_cold_build(parsed, sources, graph, &diagnostics, config)
    }

    /// A graph that drifted from its sources outside the changed file is
    /// caught: the comparison names the node that differs.
    #[specforge_test(
        behavior = "rebuild_affected_subgraph",
        verify = "debug --verify-incremental performs cold rebuild comparison"
    )]
    fn a_divergence_from_the_cold_rebuild_is_reported() {
        let mut build = cold(&[
            ("a.spec", "behavior foo \"Foo\" { contract \"x\" }"),
            ("b.spec", "behavior other \"Other\" { contract \"o\" }"),
        ]);
        build.set_verify(true);
        let mut stale = build.graph.node("other").cloned().unwrap();
        stale.title = Some("Stale Title".to_string());
        build.graph.add_node(stale);

        let rebuild = build.rebuild(vec![(
            "a.spec".to_string(),
            Some("behavior bar \"Bar\" { contract \"new\" }".to_string()),
        )]);

        assert_eq!(rebuild.rebuilt_files, ["a.spec"], "b.spec not re-parsed");
        let err = rebuild.verification.unwrap().unwrap_err();
        assert!(
            err.contains("'other'") && err.contains("Stale Title"),
            "{err}"
        );
    }

    /// Typing reuses the retained tree; a shrinking replacement still
    /// parses as the new text alone.
    #[test]
    fn a_shrinking_replacement_parses_as_the_new_text() {
        let mut build = cold(&[]);
        build.set_verify(true);
        for text in [
            "behavior foo \"Foo\" { contract \"long text here\" }",
            "behavior foo \"Foo\" { contract \"x\" }",
            "behavior f",
            "",
            "behavior bar \"Bar\" { contract \"y\" }",
        ] {
            let rebuild = build.rebuild(vec![("a.spec".to_string(), Some(text.to_string()))]);
            assert_eq!(rebuild.verification, Some(Ok(())), "{text}");
            assert_eq!(
                build.parsed_files()[0].1.entities.len(),
                parse(text, "a.spec").entities.len(),
                "{text}"
            );
        }
    }
}
