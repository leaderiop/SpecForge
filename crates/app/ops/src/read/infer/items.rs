//! The item facts inference reads (ADR 0199): the definitions a source pack collected into the compiled graph, as
//! `@code/core`'s kind `item` holds them. Inference is the host's code view: it names that one kind here and
//! nowhere else, and reads it only where a collector writes it, so an authored kind of that name never counts.

use std::path::Path;

use specforge_graph::Node;
use specforge_parser::FieldValue;

use crate::source_key::SourceKey;
use crate::view::ProjectView;

/// The collected kind inference reads: `@code/core`'s definitions.
pub const ITEM_KIND: &str = "item";

/// The field naming who may name an item.
const VISIBILITY: &str = "visibility";

/// The `visibility` of an item anything outside its package may name.
const PUBLIC: &str = "public";

/// The field listing what an item's definitions are.
const KIND: &str = "kind";

/// An item fact's own name: its title (a pack titles an item by its name), else its id.
pub(crate) fn item_name(node: &Node) -> &str {
    node.title.as_deref().unwrap_or(node.id.raw.as_str())
}

/// One item fact, where its source places it.
#[derive(Debug, Clone)]
pub(crate) struct ItemFact<'g> {
    pub(crate) node: &'g Node,
    /// The file of its first place, as a key from the project root.
    pub(crate) key: SourceKey,
    /// The first line of its first place.
    pub(crate) line: usize,
    /// The collector whose store holds it; `None` when it is in no fact store.
    pub(crate) collector: Option<String>,
}

impl ItemFact<'_> {
    /// Whether anything outside its package may name it.
    pub(crate) fn is_public(&self) -> bool {
        matches!(
            self.node.fields.get(VISIBILITY),
            Some(FieldValue::Identifier(word) | FieldValue::String(word)) if word == PUBLIC
        )
    }

    /// Its own name ([`item_name`]).
    pub(crate) fn name(&self) -> &str {
        item_name(self.node)
    }

    /// What its definitions are (`function`, `struct, module`), in the order its `kind` lists them.
    pub(crate) fn kinds(&self) -> String {
        match self.node.fields.get(KIND) {
            Some(FieldValue::List(items)) => items
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            Some(FieldValue::Identifier(word) | FieldValue::String(word)) => word.clone(),
            _ => String::new(),
        }
    }
}

/// The item facts of the view's graph whose first place is a file under `roots` (the project when there are none),
/// read from the project `root`, in graph order. An item with no place, or whose file lies outside the root, is
/// none; so is any node when no loaded extension declares a collector writes [`ITEM_KIND`].
pub(crate) fn item_facts<'g>(
    view: &ProjectView<'g>,
    root: &Path,
    roots: &[String],
) -> Vec<ItemFact<'g>> {
    if !view.registries().kinds().is_collected(ITEM_KIND) {
        return Vec::new();
    }
    let roots: Vec<SourceKey> = if roots.is_empty() {
        SourceKey::read_root(root, ".").into_iter().collect()
    } else {
        roots
            .iter()
            .filter_map(|named| SourceKey::read_root(root, named).ok())
            .collect()
    };
    view.graph()
        .nodes_by_kind(ITEM_KIND)
        .into_iter()
        .filter_map(|node| {
            let place = node.located.first()?;
            let key = SourceKey::read(root, place.file.as_str()).ok()?;
            under(&key, &roots).then(|| ItemFact {
                node,
                key,
                line: place.start_line,
                collector: specforge_common::facts::filed(node.source_span.file.as_str())
                    .map(|(collector, _)| collector.to_string()),
            })
        })
        .collect()
}

/// Whether `key` is one of `roots` or lies under one.
fn under(key: &SourceKey, roots: &[SourceKey]) -> bool {
    roots.iter().any(|root| {
        root.as_str() == "."
            || key.as_str() == root.as_str()
            || key
                .as_str()
                .strip_prefix(root.as_str())
                .is_some_and(|rest| rest.starts_with('/'))
    })
}
