//! The anchors read view (ADR 0349): an anchor is a graph edge between an authored entity and a located fact, either
//! way. The fixture is the item extensions (`@t/items`' collected `item`, whose `names` names entities; `@t/cites`'
//! `task`, whose `items` cites items), served in process with a `rust-source` store.

use serde_json::Value;
use specforge_ops::read::navigate::{FileMatch, anchors_of_entity, anchors_of_file};
use specforge_ops::view::ProjectView;
use specforge_project::testing::{
    ITEM_EXTENSIONS, TestProject, item, items_runtime, naming, stored_items, stored_items_with,
};
use specforge_test::prelude::*;

/// A project declaring `task alpha` (citing `cites`) and `task beta`, whose `rust-source` store holds `facts`.
fn anchored(cites: &[&str], facts: Vec<Value>) -> TestProject {
    let items: Vec<String> = cites.iter().map(|id| format!("\"{id}\"")).collect();
    let project = TestProject::new()
        .enabling(&ITEM_EXTENSIONS)
        .file(
            "main.spec",
            format!(
                "task alpha \"A\" {{\n  items [{}]\n}}\n\ntask beta \"B\" {{\n}}\n",
                items.join(", ")
            ),
        )
        .serving(items_runtime());
    stored_items(project.root(), "rust-source", facts);
    project
}

/// The public function item `id` at `line` of `file`.
fn at(id: &str, file: &str, line: usize) -> Value {
    let name = id.rsplit("::").next().unwrap();
    item(id, file, line, "function", "public", name)
}

/// `(file, line, fact_id, via)` of each row `entity` is answered.
fn rows(project: &TestProject, entity: &str) -> Vec<(String, usize, String, String)> {
    let compiled = project.compile();
    anchors_of_entity(&ProjectView::of(&compiled), entity)
        .unwrap()
        .into_value_and_notices()
        .0
        .implementations
        .into_iter()
        .map(|a| (a.file, a.line, a.fact_id, a.via))
        .collect()
}

fn row(file: &str, line: usize, fact: &str, via: &str) -> (String, usize, String, String) {
    (file.into(), line, fact.into(), via.into())
}

#[specforge_test(
    behavior = "provide_mcp_find_implementation_tool",
    verify = "an anchor is an edge either way: a fact naming the entity or the entity citing the fact"
)]
fn an_anchor_is_an_edge_either_way() {
    let project = anchored(
        &["rust:p::cited"],
        vec![
            naming(at("rust:p::naming", "src/a.rs", 3), &["alpha"]),
            at("rust:p::cited", "src/b.rs", 7),
        ],
    );
    assert_eq!(
        rows(&project, "alpha"),
        [
            row("src/a.rs", 3, "rust:p::naming", "item.names"),
            row("src/b.rs", 7, "rust:p::cited", "task.items"),
        ]
    );
    // A declared entity nothing anchors has none.
    assert_eq!(rows(&project, "beta"), []);
}

#[specforge_test(
    behavior = "provide_mcp_find_implementation_tool",
    verify = "find_implementation lists every anchor of the entity, by file and line"
)]
fn find_implementation_lists_every_anchor_by_file_and_line() {
    let project = anchored(
        &[],
        vec![
            naming(at("rust:p::z", "src/a.rs", 9), &["alpha"]),
            naming(at("rust:p::y", "src/a.rs", 2), &["alpha", "beta"]),
            naming(at("rust:p::x", "lib/c.rs", 5), &["alpha"]),
        ],
    );
    let compiled = project.compile();
    let found = anchors_of_entity(&ProjectView::of(&compiled), "alpha")
        .unwrap()
        .into_value_and_notices()
        .0;
    assert_eq!(found.count, 3);
    let first = &found.implementations[0];
    assert_eq!(
        (
            first.file.as_str(),
            first.line,
            first.end_line,
            first.fact_id.as_str(),
            first.item_kind.as_str(),
            first.collector.as_str(),
            first.via.as_str(),
        ),
        (
            "lib/c.rs",
            5,
            5,
            "rust:p::x",
            "function",
            "rust-source",
            "item.names"
        )
    );
    assert_eq!(
        rows(&project, "alpha")
            .into_iter()
            .map(|(file, line, ..)| (file, line))
            .collect::<Vec<_>>(),
        [
            ("lib/c.rs".to_string(), 5),
            ("src/a.rs".to_string(), 2),
            ("src/a.rs".to_string(), 9)
        ]
    );
    assert_eq!(rows(&project, "beta").len(), 1);
}

