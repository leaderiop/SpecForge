//! The extensions the inference tests share: source items as a source pack collects them (ADR 0199), served in
//! process and written as the wire carries them.
//!
//! - [`ITEMS`] (`@t/items`): the kind `item`, written by a collector under the schemes `rust` and `js`, shaped as
//!   `@code/core`'s: `kind` (an enum list), `visibility` (`public`, `crate`, `restricted`, `private`) and `names` (the
//!   entities it says it implements, edge `names`)
//! - [`CITES`] (`@t/cites`): the kind `task`, whose `items` lists the items it touches
//!
//! A test writes the facts a collect would leave with [`stored_items`], each spelled by [`item`].

use serde_json::{Value, json};
use specforge_extension_sdk::Vocabulary;
use specforge_extension_sdk::prelude::{ContributionsBuilder, ExtensionMeta, FieldType};
use specforge_wasm::testing::InProcessRuntime;

/// The extension that declares the collected kind `item`.
pub const ITEMS: &str = "@t/items";
/// The extension whose `task` cites items.
pub const CITES: &str = "@t/cites";

/// The two extensions' names, as a project enables them.
pub const ITEM_EXTENSIONS: [&str; 2] = [ITEMS, CITES];

/// The key [`stored_items`] files a store under: a project in no repository.
pub const WORKTREE: &str = "worktree";

/// What a definition is, as `@code/core` names it (a test holds the copy to `@code/core`'s pinned declaration).
const KINDS: Vocabulary = Vocabulary::new(&[
    "module",
    "function",
    "method",
    "field",
    "struct",
    "enum",
    "union",
    "trait",
    "type",
    "const",
    "static",
    "macro",
    "class",
    "interface",
    "variable",
    "namespace",
]);

/// Who may name a definition, widest first, as `@code/core` names it (held to it as [`KINDS`] is).
const VISIBILITY: Vocabulary = Vocabulary::new(&["public", "crate", "restricted", "private"]);

/// Declare [`ITEMS`]'s kind `item` in `c`: what a test serving its own builders (MCP's `TestExtension`) declares.
pub fn declare_items(c: &mut ContributionsBuilder) {
    c.kind("item", |k| {
        k.description("A definition a source pack read")
            .collected(&["rust", "js"])
            .field("kind", |f| {
                f.enum_list(&KINDS);
            })
            .field("visibility", |f| {
                f.vocabulary(&VISIBILITY);
            })
            .field("names", |f| {
                f.field_type(FieldType::ReferenceList).edge("names");
            });
    });
    c.edge("names", |_| {});
}

/// Declare [`CITES`]'s kind `task` in `c`, whose `items` lists items (edge `items`).
pub fn declare_cites(c: &mut ContributionsBuilder) {
    c.kind("task", |k| {
        k.description("A task that touches items")
            .field("items", |f| {
                f.field_type(FieldType::ReferenceList)
                    .target_kind("item")
                    .edge("items");
            });
    });
    c.edge("items", |_| {});
}

/// The two extensions, served in process.
pub fn items_runtime() -> InProcessRuntime {
    items_over(InProcessRuntime::new())
}

/// `runtime` serving the two extensions too.
pub fn items_over(runtime: InProcessRuntime) -> InProcessRuntime {
    runtime
        .with(|| {
            let mut c = ContributionsBuilder::new(ExtensionMeta::new(ITEMS, "1.0.0"));
            declare_items(&mut c);
            c
        })
        .with(|| {
            let mut c = ContributionsBuilder::new(ExtensionMeta::new(CITES, "1.0.0"));
            declare_cites(&mut c);
            c
        })
}

/// The `item` fact `id` titled `title`, of the definition kind `kind`, at `visibility`, written at `line` of `file`,
/// as the wire carries it.
pub fn item(id: &str, file: &str, line: usize, kind: &str, visibility: &str, title: &str) -> Value {
    json!({
        "kind": "item",
        "id": id,
        "title": title,
        "fields": {"kind": [kind], "visibility": visibility},
        "at": [{"file": file, "start_line": line, "start_col": 1, "end_line": line, "end_col": 2}],
    })
}

