//! `milestone_completion`: a milestone's done and proven share of its features.

use super::host::*;
use serde_json::{json, Value};
use specforge_test::prelude::*;

// ── milestone completion ──────────────────────────────────────────────────

#[specforge_test(
    behavior = "pe_query_milestone_completion",
    verify = "milestone with all features status=done returns ratio 1.0"
)]
fn a_milestone_whose_features_are_all_done_is_complete() {
    let mc = json_of("milestone_completion", json!({"milestone": "ms2"}), &plan());
    assert_eq!(mc["completion_ratio"], 1.0);
    assert_eq!(mc["done_features"], json!(["f1"]));
}

#[specforge_test(
    behavior = "pe_query_milestone_completion",
    verify = "milestone with no done features returns ratio 0.0"
)]
fn a_milestone_with_no_done_feature_is_at_zero() {
    let g = plan()
        .node("ms4", "milestone", json!({"features": ["f2", "f3"]}))
        .edge("ms4", "f2", "features")
        .edge("ms4", "f3", "features");
    let mc = json_of("milestone_completion", json!({"milestone": "ms4"}), &g);
    assert_eq!(
        (mc["done_count"].clone(), mc["total_features"].clone()),
        (json!(0), json!(2))
    );
    assert_eq!(mc["completion_ratio"], 0.0);
    assert_eq!(mc["done_features"], json!([]));
}

#[specforge_test(
    behavior = "pe_query_milestone_completion",
    verify = "empty milestone returns ratio 0.0 with zero features"
)]
fn an_empty_milestone_is_at_zero_without_dividing_by_zero() {
    let mc = json_of("milestone_completion", json!({"milestone": "ms3"}), &plan());
    assert_eq!(
        mc,
        json!({"milestone_id": "ms3", "total_features": 0, "done_count": 0,
            "completion_ratio": 0.0, "done_features": [], "evidence": {"state": "none"}})
    );
}

#[specforge_test(
    behavior = "pe_query_milestone_completion",
    verify = "milestone with mix of done and non-done features returns partial ratio"
)]
fn a_milestone_half_done_is_at_one_half() {
    let mc = json_of("milestone_completion", json!({"milestone": "ms1"}), &plan());
    assert_eq!(mc["done_count"], 1);
    assert_eq!(mc["total_features"], 2);
    assert_eq!(mc["completion_ratio"], 0.5);
}

#[specforge_test(
    behavior = "pe_query_milestone_completion",
    verify = "milestone completion is deterministic across repeated queries"
)]
fn milestone_completion_is_deterministic() {
    let runtime = runtime();
    let g = plan();
    let first = run_in(
        &runtime,
        "milestone_completion",
        json!({"milestone": "ms1"}),
        &g,
        "json",
    );
    for _ in 0..3 {
        let again = run_in(
            &runtime,
            "milestone_completion",
            json!({"milestone": "ms1"}),
            &g,
            "json",
        );
        assert_eq!(again.stdout, first.stdout);
    }
}

#[specforge_test(
    behavior = "surface_milestone_completion",
    verify = "milestone-completion returns MilestoneCompletionPayload JSON"
)]
fn milestone_completion_answers_its_payload() {
    let mc = json_of("milestone_completion", json!({"milestone": "ms1"}), &plan());
    let mut keys: Vec<&str> = mc.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "completion_ratio",
            "done_count",
            "done_features",
            "evidence",
            "milestone_id",
            "total_features"
        ]
    );
    assert_eq!(mc["milestone_id"], "ms1");
}

/// `plan()` with behaviors implementing its features: `b1` implements `f1`
/// and `f2`, `b2` implements `f2`; journeys naming features implement none.
fn implemented() -> G {
    plan()
        .n("b1", "behavior")
        .n("b2", "behavior")
        .edge("b1", "f1", "features")
        .edge("b1", "f2", "features")
        .edge("b2", "f2", "features")
}

/// Recorded evidence: `b1` proven (2/2), `b2` with one of two obligations.
fn recorded() -> Value {
    json!({"state": "recorded", "entities": {
        "b1": {"obligations": 2, "proven": 2, "failing": 0},
        "b2": {"obligations": 2, "proven": 1, "failing": 0},
    }})
}