#[specforge_test(
    behavior = "provide_mcp_find_implementation_tool",
    verify = "an id the graph does not declare is refused with E003"
)]
fn an_id_the_graph_does_not_declare_is_e003() {
    let project = anchored(
        &[],
        vec![naming(at("rust:p::a", "src/a.rs", 1), &["alpha"])],
    );
    let compiled = project.compile();
    let view = ProjectView::of(&compiled);
    let refusal = anchors_of_entity(&view, "alpah").unwrap_err();
    assert_eq!(
        refusal.subject,
        Some(specforge_ops::view::EntityField::EntityId)
    );
    assert_eq!(refusal.error.code, "E003");
    assert_eq!(
        refusal.error.suggestion.as_deref(),
        Some("did you mean 'alpha'?")
    );
    // A fact is no authored entity: its anchors are its file's.
    let fact = anchors_of_entity(&view, "rust:p::a").unwrap_err();
    assert_eq!(fact.error.kind, specforge_ops::OpErrorKind::InvalidInput);
}

#[specforge_test(
    behavior = "provide_mcp_find_spec_for_source_tool",
    verify = "find_spec_for_source lists the anchors of every located fact the path names"
)]
fn find_spec_for_source_lists_the_anchors_of_the_located_facts_of_the_path() {
    let project = anchored(
        &["rust:p::cited"],
        vec![
            naming(at("rust:p::naming", "src/a.rs", 3), &["alpha", "beta"]),
            at("rust:p::cited", "src/a.rs", 1),
            naming(at("rust:p::elsewhere", "src/b.rs", 1), &["beta"]),
            at("rust:p::lone", "src/a.rs", 5),
        ],
    );
    let compiled = project.compile();
    let found = anchors_of_file(&ProjectView::of(&compiled), "src/a.rs")
        .unwrap()
        .into_value_and_notices()
        .0;
    let entities: Vec<(&str, &str, &str, &str)> = found
        .entities
        .iter()
        .map(|e| {
            (
                e.entity_id.as_str(),
                e.kind.as_str(),
                e.fact_id.as_str(),
                e.via.as_str(),
            )
        })
        .collect();
    assert_eq!(
        entities,
        [
            ("alpha", "task", "rust:p::cited", "task.items"),
            ("alpha", "task", "rust:p::naming", "item.names"),
            ("beta", "task", "rust:p::naming", "item.names"),
        ]
    );
    assert_eq!((found.match_mode, found.count), (FileMatch::Exact, 3));
}

#[specforge_test(
    behavior = "provide_mcp_find_spec_for_source_tool",
    verify = "the tightest match wins: exact, else under the directory, else ending with the path"
)]
fn the_tightest_match_wins() {
    let project = anchored(
        &[],
        vec![
            naming(at("rust:p::top", "a.rs", 1), &["alpha"]),
            naming(at("rust:p::nested", "sub/a.rs", 1), &["alpha"]),
            naming(at("rust:p::other", "sub/b.rs", 1), &["beta"]),
            naming(at("rust:p::cache", "src/cache.rs", 1), &["beta"]),
        ],
    );
    let compiled = project.compile();
    let found = |query: &str| {
        let found = anchors_of_file(&ProjectView::of(&compiled), query)
            .unwrap()
            .into_value_and_notices()
            .0;
        let ids: Vec<String> = found.entities.iter().map(|e| e.fact_id.clone()).collect();
        (found.match_mode, ids)
    };
    assert_eq!(
        found("a.rs"),
        (FileMatch::Exact, vec!["rust:p::top".into()])
    );
    for spelling in ["./a.rs", "a.rs", ".\\a.rs"] {
        assert_eq!(found(spelling).1, ["rust:p::top"], "{spelling}");
    }
    assert_eq!(
        found("./sub"),
        (
            FileMatch::Directory,
            vec!["rust:p::nested".into(), "rust:p::other".into()]
        )
    );
    assert_eq!(
        found("b.rs"),
        (FileMatch::SuffixPath, vec!["rust:p::other".into()])
    );
    // A substring is no match.
    assert_eq!(found("e.rs").0, FileMatch::None);
    assert_eq!(found("c.rs").0, FileMatch::None);
}

/// The codes `entity`'s answer notices.
fn notices(project: &TestProject, entity: &str) -> Vec<String> {
    let compiled = project.compile();
    let (_, notices) = anchors_of_entity(&ProjectView::of(&compiled), entity)
        .unwrap()
        .into_value_and_notices();
    notices.into_iter().map(|n| n.code).collect()
}

#[specforge_test(
    behavior = "provide_mcp_find_implementation_tool",
    verify = "an answer with no located fact served carries I212"
)]
fn nothing_collected_is_i212() {
    let project = anchored(&[], vec![]);
    assert_eq!(rows(&project, "alpha"), []);
    assert_eq!(notices(&project, "alpha"), ["I212"]);
    let compiled = project.compile();
    let (found, said) = anchors_of_file(&ProjectView::of(&compiled), "src/a.rs")
        .unwrap()
        .into_value_and_notices();
    assert_eq!(found.count, 0);
    assert_eq!(said.len(), 1);
    assert_eq!(said[0].code, "I212");
    assert!(
        said[0]
            .suggestion
            .as_deref()
            .is_some_and(|s| s.contains("specforge collect")),
        "{said:?}"
    );
    // A located fact served, even one nothing anchors, is something to anchor to.
    let lone = anchored(&[], vec![at("rust:p::lone", "src/a.rs", 1)]);
    assert!(!notices_of_file(&lone).contains(&"I212".to_string()));
}

