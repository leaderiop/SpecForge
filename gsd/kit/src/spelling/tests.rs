//! The host's own resolution tests (`crates/pipeline/graph/tests/resolution.rs`, the numeric and rooted readings), read
//! over a command's graph, plus the truths a verdict names.

use super::*;

fn graph(ids: &[&str]) -> CommandGraph {
    let nodes = ids
        .iter()
        .map(|id| GraphNode {
            id: (*id).to_string(),
            kind: "thing".to_string(),
            ..Default::default()
        })
        .collect();
    CommandGraph::new(nodes, Vec::new())
}

fn one(graph: &CommandGraph, spelling: &str) -> Option<String> {
    match find(graph, spelling) {
        Found::One(node) => Some(node.id.clone()),
        _ => None,
    }
}

#[test]
fn a_numeric_segment_matches_its_padded_form() {
    let mut ids = vec!["v1.0", "v1.0/03", "v1.0/02.1", "v1.0/03a", "v1.0/03/03-01"];
    let g = graph(&ids);
    assert_eq!(one(&g, "3").as_deref(), Some("v1.0/03"));
    assert_eq!(one(&g, "v1.0/3").as_deref(), Some("v1.0/03"));
    assert_eq!(one(&g, "003").as_deref(), Some("v1.0/03"));
    assert_eq!(one(&g, "2.1").as_deref(), Some("v1.0/02.1"));
    assert_eq!(one(&g, "3a").as_deref(), Some("v1.0/03a"));
    for none in ["3.1", "2.01", "3-1", "4"] {
        assert_eq!(find(&g, none), Found::None, "{none}");
    }
    // The exact spelling is still found first.
    assert_eq!(one(&g, "03").as_deref(), Some("v1.0/03"));
    ids.extend(["v2.0", "v2.0/03"]);
    let g = graph(&ids);
    assert_eq!(find(&g, "3"), Found::Ambiguous(vec!["v1.0/03", "v2.0/03"]));
    assert_eq!(one(&g, "v2.0/3").as_deref(), Some("v2.0/03"));
}

#[test]
fn a_rooted_spelling_is_never_a_suffix() {
    let mut ids = vec!["v1.0", "v1.0/03", "v1.0/03/03-01"];
    let g = graph(&ids);
    for rooted in ["/3", "/03", "/03-01", "/01-01"] {
        assert_eq!(find(&g, rooted), Found::None, "{rooted}");
    }
    assert_eq!(one(&g, "/v1.0/3").as_deref(), Some("v1.0/03"));
    assert_eq!(one(&g, "/v1.0/03/03-01").as_deref(), Some("v1.0/03/03-01"));
    // A root-level phase is still found by its padded rooted spelling.
    ids.push("04");
    let g = graph(&ids);
    assert_eq!(one(&g, "/4").as_deref(), Some("04"));
    assert_eq!(one(&g, "3").as_deref(), Some("v1.0/03"));
}

#[test]
fn every_spelling_of_a_truth_names_it_and_a_shared_end_is_ambiguous() {
    let g = graph(&[
        "v1.0/01/01-01/TR1",
        "v1.0/01/01-02/TR1",
        "v1.0/01/01-01/AR1",
    ]);
    for spelling in [
        "v1.0/01/01-01/TR1",
        "/v1.0/01/01-01/TR1",
        "01/01-01/TR1",
        "01-01/TR1",
        "v1.0/1/01-01/TR1",
        "/v1.0/001/01-01/TR1",
    ] {
        assert_eq!(
            one(&g, spelling).as_deref(),
            Some("v1.0/01/01-01/TR1"),
            "{spelling}"
        );
    }
    assert_eq!(
        find(&g, "TR1"),
        Found::Ambiguous(vec!["v1.0/01/01-01/TR1", "v1.0/01/01-02/TR1"])
    );
    for none in ["", "/", "v1.0//01", "/01-01/TR1", "v1.0/01/1-1/TR1", "TR9"] {
        assert_eq!(find(&g, none), Found::None, "{none:?}");
    }
}

#[test]
fn a_scheme_id_is_looked_up_whole() {
    let g = graph(&["file:src/a.js", "x/file:src/a.js"]);
    assert_eq!(one(&g, "file:src/a.js").as_deref(), Some("file:src/a.js"));
    assert_eq!(find(&g, "src/a.js"), Found::None);
}

#[test]
fn a_refusal_names_the_spelling_and_the_paths() {
    let g = graph(&["a/T1", "b/T1"]);
    assert_eq!(
        find(&g, "T1").one("T1").expect_err("ambiguous"),
        "'T1' is ambiguous: a/T1, b/T1; name one by its path"
    );
    assert_eq!(
        find(&g, "T2").one("T2").expect_err("none"),
        "'T2' names no entity of the project"
    );
    assert_eq!(
        find(&g, "a/T1").one("a/T1").map(|n| n.id.as_str()),
        Ok("a/T1")
    );
}
