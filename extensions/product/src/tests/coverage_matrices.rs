//! The persona and channel coverage matrices, and feature overlap.

use super::host::*;
use serde_json::{json, Value};
use specforge_test::prelude::*;

// ── coverage matrices ─────────────────────────────────────────────────────

/// Personas, channels and deliverables over four features:
/// - j1 (dev, cli): f1, f2. j2 (dev, cli, web): f2, f3. j3 (ops, web): f4.
///   j4 (no persona or channel): f1. idle and none have no journey.
/// - d1: j1, j4. d2: j2, m1, m2. d3: m1, m2. m1: f1. m2: f2.
fn reach() -> G {
    G::default()
        .n("f1", "feature")
        .n("f2", "feature")
        .n("f3", "feature")
        .n("f4", "feature")
        .n("dev", "persona")
        .n("ops", "persona")
        .n("idle", "persona")
        .n("cli", "channel")
        .n("web", "channel")
        .n("none", "channel")
        .n("j1", "journey")
        .n("j2", "journey")
        .n("j3", "journey")
        .n("j4", "journey")
        .n("d1", "deliverable")
        .n("d2", "deliverable")
        .n("d3", "deliverable")
        .n("m1", "module")
        .n("m2", "module")
        .edge("j1", "dev", "persona")
        .edge("j2", "dev", "persona")
        .edge("j3", "ops", "persona")
        .edge("j1", "cli", "channels")
        .edge("j2", "cli", "channels")
        .edge("j2", "web", "channels")
        .edge("j3", "web", "channels")
        .edge("j1", "f1", "features")
        .edge("j1", "f2", "features")
        .edge("j2", "f2", "features")
        .edge("j2", "f3", "features")
        .edge("j3", "f4", "features")
        .edge("j4", "f1", "features")
        .edge("d1", "j1", "journeys")
        .edge("d1", "j4", "journeys")
        .edge("d2", "j2", "journeys")
        .edge("d2", "m1", "modules")
        .edge("d2", "m2", "modules")
        .edge("d3", "m1", "modules")
        .edge("d3", "m2", "modules")
        .edge("m1", "f1", "features")
        .edge("m2", "f2", "features")
}

/// The entry for `id` under `key` in a matrix payload.
fn matrix_entry(payload: &Value, key: &str, id_key: &str, id: &str) -> Value {
    payload[key]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e[id_key] == id)
        .unwrap_or_else(|| panic!("{id} not in {payload}"))
        .clone()
}

/// `n` personas, each the only one of its journey over one feature.
fn many_personas(n: usize) -> G {
    let mut g = G::default().n("f", "feature");
    for i in 0..n {
        let (p, j) = (format!("p{i:02}"), format!("j{i:02}"));
        g = g.n(&p, "persona").n(&j, "journey").edge(&j, &p, "persona");
    }
    g
}

fn ids_under(payload: &Value, key: &str, id_key: &str) -> Vec<String> {
    payload[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e[id_key].as_str().unwrap().to_string())
        .collect()
}

#[specforge_test(
    behavior = "surface_coverage_matrix",
    verify = "product:coverage-matrix returns PersonaCoverageMatrixPayload"
)]
fn coverage_matrix_answers_its_payload() {
    let cm = json_of("coverage_matrix", json!({}), &reach());
    assert_eq!(
        cm,
        json!({
            "personas": [
                {"persona_id": "dev", "reachable_features": ["f1", "f2", "f3"],
                    "unreachable_features": ["f4"], "coverage_ratio": 0.75, "journey_count": 2},
                {"persona_id": "idle", "reachable_features": [],
                    "unreachable_features": ["f1", "f2", "f3", "f4"], "coverage_ratio": 0.0,
                    "journey_count": 0},
                {"persona_id": "ops", "reachable_features": ["f4"],
                    "unreachable_features": ["f1", "f2", "f3"], "coverage_ratio": 0.25,
                    "journey_count": 1},
            ],
            "total_features": 4, "overall_coverage": 1.0 / 3.0,
            "total": 3, "offset": 0, "limit": 100, "has_more": false,
        })
    );
    let human = human_of("coverage_matrix", json!({}), &reach());
    assert_eq!(
        human,
        "persona  reachable  unreachable  coverage\n\
         dev      3          1            75%\n\
         idle     0          4            0%\n\
         ops      1          3            25%\n\
         Overall coverage: 33%\n"
    );
}

