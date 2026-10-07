//! The incremental rebuild a [`crate::ProjectSession`] runs on each change.
//!
//! A parse depends only on its own file's text, and references resolve
//! across the whole project without `use` (ADR 0004 D1-a), so a change
//! re-parses exactly the files that changed: an importer of a changed
//! file parses the same as before. This module keeps the texts and the
//! retained trees and parses; the graph and its diagnostics are the
//! session's [`GraphBuild`], which takes each re-parsed file whole (ADR
//! 0032).

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use specforge_common::Diagnostic;
use specforge_graph::{FileChange, Graph, GraphBuild, GraphConfig, GraphDelta};
use specforge_parser::{SpecFile, parse_incremental};

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

/// The texts and retained trees of a project's sources, and their graph
/// build.
pub(crate) struct IncrementalBuild {
    /// The cached parses, their graph and the graph-build diagnostics.
    build: GraphBuild,
    /// The text each cached parse was made from: what a reader resolves a
    /// span of the graph against (a span is a position in this text), and
    /// what a whole-file replacement is diffed against to edit the
    /// retained tree.
    sources: HashMap<String, Arc<str>>,
    /// Retained tree-sitter trees, fed back into `parse_incremental` so
    /// keystroke-sized edits reuse unchanged subtrees.
    trees: HashMap<String, tree_sitter::Tree>,
}

impl IncrementalBuild {
    /// Seeded by a cold `build` of files parsed from `sources`.
    pub fn from_cold_build(sources: HashMap<String, Arc<str>>, build: GraphBuild) -> Self {
        IncrementalBuild {
            build,
            sources,
            trees: HashMap::new(),
        }
    }

    pub fn empty() -> Self {
        Self::from_cold_build(HashMap::new(), GraphBuild::new(GraphConfig::default()))
    }

    /// Compare every rebuild with a cold one (costly).
    pub fn set_verify(&mut self, enabled: bool) {
        self.build.set_verify(enabled);
    }

    pub fn graph(&self) -> &Graph {
        self.build.graph()
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
        self.build.file_diagnostics(path)
    }

    /// Every file that has graph-build diagnostics (sorted).
    pub fn diagnostic_files(&self) -> Vec<String> {
        self.build.diagnostic_files()
    }

    /// The graph-build diagnostics, in build order.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.build.diagnostics().to_vec()
    }

    /// Every cached parse with its path, sorted by path.
    pub fn parsed_files(&self) -> Vec<(&str, &SpecFile)> {
        self.build.files().collect()
    }

    /// Apply new texts for `changes` (`None`: the file is gone).
    pub fn rebuild(&mut self, changes: Vec<(String, Option<String>)>) -> Rebuild {
        let changes: BTreeMap<String, Option<String>> = changes.into_iter().collect();
        let changes: Vec<FileChange> = changes
            .into_iter()
            .map(|(path, text)| match text {
                Some(text) => FileChange::Parsed(self.parse(&path, &text)),
                None => {
                    self.trees.remove(&path);
                    self.sources.remove(&path);
                    FileChange::Removed(path)
                }
            })
            .collect();
        let applied = self.build.apply(changes);
        Rebuild {
            delta: applied.delta,
            rebuilt_files: applied.files,
            changed_diagnostic_files: applied.changed_diagnostic_files,
            verification: applied.verification,
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

    fn cold(files: &[(&str, &str)]) -> IncrementalBuild {
        let build = GraphBuild::of(
            files.iter().map(|(path, text)| parse(text, path)),
            GraphConfig::default(),
        );
        let sources = files
            .iter()
            .map(|(path, text)| (path.to_string(), Arc::from(*text)))
            .collect();
        IncrementalBuild::from_cold_build(sources, build)
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
