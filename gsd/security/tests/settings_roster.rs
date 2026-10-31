//! The four settings of `@gsd/security` against the vendored gsd-core 1.19.0 snapshot, and the auditor's model and
//! variant presets that follow `@gsd/core`.

use gsd_security::build;
use serde_json::Value;
use specforge_extension_sdk::testing::MockHost;

const CAPABILITIES: &str = include_str!(gsd_kit::snapshot!("capabilities.json"));
const DEFAULTS: &str = include_str!(gsd_kit::snapshot!("config-defaults.manifest.json"));
const CATALOG: &str = include_str!(gsd_kit::snapshot!("model-catalog.json"));

fn items() -> Vec<Value> {
    let host = MockHost::new(build());
    let text = host.describe_json("settings").expect("a settings category");
    let described: Value = serde_json::from_str(&text).expect("JSON");
    described["items"].as_array().expect("items").clone()
}

fn setting(items: &[Value], name: &str) -> Value {
    items
        .iter()
        .find(|i| i["kind"] == "setting" && i["name"] == name)
        .unwrap_or_else(|| panic!("no setting {name}"))
        .clone()
}

fn preset(items: &[Value], source: &str) -> Value {
    items
        .iter()
        .find(|i| i["kind"] == "preset" && i["source"] == source)
        .unwrap_or_else(|| panic!("no preset following {source}"))
        .clone()
}

#[test]
fn the_security_keys_match_gsd_core() {
    let items = items();
    let caps: Value = serde_json::from_str(CAPABILITIES).expect("capabilities");
    let defaults: Value = serde_json::from_str(DEFAULTS).expect("defaults");
    // exactly the capability's config keys, plus the core manifest's `security.injection_blocking`
    let mut want: Vec<String> = caps
        .as_object()
        .expect("object")
        .iter()
        .filter(|(_, v)| v["capability"] == "security")
        .map(|(k, _)| k.clone())
        .collect();
    want.push("security.injection_blocking".to_string());
    want.sort();
    let mut declared: Vec<String> = items
        .iter()
        .filter(|i| i["kind"] == "setting")
        .map(|i| i["name"].as_str().expect("name").to_string())
        // `workflow.text_mode` is core's, mirrored for the prompt (`settings.rs`)
        .filter(|n| n.starts_with("workflow.") || n.starts_with("security."))
        .filter(|n| n != "workflow.text_mode")
        .collect();
    declared.sort();
    assert_eq!(declared, want);

    assert_eq!(
        setting(&items, "workflow.security_enforcement")["default"],
        true
    );
    assert_eq!(defaults["workflow"]["security_enforcement"], true);
    let asvs = setting(&items, "workflow.security_asvs_level");
    assert_eq!(asvs["field_type"], "integer");
    assert_eq!(
        (asvs["minimum"].as_i64(), asvs["maximum"].as_i64()),
        (Some(1), Some(3))
    );
    assert_eq!(asvs["default"], defaults["workflow"]["security_asvs_level"]);
    let block_on = setting(&items, "workflow.security_block_on");
    assert_eq!(
        block_on["enum_values"],
        caps["workflow.security_block_on"]["values"]
    );
    assert_eq!(block_on["default"], "high");
    let blocking = setting(&items, "security.injection_blocking");
    assert_eq!(blocking["field_type"], "bool");
    assert_eq!(
        blocking["default"],
        defaults["security"]["injection_blocking"]
    );
}

#[test]
fn workflow_security_enforcement_gates_w301() {
    let enforcement = setting(&items(), "workflow.security_enforcement");
    assert_eq!(enforcement["gates"], serde_json::json!(["W301"]));
}

#[test]
fn the_auditor_model_follows_model_profile() {
    let catalog: Value = serde_json::from_str(CATALOG).expect("catalog");
    let row = &catalog["agents"]["gsd-security-auditor"];
    let items = items();
    let values = preset(&items, "@gsd/core:model_profile")["values"].clone();
    let key = "model_overrides.gsd-security-auditor";
    let tier = |alias: &Value| match alias.as_str() {
        Some("opus") => "heavy",
        Some("haiku") => "light",
        _ => "standard",
    };
    assert_eq!(values["quality"]["settings"][key], tier(&row["golden"]));
    assert_eq!(values["balanced"]["settings"][key], tier(&row["balanced"]));
    assert_eq!(values["budget"]["settings"][key], tier(&row["budget"]));
    assert_eq!(values["adaptive"]["settings"][key], row["routingTier"]);
    assert_eq!(values["quality"]["settings"][key], "heavy");
    assert_eq!(values["balanced"]["settings"][key], "standard");
    // `inherit` presets nothing: no model line is written
    assert!(values.get("inherit").is_none(), "{values}");
    assert!(setting(&items, key)["default"].is_null());
}

#[test]
fn the_variant_follows_compact_content() {
    let items = items();
    let variant = setting(&items, "specforge.agent_variant");
    assert_eq!(
        variant["enum_values"],
        serde_json::json!(["full", "compact"])
    );
    assert_eq!(variant["default"], "full");
    let values = preset(&items, "@gsd/core:workflow.compact_content")["values"].clone();
    assert_eq!(
        values["true"]["settings"]["specforge.agent_variant"],
        "compact"
    );
}
