//! `term_graph`, `term_clusters` and `term_density` over `see_also`.

use super::host::*;
use serde_json::{json, Value};
use specforge_test::prelude::*;

// ── term analytics ────────────────────────────────────────────────────────

/// A glossary: api -> rest -> http -> tcp, grpc -> proto, and lone, which
/// links nowhere.
fn glossary() -> G {
    G::default()
        .n("api", "term")
        .n("rest", "term")
        .n("http", "term")
        .n("tcp", "term")
        .n("grpc", "term")
        .n("proto", "term")
        .n("lone", "term")
        .edge("api", "rest", "see_also")
        .edge("rest", "http", "see_also")
        .edge("http", "tcp", "see_also")
        .edge("grpc", "proto", "see_also")
}

/// Terms named `ids` with a see_also from each pair's first to its second.
fn terms(ids: &[&str], links: &[(&str, &str)]) -> G {
    let mut g = G::default();
    for id in ids {
        g = g.n(id, "term");
    }
    for (a, b) in links {
        g = g.edge(a, b, "see_also");
    }
    g
}

fn related(term: &str, args: Value, g: &G) -> Value {
    let mut args = args;
    args["term"] = json!(term);
    json_of("term_graph", args, g)["related_terms"].clone()
}

#[specforge_test(
    behavior = "surface_term_graph",
    verify = "term-graph returns TermGraphPayload JSON"
)]
fn term_graph_answers_its_payload() {
    let tg = json_of(
        "term_graph",
        json!({"term": "api", "max_hops": 2}),
        &glossary(),
    );
    assert_eq!(
        tg,
        json!({"term_id": "api", "related_terms": ["http", "rest"], "max_hops": 2})
    );
    let human = human_of("term_graph", json!({"term": "api"}), &glossary());
    assert_eq!(
        human,
        "Terms related to 'api' within 1 see_also hop:\n  rest\n"
    );
}

#[specforge_test(
    behavior = "surface_term_graph",
    verify = "max-hops flag is respected and capped at 5"
)]
fn term_graph_follows_max_hops_up_to_five() {
    let chain = terms(
        &["t0", "t1", "t2", "t3", "t4", "t5", "t6", "t7"],
        &[
            ("t0", "t1"),
            ("t1", "t2"),
            ("t2", "t3"),
            ("t3", "t4"),
            ("t4", "t5"),
            ("t5", "t6"),
            ("t6", "t7"),
        ],
    );
    assert_eq!(
        related("t0", json!({"max_hops": 3}), &chain),
        json!(["t1", "t2", "t3"])
    );
    let capped = json_of("term_graph", json!({"term": "t0", "max_hops": 7}), &chain);
    assert_eq!(capped["max_hops"], 5);
    assert_eq!(
        capped["related_terms"],
        json!(["t1", "t2", "t3", "t4", "t5"])
    );
    // Not a count: refused.
    let error = run_in(
        &runtime(),
        "term_graph",
        json!({"term": "t0", "max_hops": -1}),
        &chain,
        "json",
    );
    assert_eq!(error.exit, 2);
    assert_eq!(error.error()["code"], "INVALID_INPUT");
}

#[specforge_test(
    behavior = "surface_term_graph",
    verify = "missing term ID returns error with suggestion"
)]
fn term_graph_of_a_mistyped_term_suggests_the_nearest() {
    let error = not_found_suggesting("term_graph", json!({"term": "apj"}), &glossary());
    assert_eq!(error["suggestion"], "api");
    assert_eq!(error["message"], "term 'apj' not found");
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "term with see_also returns related terms at hop 1"
)]
fn a_terms_see_also_is_related_at_one_hop() {
    assert_eq!(
        related("rest", json!({"max_hops": 1}), &glossary()),
        json!(["http"])
    );
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "term with 2-hop chain returns transitive terms when maxHops=2"
)]
fn a_two_hop_chain_is_related_at_two_hops() {
    assert_eq!(
        related("rest", json!({"max_hops": 2}), &glossary()),
        json!(["http", "tcp"])
    );
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "term with no see_also returns empty related_terms"
)]
fn a_term_without_see_also_has_no_related_terms() {
    assert_eq!(
        related("lone", json!({"max_hops": 5}), &glossary()),
        json!([])
    );
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "source term is excluded from related_terms"
)]
fn the_term_is_not_its_own_relation() {
    let cycle = terms(&["a", "b"], &[("a", "b"), ("b", "a"), ("a", "a")]);
    assert_eq!(related("a", json!({"max_hops": 5}), &cycle), json!(["b"]));
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "omitted maxHops defaults to 1"
)]
fn max_hops_defaults_to_one() {
    let tg = json_of("term_graph", json!({"term": "api"}), &glossary());
    assert_eq!(
        (tg["max_hops"].clone(), tg["related_terms"].clone()),
        (json!(1), json!(["rest"]))
    );
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "maxHops=10 is clamped to 5"
)]
fn ten_hops_are_five() {
    let tg = json_of(
        "term_graph",
        json!({"term": "api", "max_hops": 10}),
        &glossary(),
    );
    assert_eq!(tg["max_hops"], 5);
    assert_eq!(tg["related_terms"], json!(["http", "rest", "tcp"]));
}

