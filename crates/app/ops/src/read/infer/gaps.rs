//! Inference gaps: the public items the compiled graph's item facts hold that nothing covers (`specforge
//! infer-status --gaps-detail`, the MCP `specforge.infer_gaps` tool).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use specforge_common::codes;
use specforge_common::shape::Shape;

use super::items::item_facts;
use super::manifest::InferenceManifest;
use crate::OpError;
use crate::view::ProjectView;

/// What both surfaces say when no item fact locates a source file: no source pack collected any, public or not.
pub const NO_ITEM_FACTS: &str =
    "No item facts locate a source file: load a source pack and run `specforge collect`";

/// Public source items nothing covers, by directory.
#[derive(Debug, Clone)]
pub struct Gaps {
    /// The item facts under the source roots, public or not: none means no source pack collected any
    /// ([`NO_ITEM_FACTS`]).
    pub item_facts: usize,
    pub total_pub_items: usize,
    pub covered_items: usize,
    /// Directory (`.` for the root) to its uncovered items, sorted.
    pub by_directory: BTreeMap<String, Vec<SourceItem>>,
    /// Whether the counts are approximate: a fact store was not served (W191, W192), so its items are unknown. It
    /// over-reports when that store holds no item.
    pub approximate: bool,
    /// The collectors whose stores held the counted items, sorted, each once.
    pub collectors: Vec<String>,
}

/// One public item, where its source places it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceItem {
    pub name: String,
    pub item_kind: String,
    pub file: String,
    pub line: usize,
    /// The collector whose store holds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collector: Option<String>,
}

/// The public item facts of the view's graph under the inference manifest's source roots that nothing covers: no
/// authored entity cites them and no anchor names them (the anchors read view's covered items, ADR 0199). No source
/// file is read. Without a root: `no_project`; an inference or anchors manifest that cannot be used: E071.
pub fn gaps(view: &ProjectView) -> Result<Gaps, OpError> {
    let root = view.project_root()?;
    let manifest = InferenceManifest::at(root)?;
    // Before any fact is read: an anchors manifest that cannot be used refuses the gaps.
    let covered = crate::read::navigate::CoveredItems::of(view)?;
    let facts = item_facts(view, root, &manifest.source_roots);
    let served = facts.len();
    let items = facts
        .into_iter()
        .filter(|item| item.is_public())
        .map(|item| {
            let covers = covered.covers(item.node);
            let item = SourceItem {
                name: item.name().to_string(),
                item_kind: item.kinds(),
                file: item.key.as_str().to_string(),
                line: item.line,
                collector: item.collector,
            };
            (item, covers)
        });
    let approximate = view
        .reported()
        .iter()
        .any(|d| d.is(codes::W191) || d.is(codes::W192));
    Ok(Gaps {
        item_facts: served,
        ..Gaps::of(items, approximate)
    })
}

/// The directory part of a project-relative path, `.` at the root.
pub fn directory_of(path: &str) -> &str {
    path.rfind('/').map_or(".", |i| &path[..i])
}

impl Gaps {
    /// The public `items`, each with whether it is covered: the uncovered ones by directory, the collectors of all.
    /// `approximate` when a fact store was not served. The item facts are the public ones: [`gaps`] counts the rest.
    pub fn of(items: impl IntoIterator<Item = (SourceItem, bool)>, approximate: bool) -> Gaps {
        let mut total_pub_items = 0;
        let mut covered_items = 0;
        let mut collectors = BTreeSet::new();
        let mut by_directory: BTreeMap<String, Vec<SourceItem>> = BTreeMap::new();
        for (item, covered) in items {
            total_pub_items += 1;
            collectors.extend(item.collector.clone());
            if covered {
                covered_items += 1;
                continue;
            }
            by_directory
                .entry(directory_of(&item.file).to_string())
                .or_default()
                .push(item);
        }
        Gaps {
            item_facts: total_pub_items,
            total_pub_items,
            covered_items,
            by_directory,
            approximate,
            collectors: collectors.into_iter().collect(),
        }
    }

    /// How many items nothing covers.
    pub fn gap_count(&self) -> usize {
        self.by_directory.values().map(Vec::len).sum()
    }

