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
    assert_eq!(defaults.len(), 122, "settings with a default");
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
    assert_eq!(all.len(), 194, "182 keys and 12 families");
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

/// The parsed `specforge settings --format json` of the project.
fn resolved(p: &crate::harness::Project, env: &[(&str, &str)]) -> serde_json::Value {
    let run = p.run_with(&["settings", "--format", "json"], env);
    assert_eq!(run.code, 0, "settings: {}", run.all());
    serde_json::from_str(&run.stdout).unwrap_or_else(|e| panic!("JSON ({e}): {}", run.all()))
}

fn strings(value: &serde_json::Value) -> Vec<&str> {
    value
        .as_array()
        .expect("a list")
        .iter()
        .map(|v| v.as_str().expect("a string"))
        .collect()
}

fn raised_to_error(resolved: &serde_json::Value) -> Vec<&str> {
    resolved["raised"]
        .as_array()
        .expect("raised")
        .iter()
        .filter(|r| r["severity"] == "error")
        .map(|r| r["code"].as_str().expect("a code"))
        .collect()
}

#[test]
fn gsd_strict_raises_w103_to_an_error() {
    let p = project("empty");
    assert_eq!(raised_to_error(&resolved(&p, &[])), Vec::<&str>::new());
    p.settings(serde_json::json!({ "profile": "gsd-strict" }));
    let strict = resolved(&p, &[]);
    assert!(raised_to_error(&strict).contains(&"gsd/W103"), "{strict}");
    p.settings(serde_json::json!({ "profile": "gsd-plus" }));
    assert!(raised_to_error(&resolved(&p, &[])).contains(&"gsd/W103"));
}

#[test]
fn specforge_task_commits_off_gates_w407() {
    let p = project("empty");
    let gated = |p: &crate::harness::Project| {
        let r = resolved(p, &[]);
        let all = strings(&r["gated"]);
        ["gsd/W407", "gsd/E515", "gsd/E516", "gsd/E517"].map(|code| all.contains(&code))
    };
    assert_eq!(gated(&p), [true; 4], "gsd-compat gates all four");
    p.settings(serde_json::json!({ "profile": "gsd-strict" }));
    assert_eq!(
        gated(&p),
        [false, true, true, true],
        "strict switches on task commits"
    );
    p.settings(serde_json::json!({ "profile": "gsd-plus" }));
    assert_eq!(gated(&p), [false; 4], "plus switches on all four");
    p.settings(serde_json::json!({ "profile": "gsd-plus", "specforge.task_commits": false }));
    assert_eq!(
        gated(&p),
        [true, false, false, false],
        "an explicit setting beats the profile"
    );
}

#[test]
fn a_secret_is_never_listed_with_its_value() {
    let p = project("empty");
    let listing = p.run_with(&["settings"], &[("BRAVE_API_KEY", "s3cr3t-value")]);
    assert!(!listing.all().contains("s3cr3t-value"), "{}", listing.all());
    let json = resolved(&p, &[("BRAVE_API_KEY", "s3cr3t-value")]);
    assert!(!json.to_string().contains("s3cr3t-value"));
    let secrets = json["extensions"][0]["secrets"]
        .as_array()
        .expect("secrets");
    assert_eq!(secrets.len(), 7);
    let set: Vec<&str> = secrets
        .iter()
        .filter(|s| s["set"] == true)
        .map(|s| s["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(set, ["brave_search_key"]);
    // and a key is refused in the file itself
    p.settings(serde_json::json!({ "brave_search_key": "s3cr3t-value" }));
    let check = p.run(&["check"]);
    assert_eq!(check.code, 1, "check: {}", check.all());
    assert!(check.all().contains("E087"), "check: {}", check.all());
}