#[specforge_test(
    behavior = "surface_coverage_matrix",
    verify = "no personas returns empty matrix"
)]
fn coverage_matrix_of_no_personas_is_empty() {
    let cm = json_of(
        "coverage_matrix",
        json!({}),
        &G::default().n("f1", "feature"),
    );
    assert_eq!(
        cm,
        json!({"personas": [], "total_features": 1, "overall_coverage": null,
            "total": 0, "offset": 0, "limit": 100, "has_more": false})
    );
}

#[specforge_test(
    behavior = "surface_channel_coverage_matrix",
    verify = "product:channel-coverage-matrix returns ChannelCoverageMatrixPayload"
)]
fn channel_coverage_matrix_answers_its_payload() {
    let cm = json_of("channel_coverage_matrix", json!({}), &reach());
    assert_eq!(
        cm,
        json!({
            "channels": [
                {"channel_id": "cli", "reachable_features": ["f1", "f2", "f3"],
                    "unreachable_features": ["f4"], "coverage_ratio": 0.75, "journey_count": 2},
                {"channel_id": "none", "reachable_features": [],
                    "unreachable_features": ["f1", "f2", "f3", "f4"], "coverage_ratio": 0.0,
                    "journey_count": 0},
                {"channel_id": "web", "reachable_features": ["f2", "f3", "f4"],
                    "unreachable_features": ["f1"], "coverage_ratio": 0.75, "journey_count": 2},
            ],
            "total_features": 4, "overall_coverage": 0.5,
            "total": 3, "offset": 0, "limit": 100, "has_more": false,
        })
    );
    let page = json_of(
        "channel_coverage_matrix",
        json!({"offset": 1, "limit": 1}),
        &reach(),
    );
    assert_eq!(ids_under(&page, "channels", "channel_id"), ["none"]);
    assert_eq!(
        (page["has_more"].clone(), page["overall_coverage"].clone()),
        (json!(true), json!(0.5))
    );
    let human = human_of("channel_coverage_matrix", json!({}), &reach());
    assert!(
        human.starts_with(
            "channel  reachable  unreachable  coverage\ncli      3          1            75%\n"
        ),
        "{human}"
    );
    assert!(human.ends_with("Overall coverage: 50%\n"), "{human}");
}

#[specforge_test(
    behavior = "surface_channel_coverage_matrix",
    verify = "no channels returns empty matrix"
)]
fn channel_coverage_matrix_of_no_channels_is_empty() {
    let cm = json_of("channel_coverage_matrix", json!({}), &G::default());
    assert_eq!(
        cm,
        json!({"channels": [], "total_features": 0, "overall_coverage": null,
            "total": 0, "offset": 0, "limit": 100, "has_more": false})
    );
}

#[specforge_test(
    behavior = "pe_query_persona_coverage_matrix",
    verify = "persona with journeys covering all features has coverage_ratio=1.0"
)]
fn a_persona_whose_journeys_cover_every_feature_is_at_one() {
    let g = reach()
        .edge("j1", "ops", "persona")
        .edge("j2", "ops", "persona");
    let ops = matrix_entry(
        &json_of("coverage_matrix", json!({}), &g),
        "personas",
        "persona_id",
        "ops",
    );
    assert_eq!(ops["coverage_ratio"], 1.0);
    assert_eq!(ops["unreachable_features"], json!([]));
}

#[specforge_test(
    behavior = "pe_query_persona_coverage_matrix",
    verify = "persona with no journeys has coverage_ratio=0.0"
)]
fn a_persona_without_journeys_is_at_zero() {
    let idle = matrix_entry(
        &json_of("coverage_matrix", json!({}), &reach()),
        "personas",
        "persona_id",
        "idle",
    );
    assert_eq!(
        (
            idle["coverage_ratio"].clone(),
            idle["journey_count"].clone()
        ),
        (json!(0.0), json!(0))
    );
}

