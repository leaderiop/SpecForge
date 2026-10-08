//! `feature_dependents` (and `feature_impact`'s not-found): the features that depend on one.

use super::host::*;
use serde_json::json;
use specforge_test::prelude::*;

// ── feature dependents ────────────────────────────────────────────────────

#[specforge_test(
    behavior = "pe_query_feature_dependents",
    verify = "feature with dependent returns that dependent"
)]
fn a_features_dependent_is_listed() {
    let g = G::default()
        .n("f1", "feature")
        .n("f2", "feature")
        .edge("f2", "f1", "depends_on");
    let fd = json_of("feature_dependents", json!({"feature": "f1"}), &g);
    assert_eq!(
        fd,
        json!({"feature_id": "f1", "dependents": ["f2"], "count": 1})
    );
}

#[specforge_test(
    behavior = "pe_query_feature_dependents",
    verify = "feature with multiple dependents returns all sorted by ID"
)]
fn a_features_dependents_are_sorted_by_id() {
    // Declared in reverse order; one only relates to f1 (`features`).
    let g = G::default()
        .n("f1", "feature")
        .n("f2", "feature")
        .n("f3", "feature")
        .n("f4", "feature")
        .edge("f3", "f1", "depends_on")
        .edge("f2", "f1", "depends_on")
        .edge("f4", "f1", "features");
    let fd = json_of("feature_dependents", json!({"feature": "f1"}), &g);
    assert_eq!(fd["dependents"], json!(["f2", "f3"]));
    assert_eq!(fd["count"], 2);
}

#[specforge_test(
    behavior = "pe_query_feature_dependents",
    verify = "feature with no dependents returns empty list"
)]
fn a_feature_without_dependents_has_none() {
    let fd = json_of("feature_dependents", json!({"feature": "f2"}), &plan());
    assert_eq!(
        fd,
        json!({"feature_id": "f2", "dependents": [], "count": 0})
    );
}

#[specforge_test(
    behavior = "surface_feature_dependents",
    verify = "feature-dependents returns FeatureDependentPayload JSON"
)]
fn feature_dependents_answers_its_payload() {
    let fd = json_of("feature_dependents", json!({"feature": "f1"}), &plan());
    assert_eq!(
        fd,
        json!({"feature_id": "f1", "dependents": ["f2", "f3"], "count": 2})
    );
    assert_eq!(
        human_of("feature_dependents", json!({"feature": "f1"}), &plan()),
        "Features depending on 'f1':\n  f2\n  f3\n"
    );
}

#[specforge_test(
    behavior = "surface_feature_dependents",
    verify = "missing feature ID returns error with suggestion"
)]
fn a_mistyped_feature_is_not_found_with_the_nearest() {
    let runtime = runtime();
    let error = run_in(
        &runtime,
        "feature_dependents",
        json!({"feature": "f11"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "f1");
    // An entity of another kind is not a feature.
    let error = run_in(
        &runtime,
        "feature_dependents",
        json!({"feature": "ms1"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
}

#[specforge_test(
    behavior = "surface_feature_impact",
    verify = "missing feature ID returns error with suggestion"
)]
fn feature_impact_of_a_mistyped_feature_suggests_the_nearest() {
    let error = run_in(
        &runtime(),
        "feature_impact",
        json!({"feature": "f9"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "f1");
}
