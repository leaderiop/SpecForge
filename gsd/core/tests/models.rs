//! The model table against the vendored gsd-core catalog and against what the preset declares.

use gsd_core::agents::roster::AGENTS;
use gsd_core::build;
use gsd_core::settings::models::{MODELS, PROFILES, tier};
use serde_json::Value;
use specforge_extension_sdk::testing::MockHost;

const CATALOG: &str = include_str!(gsd_kit::snapshot!("model-catalog.json"));

#[test]
fn the_model_table_is_the_catalogs() {
    let catalog: Value = serde_json::from_str(CATALOG).expect("the catalog");
    let agents = catalog["agents"].as_object().expect("agents");
    assert_eq!(MODELS.len(), 15);
    for (id, aliases, routing) in MODELS {
        let row = &agents[*id];
        assert_eq!(row["golden"], aliases[0], "{id} quality");
        assert_eq!(row["balanced"], aliases[1], "{id} balanced");
        assert_eq!(row["budget"], aliases[2], "{id} budget");
        assert_eq!(row["routingTier"], *routing, "{id} tier");
    }
    // the roster and the table list the same agents, in the same order
    let roster: Vec<&str> = AGENTS.iter().map(|a| a.id).collect();
    let table: Vec<&str> = MODELS.iter().map(|m| m.0).collect();
    assert_eq!(roster, table);
    // the tiers are the catalog's adaptive map
    for (host_tier, alias) in catalog["adaptiveTierMap"].as_object().expect("map") {
        assert_eq!(tier(alias.as_str().expect("alias")), host_tier);
    }
}

#[test]
fn the_preset_sets_every_agent_for_every_profile_but_inherit() {
    let host = MockHost::new(build());
    let text = host.describe_json("settings").expect("settings");
    let described: Value = serde_json::from_str(&text).expect("JSON");
    let preset = described["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|i| i["kind"] == "preset" && i["source"] == "model_profile")
        .expect("a model_profile preset");
    let values = preset["values"].as_object().expect("values");
    let mut names: Vec<&str> = values.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(names, ["adaptive", "balanced", "budget", "quality"]);
    for (profile, column) in PROFILES {
        let set = &values[*profile]["settings"];
        for (id, aliases, routing) in MODELS {
            let want = match column {
                Some(i) => tier(aliases[*i]),
                None => routing,
            };
            assert_eq!(set[format!("model_overrides.{id}")], want, "{profile} {id}");
        }
    }
}