#[specforge_test(
    behavior = "pe_query_persona_coverage_matrix",
    verify = "overall_coverage is arithmetic mean of persona ratios"
)]
fn overall_persona_coverage_is_the_mean_of_the_ratios() {
    let cm = json_of("coverage_matrix", json!({}), &reach());
    let ratios: Vec<f64> = cm["personas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["coverage_ratio"].as_f64().unwrap())
        .collect();
    assert_eq!(
        cm["overall_coverage"].as_f64().unwrap(),
        ratios.iter().sum::<f64>() / 3.0
    );
    // Over every persona, not the page.
    let page = json_of("coverage_matrix", json!({"limit": 1}), &reach());
    assert_eq!(page["overall_coverage"], cm["overall_coverage"]);
}

#[specforge_test(
    behavior = "pe_query_persona_coverage_matrix",
    verify = "diamond topology deduplicates shared features"
)]
fn a_feature_two_of_a_personas_journeys_share_counts_once() {
    // dev reaches f2 through j1 and j2.
    let dev = matrix_entry(
        &json_of("coverage_matrix", json!({}), &reach()),
        "personas",
        "persona_id",
        "dev",
    );
    assert_eq!(dev["reachable_features"], json!(["f1", "f2", "f3"]));
}

#[specforge_test(
    behavior = "pe_query_channel_coverage_matrix",
    verify = "channel with journeys covering all features has coverage_ratio=1.0"
)]
fn a_channel_whose_journeys_cover_every_feature_is_at_one() {
    let g = reach().edge("j4", "web", "channels");
    let web = matrix_entry(
        &json_of("channel_coverage_matrix", json!({}), &g),
        "channels",
        "channel_id",
        "web",
    );
    assert_eq!(web["coverage_ratio"], 1.0);
    assert_eq!(web["journey_count"], 3);
}

#[specforge_test(
    behavior = "pe_query_channel_coverage_matrix",
    verify = "channel with no journeys has coverage_ratio=0.0"
)]
fn a_channel_without_journeys_is_at_zero() {
    let none = matrix_entry(
        &json_of("channel_coverage_matrix", json!({}), &reach()),
        "channels",
        "channel_id",
        "none",
    );
    assert_eq!(none["coverage_ratio"], 0.0);
}

#[specforge_test(
    behavior = "pe_query_channel_coverage_matrix",
    verify = "overall_coverage is arithmetic mean of channel ratios"
)]
fn overall_channel_coverage_is_the_mean_of_the_ratios() {
    // (0.75 + 0 + 0.75) / 3.
    let cm = json_of("channel_coverage_matrix", json!({}), &reach());
    assert_eq!(cm["overall_coverage"], 0.5);
}

#[specforge_test(
    behavior = "pe_query_channel_coverage_matrix",
    verify = "diamond topology deduplicates shared features"
)]
fn a_feature_two_of_a_channels_journeys_share_counts_once() {
    // web reaches f2 and f3 through j2, f4 through j3; cli f2 through j1 and j2.
    let cm = json_of("channel_coverage_matrix", json!({}), &reach());
    let cli = matrix_entry(&cm, "channels", "channel_id", "cli");
    assert_eq!(cli["reachable_features"], json!(["f1", "f2", "f3"]));
}

#[specforge_test(
    behavior = "pe_query_channel_coverage_matrix",
    verify = "empty channel graph returns empty matrix"
)]
fn a_graph_without_channels_has_an_empty_channel_matrix() {
    let cm = json_of("channel_coverage_matrix", json!({}), &many_personas(2));
    assert_eq!(
        (
            cm["channels"].clone(),
            cm["total"].clone(),
            cm["overall_coverage"].clone()
        ),
        (json!([]), json!(0), json!(null))
    );
}

