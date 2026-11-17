//! `persona_features` and `channel_features`: the features a persona's or a channel's journeys reach.

use super::host::*;
use serde_json::json;
use specforge_test::prelude::*;

// ── persona and channel features ──────────────────────────────────────────

#[specforge_test(requirement = "FEATURE-093")]
fn a_personas_journey_gives_its_features() {
    let g = G::default()
        .n("p1", "persona")
        .n("j1", "journey")
        .n("f1", "feature")
        .n("f2", "feature")
        .edge("j1", "p1", "persona")
        .edge("j1", "f2", "features")
        .edge("j1", "f1", "features");
    let pf = json_of("persona_features", json!({"persona": "p1"}), &g);
    assert_eq!(
        pf,
        json!({"persona_id": "p1", "features": ["f1", "f2"], "via_journey_ids": ["j1"], "count": 2})
    );
}

#[specforge_test(requirement = "FEATURE-093")]
fn a_personas_journeys_give_each_feature_once() {
    let pf = json_of("persona_features", json!({"persona": "dev"}), &plan());
    assert_eq!(pf["features"], json!(["f1", "f2", "f3"]));
    assert_eq!(pf["via_journey_ids"], json!(["j1", "j2"]));
    assert_eq!(pf["count"], 3);
}

#[specforge_test(requirement = "FEATURE-093")]
fn a_persona_without_journeys_has_no_features() {
    let pf = json_of("persona_features", json!({"persona": "ops"}), &plan());
    assert_eq!(
        pf,
        json!({"persona_id": "ops", "features": [], "via_journey_ids": [], "count": 0})
    );
}

#[specforge_test(requirement = "FEATURE-093")]
fn a_mistyped_persona_is_not_found_with_the_nearest() {
    let error = run_in(
        &runtime(),
        "persona_features",
        json!({"persona": "dex"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "dev");
}

#[specforge_test(requirement = "FEATURE-095")]
fn persona_features_answers_its_payload() {
    let pf = json_of("persona_features", json!({"persona": "dev"}), &plan());
    let mut keys: Vec<&str> = pf.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["count", "features", "persona_id", "via_journey_ids"]);
    assert_eq!(
        human_of("persona_features", json!({"persona": "dev"}), &plan()),
        "Features for persona 'dev':\n  f1\n  f2\n  f3\n"
    );
}

#[specforge_test(requirement = "FEATURE-095")]
fn persona_features_of_a_mistyped_persona_suggests_the_nearest() {
    let out = run_in(
        &runtime(),
        "persona_features",
        json!({"persona": "ops2"}),
        &plan(),
        "human",
    );
    assert_eq!(out.exit, 1);
    assert_eq!(
        out.stderr,
        "error: persona 'ops2' not found\ndid you mean 'ops'?\n"
    );
}

#[specforge_test(requirement = "FEATURE-093")]
fn a_channels_journey_gives_its_features() {
    let g = G::default()
        .n("c1", "channel")
        .n("j1", "journey")
        .n("f1", "feature")
        .edge("j1", "c1", "channels")
        .edge("j1", "f1", "features");
    let cf = json_of("channel_features", json!({"channel": "c1"}), &g);
    assert_eq!(cf["features"], json!(["f1"]));
    assert_eq!(cf["count"], 1);
}

#[specforge_test(requirement = "FEATURE-093")]
fn a_channels_journeys_give_each_feature_once() {
    let cf = json_of("channel_features", json!({"channel": "cli"}), &plan());
    assert_eq!(cf["features"], json!(["f1", "f2", "f3"]));
    assert_eq!(cf["count"], 3);
}

#[specforge_test(requirement = "FEATURE-093")]
fn a_channel_without_journeys_has_no_features() {
    let g = plan().n("tui", "channel");
    let cf = json_of("channel_features", json!({"channel": "tui"}), &g);
    assert_eq!(
        cf,
        json!({"channel_id": "tui", "features": [], "via_journey_ids": [], "count": 0})
    );
}

#[specforge_test(requirement = "FEATURE-093")]
fn a_channel_whose_journeys_have_no_features_has_none() {
    let cf = json_of("channel_features", json!({"channel": "web"}), &plan());
    assert_eq!(cf["features"], json!([]));
    assert_eq!(cf["via_journey_ids"], json!(["j3"]));
}

#[specforge_test(requirement = "FEATURE-093")]
fn channel_features_name_the_journeys_they_came_through() {
    let cf = json_of("channel_features", json!({"channel": "cli"}), &plan());
    assert_eq!(cf["via_journey_ids"], json!(["j1", "j2"]));
}

#[specforge_test(requirement = "FEATURE-095")]
fn channel_features_answers_its_payload() {
    let cf = json_of("channel_features", json!({"channel": "cli"}), &plan());
    assert_eq!(
        cf,
        json!({"channel_id": "cli", "features": ["f1", "f2", "f3"],
            "via_journey_ids": ["j1", "j2"], "count": 3})
    );
}

#[specforge_test(requirement = "FEATURE-095")]
fn a_mistyped_channel_is_not_found_with_the_nearest() {
    let error = run_in(
        &runtime(),
        "channel_features",
        json!({"channel": "cly"}),
        &plan(),
        "json",
    )
    .error();
    assert_eq!(error["code"], "ENTITY_NOT_FOUND");
    assert_eq!(error["suggestion"], "cli");
}