#[specforge_test(
    behavior = "pe_query_milestone_completion",
    verify = "a milestone's completion reports the features the recorded tests prove beside the done ones"
)]
#[specforge_test(
    type = "FeatureEvidence",
    verify = "a feature is proven when at least one behavior implements it and every one is proven"
)]
fn milestone_completion_reports_what_the_recorded_tests_prove() {
    let out = run_with(
        &runtime(),
        "milestone_completion",
        json!({"milestone": "ms1"}),
        &implemented(),
        "json",
        Some(recorded()),
    );
    let mc = out.json();
    assert_eq!(mc["done_count"], 1, "the declared count is the status's");
    assert_eq!(mc["evidence"], json!({"state": "recorded"}));
    assert_eq!(mc["proven_count"], 1);
    assert_eq!(mc["proven_ratio"], 0.5);
    assert_eq!(mc["proven_features"], json!(["f1"]));
    assert_eq!(
        mc["feature_evidence"][1],
        json!({"feature_id": "f2", "behaviors": 2, "proven_behaviors": 1,
            "obligations": 4, "proven_obligations": 3, "failing": 0, "proven": false})
    );
}

#[specforge_test(
    behavior = "surface_milestone_completion",
    verify = "human format shows the proven share and each feature's evidence beside its status"
)]
fn milestone_completion_shows_the_evidence_of_each_feature() {
    let out = run_with(
        &runtime(),
        "milestone_completion",
        json!({"milestone": "ms1"}),
        &implemented(),
        "human",
        Some(recorded()),
    );
    assert_eq!(
        out.stdout,
        "Milestone: ms1 (active)\nCompletion: 50% (1/2 features done)\n\
         Evidence:   50% (1/2 features proven by recorded tests)\n\
         \x20 f1 [done] proven 1/1 behaviors proven (2/2 obligations)\n\
         \x20 f2 [in_progress] unproven 1/2 behaviors proven (3/4 obligations)\n"
    );
}

#[specforge_test(
    behavior = "pe_query_milestone_completion",
    verify = "without a recorded report the completion says no evidence is recorded, and an unreadable one says why"
)]
fn milestone_completion_without_evidence_says_so() {
    let none = json_of(
        "milestone_completion",
        json!({"milestone": "ms1"}),
        &implemented(),
    );
    assert_eq!(none["evidence"], json!({"state": "none"}));
    assert!(none.get("proven_count").is_none());
    let unreadable = run_with(
        &runtime(),
        "milestone_completion",
        json!({"milestone": "ms1"}),
        &implemented(),
        "human",
        Some(json!({"state": "unreadable", "reason": "invalid test results x"})),
    );
    assert!(
        unreadable
            .stdout
            .contains("Evidence:   unreadable: invalid test results x\n"),
        "{}",
        unreadable.stdout
    );
}

#[specforge_test(
    behavior = "surface_milestone_completion",
    verify = "missing milestone ID returns error with suggestion"
)]
fn a_mistyped_milestone_is_not_found_with_the_nearest() {
    let error = run_in(
        &runtime(),
        "milestone_completion",
        json!({"milestone": "ms9"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["entity_id"], "ms9");
    assert_eq!(error["suggestion"], "ms1");
}

#[specforge_test(
    behavior = "surface_milestone_completion",
    verify = "human format shows ratio as percentage"
)]
fn milestone_completion_shows_a_percentage() {
    let out = human_of("milestone_completion", json!({"milestone": "ms1"}), &plan());
    assert_eq!(
        out,
        "Milestone: ms1 (active)\nCompletion: 50% (1/2 features done)\n\
         Evidence:   none recorded (run `specforge collect`)\n  f1 [done]\n  f2 [in_progress]\n"
    );
}

#[specforge_test(
    behavior = "surface_milestone_completion",
    verify = "exit code 0 on success, 1 on error"
)]
fn milestone_completion_exits_zero_or_one() {
    let runtime = runtime();
    let g = plan();
    for format in ["human", "json"] {
        let ok = run_in(
            &runtime,
            "milestone_completion",
            json!({"milestone": "ms1"}),
            &g,
            format,
        );
        assert_eq!(ok.exit, 0);
        let missing = run_in(
            &runtime,
            "milestone_completion",
            json!({"milestone": "x"}),
            &g,
            format,
        );
        assert_eq!(missing.exit, 1);
        // Not a milestone, though an entity.
        let other = run_in(
            &runtime,
            "milestone_completion",
            json!({"milestone": "f1"}),
            &g,
            format,
        );
        assert_eq!(other.exit, 1);
    }
}