#[specforge_test(
    behavior = "product_new_query_correctness",
    verify = "coverage matrix ratios are mathematically correct"
)]
fn each_coverage_ratio_is_reachable_over_total() {
    for (id, key, id_key) in [
        ("coverage_matrix", "personas", "persona_id"),
        ("channel_coverage_matrix", "channels", "channel_id"),
    ] {
        let cm = json_of(id, json!({}), &reach());
        let total = cm["total_features"].as_f64().unwrap();
        for e in cm[key].as_array().unwrap() {
            let reachable = e["reachable_features"].as_array().unwrap().len() as f64;
            let unreachable = e["unreachable_features"].as_array().unwrap().len() as f64;
            assert_eq!(reachable + unreachable, total, "{id}: {}", e[id_key]);
            assert_eq!(
                e["coverage_ratio"].as_f64().unwrap(),
                reachable / total,
                "{id}: {e}"
            );
        }
    }
}

#[specforge_test(
    behavior = "product_query_pagination_required",
    verify = "matrix query with limit=10 returns at most 10 entries"
)]
fn a_matrix_query_with_limit_ten_returns_ten() {
    let cm = json_of("coverage_matrix", json!({"limit": 10}), &many_personas(12));
    assert_eq!(cm["personas"].as_array().unwrap().len(), 10);
    assert_eq!(
        (cm["total"].clone(), cm["has_more"].clone()),
        (json!(12), json!(true))
    );
}

#[specforge_test(
    behavior = "product_query_pagination_required",
    verify = "offset=limit retrieves the second page"
)]
fn offset_equal_to_limit_is_the_second_page() {
    let g = many_personas(12);
    let first = json_of("coverage_matrix", json!({"limit": 5}), &g);
    let second = json_of("coverage_matrix", json!({"offset": 5, "limit": 5}), &g);
    let all = json_of("coverage_matrix", json!({}), &g);
    let ids = ids_under(&all, "personas", "persona_id");
    assert_eq!(ids_under(&first, "personas", "persona_id"), ids[..5]);
    assert_eq!(ids_under(&second, "personas", "persona_id"), ids[5..10]);
}

#[specforge_test(
    behavior = "product_query_pagination_required",
    verify = "limit>1000 is clamped to 1000"
)]
fn a_matrix_limit_above_a_thousand_is_a_thousand() {
    for id in [
        "coverage_matrix",
        "channel_coverage_matrix",
        "feature_overlap",
    ] {
        let page = json_of(id, json!({"limit": 5000}), &reach());
        assert_eq!(page["limit"], 1000, "{id}");
        let page = json_of(id, json!({"limit": 0}), &reach());
        assert_eq!(page["limit"], 1, "{id}");
        let error =
            RUNTIME.with(|runtime| run_in(runtime, id, json!({"offset": -1}), &reach(), "json"));
        assert_eq!(error.exit, 2, "{id}");
        assert_eq!(error.error()["code"], "INVALID_INPUT");
    }
}

#[specforge_test(
    behavior = "surface_feature_overlap",
    verify = "feature-overlap returns FeatureOverlapPayload JSON"
)]
fn feature_overlap_answers_its_payload() {
    let fo = json_of("feature_overlap", json!({}), &reach());
    assert_eq!(
        fo,
        json!({
            "overlapping_features": [
                {"feature_id": "f1", "deliverable_ids": ["d1", "d2", "d3"], "deliverable_count": 3},
                {"feature_id": "f2", "deliverable_ids": ["d1", "d2", "d3"], "deliverable_count": 3},
            ],
            "count": 2, "total_features": 4,
            "total": 2, "offset": 0, "limit": 100, "has_more": false,
        })
    );
    let page = json_of("feature_overlap", json!({"offset": 1}), &reach());
    assert_eq!(
        ids_under(&page, "overlapping_features", "feature_id"),
        ["f2"]
    );
    assert_eq!(page["count"], 2);
    let human = human_of("feature_overlap", json!({}), &reach());
    assert_eq!(
        human,
        "feature  deliverables  ids\n\
         f1       3             d1, d2, d3\n\
         f2       3             d1, d2, d3\n\
         2 of 4 features shared by two or more deliverables\n"
    );
}

#[specforge_test(
    behavior = "surface_feature_overlap",
    verify = "no overlapping features returns empty list"
)]
fn feature_overlap_without_shared_features_is_empty() {
    let fo = json_of("feature_overlap", json!({}), &plan());
    assert_eq!(
        fo,
        json!({"overlapping_features": [], "count": 0, "total_features": 3,
            "total": 0, "offset": 0, "limit": 100, "has_more": false})
    );
}