    /// The document both surfaces answer with (the spec's
    /// `InferenceGapReport`): totals, then each directory's items.
    pub fn document(&self) -> GapsDocument {
        GapsDocument {
            item_facts: self.item_facts,
            total_pub_items: self.total_pub_items,
            covered_items: self.covered_items,
            gap_count: self.gap_count(),
            approximate: self.approximate,
            collectors: self.collectors.clone(),
            by_directory: self
                .by_directory
                .iter()
                .map(|(dir, gaps)| DirectoryGaps {
                    directory: dir.clone(),
                    count: gaps.len(),
                    items: gaps
                        .iter()
                        .map(|g| GapItem {
                            name: g.name.clone(),
                            item_kind: g.item_kind.clone(),
                            file: g.file.clone(),
                            line: g.line,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

/// What inference gaps answer with, on both surfaces (the spec's
/// `InferenceGapReport`).
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct GapsDocument {
    /// The item facts under the source roots, public or not; none when no source pack collected any.
    pub item_facts: usize,
    pub total_pub_items: usize,
    pub covered_items: usize,
    pub gap_count: usize,
    pub approximate: bool,
    pub collectors: Vec<String>,
    pub by_directory: Vec<DirectoryGaps>,
}

/// The gaps of one directory.
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct DirectoryGaps {
    pub directory: String,
    pub count: usize,
    pub items: Vec<GapItem>,
}

/// One public item nothing covers.
#[derive(Debug, Clone, PartialEq, Serialize, Shape)]
pub struct GapItem {
    pub name: String,
    pub item_kind: String,
    pub file: String,
    pub line: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::ProjectView;
    use specforge_project::testing::{TestProject, item, items_runtime, naming, stored_items};
    use specforge_test_macros::test as specforge_test;

    fn item_at(name: &str, kind: &str, file: &str, collector: Option<&str>) -> SourceItem {
        SourceItem {
            name: name.into(),
            item_kind: kind.into(),
            file: file.into(),
            line: 1,
            collector: collector.map(str::to_string),
        }
    }

    #[test]
    fn gaps_keep_the_items_nothing_covers() {
        let gaps = Gaps::of(
            [
                (
                    item_at("hello", "function", "src/lib.rs", Some("rust-source")),
                    true,
                ),
                (
                    item_at("config", "struct", "src/lib.rs", Some("rust-source")),
                    false,
                ),
            ],
            false,
        );
        assert_eq!(gaps.total_pub_items, 2);
        assert_eq!(gaps.covered_items, 1);
        assert_eq!(gaps.by_directory["src"].len(), 1);
        assert_eq!(gaps.by_directory["src"][0].name, "config");
        assert!(!gaps.approximate);
        assert_eq!(gaps.collectors, ["rust-source"]);
    }

    #[test]
    fn gaps_of_covered_items_are_empty() {
        let gaps = Gaps::of([(item_at("hello", "function", "a.rs", None), true)], false);
        assert_eq!(gaps.gap_count(), 0);
        assert_eq!(gaps.covered_items, 1);
        assert!(gaps.collectors.is_empty());
    }

    #[test]
    fn gaps_of_no_items_count_nothing() {
        let gaps = Gaps::of([], true);
        assert_eq!((gaps.total_pub_items, gaps.gap_count()), (0, 0));
        assert!(gaps.approximate);
    }

    #[test]
    fn the_collectors_are_sorted_and_named_once() {
        let gaps = Gaps::of(
            [
                (item_at("a", "function", "a.rs", Some("rust-source")), false),
                (
                    item_at("b", "function", "b.ts", Some("javascript-source")),
                    true,
                ),
                (item_at("c", "function", "c.rs", Some("rust-source")), false),
            ],
            false,
        );
        assert_eq!(gaps.collectors, ["javascript-source", "rust-source"]);
    }

    #[test]
    fn gaps_document_groups_items_by_directory() {
        let gaps = Gaps::of(
            [
                (item_at("main", "fn", "main.rs", None), false),
                (item_at("alpha", "fn", "src/a.rs", None), true),
                (item_at("beta", "fn", "src/b.rs", None), false),
            ],
            true,
        );
        let doc = serde_json::to_value(gaps.document()).unwrap();
        assert_eq!(doc["gap_count"], 2);
        assert_eq!(doc["by_directory"][1]["directory"], "src");
        assert_eq!(doc["by_directory"][1]["items"][0]["file"], "src/b.rs");
        assert_eq!(directory_of("main.rs"), ".");
    }

    /// Public fns, one only a raw string spells.
    const STRING_SPELLED: &str = "pub fn real_one() {}\npub(crate) fn crate_visible() {}\n\
         pub const SNIPPET: &str = r#\"\npub fn ghost_in_a_string() {}\n\"#;\n";

    /// A project serving the item fixtures whose `rust-source` store holds `real_one` (public) and
    /// `crate_visible` (crate), with `src/lib.rs` = [`STRING_SPELLED`] and a `task real_one` that cites nothing.
    fn collected() -> TestProject {
        let project = TestProject::new()
            .enabling(&specforge_project::testing::ITEM_EXTENSIONS)
            .file("main.spec", "task real_one \"R\" {\n}\n")
            .file("src/lib.rs", STRING_SPELLED)
            .serving(items_runtime());
        stored_items(
            project.root(),
            "rust-source",
            vec![
                item(
                    "rust:p::real_one",
                    "src/lib.rs",
                    1,
                    "function",
                    "public",
                    "real_one",
                ),
                item(
                    "rust:p::crate_visible",
                    "src/lib.rs",
                    2,
                    "function",
                    "crate",
                    "crate_visible",
                ),
            ],
        );
        project
    }

    fn gaps_of(project: &TestProject) -> Gaps {
        let compiled = project.compile();
        gaps(&ProjectView::of(&compiled)).unwrap()
    }

    fn gap_names(gaps: &Gaps) -> Vec<&str> {
        gaps.by_directory
            .values()
            .flatten()
            .map(|item| item.name.as_str())
            .collect()
    }

    #[specforge_test(
        behavior = "provide_mcp_infer_gaps_tool",
        verify = "gaps never read source text: a public item only a string literal spells is no gap"
    )]
    fn gaps_never_read_source_text() {
        let gaps = gaps_of(&collected());
        assert_eq!(gap_names(&gaps), ["real_one"], "{gaps:?}");
        assert_eq!(gaps.total_pub_items, 1, "{gaps:?}");
        assert_eq!(gaps.collectors, ["rust-source"]);
        assert_eq!(gaps.by_directory["src"][0].line, 1);
        assert_eq!(gaps.by_directory["src"][0].item_kind, "function");
    }

    #[specforge_test(
        behavior = "provide_mcp_infer_gaps_tool",
        verify = "an entity that only shares an item's name does not cover it"
    )]
    fn a_same_named_entity_covers_nothing() {
        let gaps = gaps_of(&collected());
        assert_eq!(gaps.covered_items, 0, "{gaps:?}");
        assert!(gap_names(&gaps).contains(&"real_one"), "{gaps:?}");
    }

    #[test]
    fn only_private_items_are_item_facts_with_no_public_one() {
        let project = collected();
        stored_items(
            project.root(),
            "rust-source",
            vec![item(
                "rust:p::crate_visible",
                "src/lib.rs",
                2,
                "function",
                "crate",
                "crate_visible",
            )],
        );
        let gaps = gaps_of(&project);
        assert_eq!((gaps.item_facts, gaps.total_pub_items), (1, 0), "{gaps:?}");
        assert!(gaps.collectors.is_empty(), "{gaps:?}");
    }

    #[test]
    fn an_item_that_names_a_task_is_covered() {
        let project = collected();
        stored_items(
            project.root(),
            "rust-source",
            vec![naming(
                item(
                    "rust:p::real_one",
                    "src/lib.rs",
                    1,
                    "function",
                    "public",
                    "real_one",
                ),
                &["real_one"],
            )],
        );
        let gaps = gaps_of(&project);
        assert_eq!((gaps.covered_items, gaps.gap_count()), (1, 0), "{gaps:?}");
    }
}