#[specforge_test(
    behavior = "pe_query_term_graph",
    verify = "maxHops=0 returns empty related_terms"
)]
fn zero_hops_relate_nothing() {
    assert_eq!(
        related("api", json!({"max_hops": 0}), &glossary()),
        json!([])
    );
}

#[specforge_test(
    behavior = "product_query_correctness",
    verify = "term graph respects maxHops boundary"
)]
fn each_hop_adds_exactly_the_terms_one_further() {
    // Along api -> rest -> http -> tcp, hop h reaches the first h terms.
    let order = ["rest", "http", "tcp"];
    for hops in 0..=5usize {
        let mut expected: Vec<&str> = order[..hops.min(3)].to_vec();
        expected.sort_unstable();
        assert_eq!(
            related("api", json!({"max_hops": hops}), &glossary()),
            json!(expected),
            "{hops} hops"
        );
    }
}

#[specforge_test(
    behavior = "surface_term_clusters",
    verify = "product:term-clusters returns TermClusterPayload"
)]
fn term_clusters_answers_its_payload() {
    let tc = json_of("term_clusters", json!({}), &glossary());
    assert_eq!(
        tc,
        json!({
            "clusters": [
                {"cluster_id": 1, "term_ids": ["api", "http", "rest", "tcp"], "term_count": 4},
                {"cluster_id": 2, "term_ids": ["grpc", "proto"], "term_count": 2},
            ],
            "cluster_count": 2, "isolated_count": 1, "total_terms": 7,
        })
    );
    let human = human_of("term_clusters", json!({}), &glossary());
    assert_eq!(
        human,
        "cluster  terms  ids\n\
         1        4      api, http, rest, tcp\n\
         2        2      grpc, proto\n\
         2 clusters, 1 isolated of 7 terms\n"
    );
}

#[specforge_test(
    behavior = "surface_term_clusters",
    verify = "no terms returns zero clusters and zero isolated"
)]
fn term_clusters_of_no_terms_is_empty() {
    assert_eq!(
        json_of("term_clusters", json!({}), &G::default().n("f1", "feature")),
        json!({"clusters": [], "cluster_count": 0, "isolated_count": 0, "total_terms": 0})
    );
}

fn cluster_ids(g: &G) -> Value {
    let tc = json_of("term_clusters", json!({}), g);
    Value::Array(
        tc["clusters"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["term_ids"].clone())
            .collect(),
    )
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "three terms in a connected chain produce one cluster of size 3"
)]
fn a_chain_of_three_terms_is_one_cluster() {
    // b links to both, so a and c are clustered through it.
    let g = terms(&["a", "b", "c"], &[("b", "a"), ("b", "c")]);
    let tc = json_of("term_clusters", json!({}), &g);
    assert_eq!(
        tc["clusters"],
        json!([{"cluster_id": 1, "term_ids": ["a", "b", "c"], "term_count": 3}])
    );
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "two disconnected pairs produce two clusters of size 2"
)]
fn two_disconnected_pairs_are_two_clusters() {
    let g = terms(&["a", "b", "c", "d"], &[("a", "b"), ("d", "c")]);
    assert_eq!(cluster_ids(&g), json!([["a", "b"], ["c", "d"]]));
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "isolated term with no see_also edges is counted in isolated_count"
)]
fn a_term_without_see_also_is_isolated() {
    let tc = json_of("term_clusters", json!({}), &glossary());
    assert_eq!(tc["isolated_count"], 1);
    assert!(!cluster_ids(&glossary()).to_string().contains("lone"));
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "empty term graph returns zero clusters and zero isolated"
)]
fn an_empty_glossary_has_no_clusters() {
    let tc = json_of("term_clusters", json!({}), &G::default());
    assert_eq!(
        (
            tc["cluster_count"].clone(),
            tc["isolated_count"].clone(),
            tc["total_terms"].clone()
        ),
        (json!(0), json!(0), json!(0))
    );
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "total_terms equals sum of cluster sizes plus isolated_count"
)]
fn cluster_sizes_and_isolated_terms_add_up_to_every_term() {
    let tc = json_of("term_clusters", json!({}), &glossary());
    let sizes: u64 = tc["clusters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["term_count"].as_u64().unwrap())
        .sum();
    assert_eq!(
        sizes + tc["isolated_count"].as_u64().unwrap(),
        tc["total_terms"].as_u64().unwrap()
    );
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "clusters sorted by size descending"
)]
fn the_largest_cluster_comes_first() {
    let g = terms(
        &["a", "b", "x", "y", "z", "m", "n"],
        &[("a", "b"), ("x", "y"), ("y", "z"), ("n", "m")],
    );
    assert_eq!(
        cluster_ids(&g),
        json!([["x", "y", "z"], ["a", "b"], ["m", "n"]])
    );
}

