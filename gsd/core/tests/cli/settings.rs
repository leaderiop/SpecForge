//! `specforge settings` over a project that loads the built `@gsd/core`: what the roster resolves to.

use std::collections::BTreeMap;

use crate::harness::project;

/// `name -> value` of every setting `specforge settings --format json` lists for `@gsd/core`.
fn listed(p: &crate::harness::Project, all: bool) -> BTreeMap<String, serde_json::Value> {
    let mut args = vec!["settings", "--format", "json", "--extension", "@gsd/core"];
    if all {
        args.push("--all");
    }
    let run = p.run(&args);
    assert_eq!(run.code, 0, "settings: {}", run.all());
    let json: serde_json::Value = serde_json::from_str(&run.stdout)
        .unwrap_or_else(|e| panic!("settings prints JSON ({e}): {}", run.all()));
    json["extensions"][0]["settings"]
        .as_array()
        .expect("settings")
        .iter()
        .map(|s| {
            (
                s["name"].as_str().expect("a name").to_string(),
                s["value"].clone(),
            )
        })
        .collect()
}

#[test]
fn specforge_settings_lists_gsd_core_defaults() {
    let p = project("empty");
    let defaults = listed(&p, false);
    assert_eq!(defaults.len(), 117, "settings with a default");
    for (name, value) in [
        ("mode", serde_json::json!("interactive")),
        ("granularity", serde_json::json!("standard")),
        ("model_profile", serde_json::json!("balanced")),
        ("resolve_model_ids", serde_json::json!("aliases")),
        ("workflow.inline_plan_threshold", serde_json::json!(2)),
        ("workflow.tdd_mode", serde_json::json!(false)),
        (
            "parallelization.max_concurrent_agents",
            serde_json::json!(3),
        ),
        (
            "effort.routing_tier_defaults.heavy",
            serde_json::json!("xhigh"),
        ),
        ("hooks.context_warning_threshold", serde_json::json!(35)),
    ] {
        assert_eq!(defaults.get(name), Some(&value), "{name}");
    }
    let all = listed(&p, true);
    assert_eq!(all.len(), 189, "177 keys and 12 families");
    assert!(all.contains_key("models.*") && all.contains_key("review.reviewer_instances.*.*"));
}

#[test]
fn a_family_key_outside_its_list_is_refused() {
    let p = project("empty");
    p.settings(serde_json::json!({ "granularities.fine": "coarse" }));
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "check: {}", check.all());
    assert!(
        check.all().contains("E086") && check.all().contains("planning, discuss"),
        "check: {}",
        check.all()
    );
}

#[test]
fn an_explicit_setting_beats_its_default() {
    let p = project("empty");
    p.settings(serde_json::json!({
        "workflow.tdd_mode": true,
        "models.planning": "opus",
        "model_profile_overrides.codex.opus": { "model": "gpt-5" },
    }));
    let check = p.run(&["check"]);
    assert_eq!(check.code, 0, "check: {}", check.all());
    let set = listed(&p, true);
    assert_eq!(set["workflow.tdd_mode"], serde_json::json!(true));
}