#[specforge_test(
    behavior = "pe_query_feature_overlap",
    verify = "feature in two deliverables appears in overlap list"
)]
fn a_feature_two_deliverables_reach_overlaps() {
    let g = G::default()
        .n("f", "feature")
        .n("j", "journey")
        .n("m", "module")
        .n("a", "deliverable")
        .n("b", "deliverable")
        .edge("j", "f", "features")
        .edge("m", "f", "features")
        .edge("a", "j", "journeys")
        .edge("b", "m", "modules");
    let fo = json_of("feature_overlap", json!({}), &g);
    assert_eq!(
        fo["overlapping_features"],
        json!([{"feature_id": "f", "deliverable_ids": ["a", "b"], "deliverable_count": 2}])
    );
}

#[specforge_test(
    behavior = "pe_query_feature_overlap",
    verify = "feature in one deliverable is excluded"
)]
fn a_feature_one_deliverable_reaches_does_not_overlap() {
    // f3 is d2's alone, f4 no deliverable's.
    let fo = json_of("feature_overlap", json!({}), &reach());
    let ids = ids_under(&fo, "overlapping_features", "feature_id");
    assert!(
        !ids.contains(&"f3".to_string()) && !ids.contains(&"f4".to_string()),
        "{fo}"
    );
}

#[specforge_test(
    behavior = "pe_query_feature_overlap",
    verify = "feature reachable via journey path and module path from same deliverable counts once"
)]
fn a_deliverable_reaching_a_feature_both_ways_counts_once() {
    // d2 reaches f2 through j2 and through m2.
    let fo = json_of("feature_overlap", json!({}), &reach());
    let f2 = matrix_entry(&fo, "overlapping_features", "feature_id", "f2");
    assert_eq!(f2["deliverable_ids"], json!(["d1", "d2", "d3"]));
    // A deliverable alone, both ways, is no overlap.
    let g = G::default()
        .n("f", "feature")
        .n("j", "journey")
        .n("m", "module")
        .n("a", "deliverable")
        .edge("j", "f", "features")
        .edge("m", "f", "features")
        .edge("a", "j", "journeys")
        .edge("a", "m", "modules");
    assert_eq!(json_of("feature_overlap", json!({}), &g)["count"], 0);
}

#[specforge_test(
    behavior = "pe_query_feature_overlap",
    verify = "diamond topology correctly deduplicates"
)]
fn a_diamond_lists_each_feature_and_deliverable_once() {
    // d1 reaches f1 through j1 and j4; d2 and d3 through m1.
    let fo = json_of("feature_overlap", json!({}), &reach());
    let f1 = matrix_entry(&fo, "overlapping_features", "feature_id", "f1");
    assert_eq!(
        (
            f1["deliverable_ids"].clone(),
            f1["deliverable_count"].clone()
        ),
        (json!(["d1", "d2", "d3"]), json!(3))
    );
    let ids = ids_under(&fo, "overlapping_features", "feature_id");
    assert_eq!(ids, ["f1", "f2"]);
}

#[specforge_test(
    behavior = "product_new_query_correctness",
    verify = "feature overlap detected via both journey and module paths"
)]
fn overlap_is_found_through_journeys_and_through_modules() {
    let base = || {
        G::default()
            .n("f", "feature")
            .n("j1", "journey")
            .n("j2", "journey")
            .n("m1", "module")
            .n("m2", "module")
            .n("a", "deliverable")
            .n("b", "deliverable")
            .edge("j1", "f", "features")
            .edge("j2", "f", "features")
            .edge("m1", "f", "features")
            .edge("m2", "f", "features")
    };
    for g in [
        base()
            .edge("a", "j1", "journeys")
            .edge("b", "j2", "journeys"),
        base().edge("a", "m1", "modules").edge("b", "m2", "modules"),
        base()
            .edge("a", "j1", "journeys")
            .edge("b", "m2", "modules"),
    ] {
        let fo = json_of("feature_overlap", json!({}), &g);
        assert_eq!(
            fo["overlapping_features"][0]["deliverable_ids"],
            json!(["a", "b"]),
            "{fo}"
        );
    }
}