/// The codes the answer for `src/a.rs` notices.
fn notices_of_file(project: &TestProject) -> Vec<String> {
    let compiled = project.compile();
    let (_, notices) = anchors_of_file(&ProjectView::of(&compiled), "src/a.rs")
        .unwrap()
        .into_value_and_notices();
    notices.into_iter().map(|n| n.code).collect()
}

#[specforge_test(
    behavior = "provide_mcp_find_implementation_tool",
    verify = "a source file edited since collect gives one W198 and keeps its rows"
)]
fn an_edited_source_file_is_one_w198() {
    let project = anchored(&[], vec![]);
    let root = project.root();
    let mut sources = std::collections::BTreeMap::new();
    for (file, text) in [("src/a.rs", "fn a() {}\n"), ("src/b.rs", "fn b() {}\n")] {
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join(file), text).unwrap();
        sources.insert(
            file.to_string(),
            specforge_ops::revision::of_bytes(text.as_bytes()),
        );
    }
    stored_items_with(
        root,
        "rust-source",
        vec![
            naming(at("rust:p::a", "src/a.rs", 1), &["alpha"]),
            naming(at("rust:p::a2", "src/a.rs", 3), &["alpha"]),
            naming(at("rust:p::b", "src/b.rs", 1), &["alpha"]),
        ],
        sources,
    );
    // As collected: no notice.
    assert_eq!(notices(&project, "alpha"), Vec::<String>::new());

    std::fs::write(root.join("src/a.rs"), "fn a() { 1 }\n").unwrap();
    let compiled = project.compile();
    let (found, said) = anchors_of_entity(&ProjectView::of(&compiled), "alpha")
        .unwrap()
        .into_value_and_notices();
    assert_eq!(found.count, 3, "the stale file's rows are still listed");
    assert_eq!(said.len(), 1, "{said:?}");
    assert_eq!(said[0].code, "W198");
    assert!(said[0].message.contains("'src/a.rs'"), "{said:?}");

    // A file gone since the collect is stale too.
    std::fs::remove_file(root.join("src/b.rs")).unwrap();
    assert_eq!(notices(&project, "alpha"), ["W198", "W198"]);
}

#[specforge_test(
    behavior = "provide_mcp_find_implementation_tool",
    verify = "two stores placing one file are each judged by their own digest"
)]
fn two_stores_placing_one_file_are_each_judged_by_their_own_digest() {
    let project = anchored(&[], vec![]);
    let root = project.root();
    std::fs::create_dir_all(root.join("src")).unwrap();
    let (then, now) = ("fn a() {}\n", "fn a() { 1 }\n");
    std::fs::write(root.join("src/a.rs"), now).unwrap();
    let digest = |text: &str| specforge_ops::revision::of_bytes(text.as_bytes());
    // `rust-source` collected the file as it is now, `rust-older` before the edit.
    for (collector, id, entity, text) in [
        ("rust-source", "rust:p::a", "alpha", now),
        ("rust-older", "rust:p::b", "beta", then),
    ] {
        stored_items_with(
            root,
            collector,
            vec![naming(at(id, "src/a.rs", 1), &[entity])],
            [("src/a.rs".to_string(), digest(text))].into(),
        );
    }

    assert_eq!(notices(&project, "alpha"), Vec::<String>::new());
    assert_eq!(notices(&project, "beta"), ["W198"]);
    assert_eq!(notices_of_file(&project), ["W198"]);
}

#[test]
fn an_edge_between_two_facts_is_no_anchor() {
    let project = anchored(
        &[],
        vec![
            naming(at("rust:p::a", "src/a.rs", 1), &["rust:p::b"]),
            at("rust:p::b", "src/a.rs", 2),
        ],
    );
    let compiled = project.compile();
    let found = anchors_of_file(&ProjectView::of(&compiled), "src/a.rs")
        .unwrap()
        .into_value_and_notices()
        .0;
    assert_eq!(found.count, 0, "{found:?}");
}

#[test]
fn a_rootless_view_is_no_project() {
    use crate::view_support::{Assembled, Viewed};
    let project = Assembled::over(specforge_registry::RegistryBuild::default())
        .source("behavior alpha \"A\" {\n}\n");
    let file = anchors_of_file(&project.rootless_view(), "src/a.rs").unwrap_err();
    assert_eq!(
        file.error.kind,
        specforge_ops::OpErrorKind::PreconditionFailed
    );
    assert_eq!(file.error.code, "no_project");
    assert_eq!(file.subject, None);
}
