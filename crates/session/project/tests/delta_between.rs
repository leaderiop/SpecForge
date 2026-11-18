//! The delta between two graphs (ADR 0376 D5, D6): each changed node by kind and id with the digests of its content,
//! sorted as bytes, a kind change a removal and an addition, a layout-only edit no change, and filters by kind and by
//! declaring file that need no graph.

use specforge_graph::Graph;
use specforge_project::CompiledProject;
use specforge_project::delta::Delta;
use specforge_project::testing::TestProject;
use specforge_protocol_types::{DeltaAspect, DeltaBase, DeltaChange, DeltaEdge, DeltaNode};
use specforge_test::prelude::*;

/// `files` compiled with no extension: generic nodes whose kinds are their keywords, `refs` an edge.
fn compiled(files: &[(&str, &str)]) -> CompiledProject {
    TestProject::new().files(files).compile_bare()
}

/// The delta from `base` to `now`, labelled `none`.
fn between(base: &CompiledProject, now: &CompiledProject) -> Delta {
    Delta::between(
        base.graph(),
        now.graph(),
        now.environment().registries().field_forms(),
        DeltaBase::None,
    )
}

/// Each node of `delta` as `<change> <kind> <id>`.
fn changes(delta: &Delta) -> Vec<String> {
    delta
        .payload()
        .nodes
        .iter()
        .map(|n| format!("{:?} {} {}", n.change, n.kind, n.id))
        .collect()
}

fn edge(source: &str, target: &str, label: &str) -> DeltaEdge {
    DeltaEdge {
        source: source.into(),
        target: target.into(),
        label: label.into(),
    }
}

const TWO: &str = "item a \"A\" {\n  size 1\n  refs [b]\n}\n\nnote b \"B\" {\n}\n";

#[specforge_test(requirement = "FEATURE-142")]
fn from_the_empty_graph_everything_is_added() {
    let now = compiled(&[("a.spec", TWO)]);
    let delta = Delta::between(
        &Graph::default(),
        now.graph(),
        now.environment().registries().field_forms(),
        DeltaBase::None,
    );
    let payload = delta.payload();
    assert_eq!(payload.base, DeltaBase::None);
    assert_eq!(changes(&delta), ["Added item a", "Added note b"]);
    for node in &payload.nodes {
        let after = node.after.as_deref().unwrap();
        assert!(
            after.starts_with("sha256:") && after.len() == 7 + 64,
            "{after}"
        );
        assert!(node.before.is_none() && node.fields.is_empty() && node.aspects.is_empty());
    }
    assert_eq!(payload.added_edges, [edge("a", "b", "refs")]);
    assert!(payload.removed_edges.is_empty());
}