/// [`ITEMS`]' and [`CITES`]' declarations, as a compile loads them: what an in-memory project
/// (`Assembled::declarations`) holding [`item_node`]s declares.
pub fn item_declarations() -> Vec<specforge_protocol_types::ExtensionDeclaration> {
    let runtime = items_runtime();
    ITEM_EXTENSIONS
        .iter()
        .map(|name| {
            specforge_wasm::protocol::load_declaration(&runtime, name)
                .expect("the item fixtures declare")
                .declaration
        })
        .collect()
}

/// The public `function` item fact `id`, titled by its last name, written at `line` of `file`, as a graph holds it
/// served from the `rust-source` store under [`WORKTREE`].
pub fn item_node(id: &str, file: &str, line: usize) -> specforge_graph::Node {
    use specforge_common::{SourceSpan, Sym};
    use specforge_parser::{FieldValue, ListItem, Written};
    let span = |file: &str, line: usize| SourceSpan {
        file: Sym::new(file),
        start_line: line,
        start_col: 1,
        end_line: line,
        end_col: 2,
    };
    let store = crate::facts::Store::path("rust-source", WORKTREE);
    let mut node = specforge_graph::Node::new(id, "item", span(&store, 1));
    node.title = Some(id.rsplit("::").next().unwrap_or(id).to_string());
    node.located = vec![span(file, line)];
    node.fields.push(
        Sym::new("kind"),
        FieldValue::List(vec![ListItem {
            text: "function".into(),
            span: span(&store, 1),
            written: Written::Bare,
        }]),
    );
    node.fields.push(
        Sym::new("visibility"),
        FieldValue::Identifier("public".into()),
    );
    node
}

/// [`item`], naming `entities` in its `names`.
pub fn naming(mut item: Value, entities: &[&str]) -> Value {
    item["fields"]["names"] = json!(entities);
    item
}

/// Write `collector`'s store of `facts` (each an [`item`]) under [`WORKTREE`] in the project at `root`, and the
/// `specforge-harness` line that records it, as a collect leaves them.
pub fn stored_items(root: &std::path::Path, collector: &str, facts: Vec<Value>) {
    super::collected::store_of(
        root,
        collector,
        ITEMS,
        WORKTREE,
        Value::Array(facts),
        Default::default(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixture_serves_a_collected_item_and_a_citing_task() {
        let runtime = items_runtime();
        let items = specforge_wasm::protocol::load_declaration(&runtime, ITEMS)
            .unwrap()
            .declaration;
        let cites = specforge_wasm::protocol::load_declaration(&runtime, CITES)
            .unwrap()
            .declaration;
        let item = &items.entities[0];
        assert_eq!(
            (item.writer.as_deref(), &item.schemes[..]),
            (
                Some("collector"),
                &["rust".to_string(), "js".to_string()][..]
            )
        );
        assert_eq!(
            cites.entities[0].fields[0].target_kind.as_deref(),
            Some("item")
        );
    }

    /// The item's `kind` and `visibility` (type and values) as `declaration` declares them.
    fn shaped(
        declaration: &specforge_protocol_types::ExtensionDeclaration,
    ) -> Vec<(String, Vec<String>)> {
        let item = declaration
            .entities
            .iter()
            .find(|kind| kind.name == "item")
            .expect("an item kind");
        ["kind", "visibility"]
            .iter()
            .map(|name| {
                let field = item.fields.iter().find(|f| f.name == *name).expect(name);
                (field.field_type.clone(), field.enum_values.clone())
            })
            .collect()
    }

    #[test]
    fn the_fixture_item_is_shaped_as_code_cores() {
        let runtime = items_runtime();
        let fixture = specforge_wasm::protocol::load_declaration(&runtime, ITEMS)
            .unwrap()
            .declaration;
        let code_core = specforge_component::committed::pinned_builtin_declarations()
            .unwrap()
            .into_iter()
            .find(|declaration| declaration.name() == "@code/core")
            .expect("@code/core is a builtin");
        let declared = shaped(&code_core);
        assert!(
            declared.iter().all(|(_, values)| !values.is_empty()),
            "{declared:?}"
        );
        assert_eq!(shaped(&fixture), declared);
    }
}