#[specforge_test(
    behavior = "pe_query_term_clusters",
    verify = "result is deterministic across repeated queries"
)]
fn term_clusters_are_the_same_every_time_and_in_any_order() {
    let first = json_of("term_clusters", json!({}), &glossary());
    assert_eq!(json_of("term_clusters", json!({}), &glossary()), first);
    let mut g = glossary();
    g.nodes.reverse();
    g.edges.reverse();
    assert_eq!(json_of("term_clusters", json!({}), &g), first);
}

/// A hub and six spokes linked in a ring and by two chords: 14 links over
/// 7 terms, an average of 2; the hub has 6 connections, each spoke 3 or 4.
fn hub() -> G {
    terms(
        &["h", "s1", "s2", "s3", "s4", "s5", "s6"],
        &[
            ("h", "s1"),
            ("h", "s2"),
            ("h", "s3"),
            ("h", "s4"),
            ("h", "s5"),
            ("h", "s6"),
            ("s1", "s2"),
            ("s2", "s3"),
            ("s3", "s4"),
            ("s4", "s5"),
            ("s5", "s6"),
            ("s6", "s1"),
            ("s1", "s3"),
            ("s2", "s4"),
        ],
    )
}

#[specforge_test(
    behavior = "surface_term_density",
    verify = "product:term-density returns TermDensityPayload"
)]
fn term_density_answers_its_payload() {
    let td = json_of("term_density", json!({}), &glossary());
    assert_eq!(
        td,
        json!({"total_terms": 7, "total_see_also": 4, "avg_connections": 4.0 / 7.0,
            "max_connections": 2, "hub_terms": [], "isolated_terms": ["lone"]})
    );
    let human = human_of("term_density", json!({}), &hub());
    assert_eq!(
        human,
        "Terms:           7\n\
         see_also edges:  14\n\
         Avg connections: 2.00\n\
         Max connections: 6\n\
         Hubs (1):        h\n\
         Isolated (0):    -\n"
    );
}

#[specforge_test(
    behavior = "surface_term_density",
    verify = "empty graph returns zero stats"
)]
fn term_density_of_no_terms_is_zero() {
    assert_eq!(
        json_of("term_density", json!({}), &G::default()),
        json!({"total_terms": 0, "total_see_also": 0, "avg_connections": null,
            "max_connections": 0, "hub_terms": [], "isolated_terms": []})
    );
}

#[specforge_test(
    behavior = "pe_query_term_density",
    verify = "5 terms with 8 edges computes correct average"
)]
fn five_terms_with_eight_links_average_one_point_six() {
    let g = terms(
        &["a", "b", "c", "d", "e"],
        &[
            ("a", "b"),
            ("a", "c"),
            ("a", "d"),
            ("a", "e"),
            ("b", "c"),
            ("c", "d"),
            ("d", "e"),
            ("e", "b"),
        ],
    );
    let td = json_of("term_density", json!({}), &g);
    assert_eq!(
        (td["total_see_also"].clone(), td["avg_connections"].clone()),
        (json!(8), json!(1.6))
    );
}

#[specforge_test(
    behavior = "pe_query_term_density",
    verify = "term with 6 connections in a graph averaging 2 is a hub"
)]
fn six_connections_against_an_average_of_two_is_a_hub() {
    let td = json_of("term_density", json!({}), &hub());
    assert_eq!(td["avg_connections"], 2.0);
    assert_eq!(td["hub_terms"], json!(["h"]));
}

#[specforge_test(
    behavior = "pe_query_term_density",
    verify = "term with zero connections is listed in isolated_terms"
)]
fn a_term_without_links_is_isolated() {
    assert_eq!(
        json_of("term_density", json!({}), &glossary())["isolated_terms"],
        json!(["lone"])
    );
}

#[specforge_test(
    behavior = "pe_query_term_density",
    verify = "empty term graph returns total_terms=0 and avg_connections=null"
)]
fn an_empty_glossary_has_no_average() {
    let td = json_of("term_density", json!({}), &G::default().n("f1", "feature"));
    assert_eq!(
        (td["total_terms"].clone(), td["avg_connections"].clone()),
        (json!(0), json!(null))
    );
}

#[specforge_test(
    behavior = "pe_query_term_density",
    verify = "max_connections reflects the most-connected term"
)]
fn max_connections_is_the_hubs() {
    assert_eq!(
        json_of("term_density", json!({}), &hub())["max_connections"],
        6
    );
    // rest and http each link two terms, one either way.
    assert_eq!(
        json_of("term_density", json!({}), &glossary())["max_connections"],
        2
    );
}

#[specforge_test(
    behavior = "pe_query_term_density",
    verify = "result is deterministic across repeated queries"
)]
fn term_density_is_the_same_every_time_and_in_any_order() {
    let first = json_of("term_density", json!({}), &hub());
    assert_eq!(json_of("term_density", json!({}), &hub()), first);
    let mut g = hub();
    g.nodes.reverse();
    g.edges.reverse();
    assert_eq!(json_of("term_density", json!({}), &g), first);
}