#[specforge_test(requirement = "FEATURE-142")]
fn an_edited_field_is_a_modified_node_with_both_digests() {
    let base = compiled(&[("a.spec", TWO)]);
    let now = compiled(&[("a.spec", &TWO.replace("size 1", "size 2"))]);
    let delta = between(&base, &now);
    let [node] = delta.payload().nodes.as_slice() else {
        panic!("{:?}", delta.payload())
    };
    assert_eq!(
        (node.change, node.kind.as_str(), node.id.as_str()),
        (DeltaChange::Modified, "item", "a")
    );
    assert_eq!(node.fields, ["size"]);
    assert!(node.aspects.is_empty(), "{:?}", node.aspects);
    let (before, after) = (
        node.before.as_deref().unwrap(),
        node.after.as_deref().unwrap(),
    );
    assert_ne!(before, after);
    assert!(before.starts_with("sha256:") && after.starts_with("sha256:"));
    assert!(delta.payload().added_edges.is_empty() && delta.payload().removed_edges.is_empty());
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_layout_only_edit_is_no_change() {
    let base = compiled(&[("a.spec", TWO)]);
    let laid_out = TWO
        .replace("\n  ", "\n        ")
        .replace('\n', "\r\n")
        .replace("item a", "\r\n\r\nitem   a");
    let now = compiled(&[("a.spec", &laid_out)]);
    let delta = between(&base, &now);
    assert!(delta.is_empty(), "{:?}", delta.payload());
    assert_eq!(delta.payload().base, DeltaBase::None);
}

#[specforge_test(requirement = "FEATURE-142")]
fn a_kind_change_is_a_removal_and_an_addition() {
    let base = compiled(&[("a.spec", TWO)]);
    let now = compiled(&[("a.spec", &TWO.replace("item a", "note a"))]);
    let delta = between(&base, &now);
    assert_eq!(changes(&delta), ["Removed item a", "Added note a"]);
    let [removed, added] = delta.payload().nodes.as_slice() else {
        unreachable!()
    };
    assert!(removed.before.is_none() && removed.after.is_none());
    assert!(added.after.is_some() && added.before.is_none());
}

#[specforge_test(requirement = "FEATURE-142")]
fn nodes_sort_by_kind_then_id_and_edges_by_source_target_label_as_bytes() {
    let files = [
        (
            "a.spec",
            "note b \"B\" {\n  refs [Z, a]\n}\n\nnote Z \"Z\" {\n}\n",
        ),
        (
            "b.spec",
            "item a \"A\" {\n  refs [b]\n  uses [b]\n}\n\nitem B \"B2\" {\n}\n",
        ),
    ];
    let now = compiled(&files);
    let delta = Delta::between(
        &Graph::default(),
        now.graph(),
        now.environment().registries().field_forms(),
        DeltaBase::Previous,
    );
    assert_eq!(
        changes(&delta),
        [
            "Added item B",
            "Added item a",
            "Added note Z",
            "Added note b"
        ]
    );
    let edges = &delta.payload().added_edges;
    let mut sorted = edges.clone();
    sorted.sort_by(|x, y| {
        (x.source.as_bytes(), x.target.as_bytes(), x.label.as_bytes()).cmp(&(
            y.source.as_bytes(),
            y.target.as_bytes(),
            y.label.as_bytes(),
        ))
    });
    assert_eq!(edges, &sorted);
    assert_eq!(edges[0], edge("a", "b", "refs"), "{edges:?}");
    assert_eq!(edges.last().unwrap(), &edge("b", "a", "refs"), "{edges:?}");
}

/// A base of an item and a note, and a graph where each changed and an edge of each went.
fn mixed() -> Delta {
    let base = compiled(&[
        ("a.spec", "item a \"A\" {\n  refs [n]\n}\n"),
        (
            "b.spec",
            "note n \"N\" {\n  refs [m]\n}\n\nnote m \"M\" {\n}\n",
        ),
    ]);
    let now = compiled(&[
        ("a.spec", "item a \"A2\" {\n}\n\nitem c \"C\" {\n}\n"),
        ("b.spec", "note n \"N\" {\n}\n\nnote m \"M2\" {\n}\n"),
    ]);
    between(&base, &now)
}

#[specforge_test(requirement = "FEATURE-142")]
fn restricted_to_a_kind_keeps_its_nodes_and_the_edges_with_an_end_of_it() {
    let delta = mixed();
    let items = delta.restricted_to(&["item".to_string()]);
    let named: Vec<(&str, &str)> = items
        .nodes
        .iter()
        .map(|n| (n.kind.as_str(), n.id.as_str()))
        .collect();
    assert_eq!(named, [("item", "a"), ("item", "c")]);
    // `a -refs-> n` has an item end; `n -refs-> m` has none
    assert_eq!(items.removed_edges, [edge("a", "n", "refs")]);
    assert!(items.added_edges.is_empty());
    let modified: &DeltaNode = &items.nodes[0];
    assert!(
        modified.aspects.contains(&DeltaAspect::Title),
        "{modified:?}"
    );
    let notes = delta.restricted_to(&["note".to_string()]);
    assert!(notes.nodes.iter().all(|n| n.kind == "note"), "{notes:?}");
    assert_eq!(
        notes.removed_edges,
        [edge("a", "n", "refs"), edge("n", "m", "refs")]
    );
    let none = delta.restricted_to(&[]);
    assert!(none.nodes.is_empty() && none.added_edges.is_empty() && none.removed_edges.is_empty());
    assert_eq!(none.base, DeltaBase::None);
}

#[specforge_test(requirement = "FEATURE-142")]
fn without_a_file_drops_its_nodes_and_every_edge_with_an_end_there() {
    let delta = mixed();
    let kept = delta.without(|file| file == "b.spec");
    assert!(
        kept.payload().nodes.iter().all(|n| n.kind == "item"),
        "{:?}",
        kept.payload()
    );
    assert_eq!(kept.payload().nodes.len(), 2);
    assert!(
        kept.payload().removed_edges.is_empty(),
        "{:?}",
        kept.payload()
    );
    let all = delta.without(|_| false);
    assert_eq!(&all, &delta);
}
