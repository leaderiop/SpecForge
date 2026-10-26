//! An entity only it references, through the real builtin extensions (ADR 0270): the edge rules
//! report it, as the graph's degree calls it unconnected.

use specforge_project::testing::TestProject;
use specforge_test_macros::test as spec;

/// One entity referencing only itself beside one referencing nothing, per kind.
const SELF_REFERENCES: &str = "\
feature f_self \"Self feature\" {\n  problem \"p\"\n  depends_on [f_self]\n}\n\n\
feature f_alone \"Alone feature\" {\n  problem \"p\"\n}\n\n\
module m_self \"Self module\" {\n  depends_on [m_self]\n}\n\n\
module m_alone \"Alone module\" {\n}\n\n\
type T_self \"Self type\" {\n  composed_types [T_self]\n  label string\n}\n\n\
type T_alone \"Alone type\" {\n  label string\n}\n\n\
process p_self \"Self process\" {\n  alphabet [a]\n  initial_state \"s\"\n  sub_processes [p_self]\n}\n\n\
process p_alone \"Alone process\" {\n  alphabet [a]\n  initial_state \"s\"\n}\n\n\
term loner \"Loner\" {\n  definition \"a term only it names\"\n  see_also [loner]\n}\n\n\
term orphan \"Orphan\" {\n  definition \"a term nothing names\"\n}\n\n\
behavior b1 \"B1\" {\n}\n\nbehavior b2 \"B2\" {\n}\n\n\
refinement r_alone \"Alone\" {\n  refines b1\n  refined_by b2\n}\n\n\
refinement r_self \"Self\" {\n  refines b1\n  refined_by b2\n  chains_to r_self\n}\n";

#[spec(
    behavior = "execute_validation_pattern",
    verify = "an edge from an entity to itself is no incoming edge: a builtin no_incoming_edges rule reports an entity only it references"
)]
#[spec(
    behavior = "execute_validation_pattern",
    verify = "a reference an entity writes to itself gives its field its link: no_edges stays quiet and W061 reports it"
)]
fn an_entity_only_it_references_is_unreferenced() {
    let compiled = TestProject::new()
        .enabling(&[
            "@specforge/software",
            "@specforge/product",
            "@specforge/formal",
        ])
        .file("main.spec", SELF_REFERENCES)
        .compile();
    let rules = ["W002", "W041", "W044", "W131", "W134", "I010"];
    let id_of = |message: &str| message.split('\'').nth(1).unwrap_or_default().to_string();
    let mut reported: Vec<(String, String)> = compiled
        .diagnostics()
        .iter()
        .filter(|d| rules.contains(&d.code.as_str()))
        .map(|d| (d.code.clone(), id_of(&d.message)))
        .collect();
    reported.sort();
    let pair = |code: &str, id: &str| (code.to_string(), id.to_string());
    assert_eq!(
        reported,
        [
            pair("I010", "orphan"),
            pair("W002", "T_alone"),
            pair("W002", "T_self"),
            pair("W041", "f_alone"),
            pair("W041", "f_self"),
            pair("W044", "m_alone"),
            pair("W044", "m_self"),
            pair("W131", "r_alone"),
            pair("W131", "r_self"),
            pair("W134", "p_alone"),
            pair("W134", "p_self"),
        ]
    );
    assert!(
        compiled
            .diagnostics()
            .iter()
            .any(|d| d.code == "W061" && d.message.contains("loner -> loner"))
    );
    // The rules and the degree answer alike for the kinds the five untyped rules target.
    let graph = compiled.graph();
    for (code, kind) in [
        ("W002", "type"),
        ("W041", "feature"),
        ("W044", "module"),
        ("W131", "refinement"),
        ("W134", "process"),
    ] {
        for node in graph
            .nodes()
            .into_iter()
            .filter(|n| n.kind.raw.as_str() == kind)
        {
            let id = node.id.raw.as_str();
            let flagged = reported.iter().any(|(c, i)| c == code && i == id);
            assert_eq!(flagged, graph.degree(id).incoming == 0, "{code} {id}");
        }
    }
}
