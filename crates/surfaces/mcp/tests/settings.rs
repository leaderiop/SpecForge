//! The MCP `specforge.settings` tool (ADR 0288).

use std::sync::Arc;

use serde_json::{Value, json};
use specforge_project::testing::{
    FOLLOWER_EXTENSION, SETTINGS_EXTENSION, follower_extension, settings_extension,
};
use specforge_wasm::testing::InProcessRuntime;

use crate::support::*;

#[specforge_test_macros::test(
    behavior = "load_extension_settings",
    verify = "settings lists each setting's type, value, origin, default, gates, raises and each secret's status"
)]
fn specforge_settings_replies_with_its_output_schema() {
    let runtime = Arc::new(
        InProcessRuntime::new()
            .with(settings_extension)
            .with(follower_extension),
    );
    let mut server = TestProject::new()
        .config(|c| c["settings"] = json!({"@t/s": {"profile": "strict"}}))
        .enabling(&[SETTINGS_EXTENSION, FOLLOWER_EXTENSION])
        .serve_in(runtime);
    let reply = call_tool(&mut server, "specforge.settings", json!({}));
    let document: Value = serde_json::from_str(&tool_text(&reply)).unwrap();

    // The reply is the document the output schema describes.
    let schema = specforge_mcp::tools::core_tool("specforge.settings")
        .unwrap()
        .output_schema()
        .expect("the tool declares an output schema");
    let violations = specforge_common::shape::violations(schema, &document);
    assert!(violations.is_empty(), "{violations:#?}\n{document}");

    let extension = &document["extensions"][0];
    assert_eq!(extension["extension"], SETTINGS_EXTENSION);
    let setting = |name: &str| {
        extension["settings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"] == name)
            .unwrap_or_else(|| panic!("{name} is listed: {extension}"))
            .clone()
    };
    let profile = setting("profile");
    assert_eq!(
        (&profile["origin"], &profile["value"]),
        (&json!("explicit"), &json!("strict"))
    );
    let flag = setting("flag");
    assert_eq!(flag["origin"], "preset");
    assert_eq!(flag["preset"], "profile = strict");
    assert_eq!(flag["gates"], json!(["t.s/W901", "t.s/W903"]));
    // `mode` has no value and no default: listed with --all only.
    assert!(
        extension["settings"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["name"] != "mode")
    );
    // A secret is listed by whether it is set, never by value.
    assert_eq!(extension["secrets"][0]["name"], "api_key");
    assert!(extension["secrets"][0]["set"].is_boolean());
    assert!(extension["secrets"][0].get("value").is_none());
    assert_eq!(
        document["raised"],
        json!([
            {"code": "t.f/W904", "severity": "error"},
            {"code": "t.s/W902", "severity": "error"},
        ])
    );
    assert_eq!(document["gated"], json!([]));

    let all = call_tool(
        &mut server,
        "specforge.settings",
        json!({"all": true, "extension": "@t/s"}),
    );
    let all: Value = serde_json::from_str(&tool_text(&all)).unwrap();
    assert!(
        all["extensions"][0]["settings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["name"] == "mode")
    );
    assert_eq!(all["extensions"].as_array().unwrap().len(), 1);
}
