//! `journey_coverage`: a journey's done features, and the rest uncovered.

use super::host::*;
use serde_json::json;
use specforge_test::prelude::*;

// ── journey coverage ──────────────────────────────────────────────────────

#[specforge_test(
    behavior = "pe_query_journey_coverage",
    verify = "journey with all features status=done returns full coverage"
)]
fn a_journey_whose_features_are_done_is_covered() {
    let g = plan().n("j4", "journey").edge("j4", "f1", "features");
    let jc = json_of("journey_coverage", json!({"journey": "j4"}), &g);
    assert_eq!(jc["covered_count"], 1);
    assert_eq!(jc["total_features"], 1);
    assert_eq!(jc["uncovered_features"], json!([]));
}

#[specforge_test(
    behavior = "pe_query_journey_coverage",
    verify = "journey with uncovered features lists them"
)]
fn a_journeys_features_not_done_are_uncovered() {
    let jc = json_of("journey_coverage", json!({"journey": "j1"}), &plan());
    assert_eq!(jc["covered_count"], 1);
    assert_eq!(jc["uncovered_features"], json!(["f2", "f3"]));
}

#[specforge_test(
    behavior = "pe_query_journey_coverage",
    verify = "journey with zero features returns empty coverage"
)]
fn a_journey_without_features_has_empty_coverage() {
    let jc = json_of("journey_coverage", json!({"journey": "j3"}), &plan());
    assert_eq!(
        jc,
        json!({"journey_id": "j3", "total_features": 0, "covered_count": 0, "uncovered_features": []})
    );
}

#[specforge_test(
    behavior = "pe_query_journey_coverage",
    verify = "features without status field are treated as uncovered"
)]
fn a_feature_without_a_status_is_uncovered() {
    let g = plan().n("j4", "journey").edge("j4", "f3", "features");
    let jc = json_of("journey_coverage", json!({"journey": "j4"}), &g);
    assert_eq!(jc["covered_count"], 0);
    assert_eq!(jc["uncovered_features"], json!(["f3"]));
}

#[specforge_test(
    behavior = "pe_query_journey_coverage",
    verify = "journey coverage is deterministic across repeated queries"
)]
fn journey_coverage_is_deterministic() {
    let runtime = runtime();
    let g = plan();
    let first = run_in(
        &runtime,
        "journey_coverage",
        json!({"journey": "j1"}),
        &g,
        "json",
    );
    for _ in 0..3 {
        let again = run_in(
            &runtime,
            "journey_coverage",
            json!({"journey": "j1"}),
            &g,
            "json",
        );
        assert_eq!(again.stdout, first.stdout);
    }
}

#[specforge_test(
    behavior = "surface_journey_coverage",
    verify = "journey-coverage returns JourneyCoveragePayload JSON"
)]
fn journey_coverage_answers_its_payload() {
    let jc = json_of("journey_coverage", json!({"journey": "j1"}), &plan());
    assert_eq!(
        jc,
        json!({"journey_id": "j1", "total_features": 3, "covered_count": 1,
            "uncovered_features": ["f2", "f3"]})
    );
    // Its human layout: covered/total and the uncovered list.
    assert_eq!(
        human_of("journey_coverage", json!({"journey": "j1"}), &plan()),
        "Journey: j1 (persona: dev)\nCoverage: 33% (1/3 features done)\nUncovered:\n  f2\n  f3\n"
    );
}

#[specforge_test(
    behavior = "surface_journey_coverage",
    verify = "missing journey ID returns error with suggestion"
)]
fn a_mistyped_journey_is_not_found_with_the_nearest() {
    let error = run_in(
        &runtime(),
        "journey_coverage",
        json!({"journey": "jj1"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "j1");
}

#[specforge_test(
    behavior = "surface_journey_coverage",
    verify = "exit code 0 on success, 1 on error"
)]
fn journey_coverage_exits_zero_or_one() {
    let runtime = runtime();
    let g = plan();
    assert_eq!(
        run_in(
            &runtime,
            "journey_coverage",
            json!({"journey": "j1"}),
            &g,
            "human"
        )
        .exit,
        0
    );
    assert_eq!(
        run_in(
            &runtime,
            "journey_coverage",
            json!({"journey": "nope"}),
            &g,
            "human"
        )
        .exit,
        1
    );
}
