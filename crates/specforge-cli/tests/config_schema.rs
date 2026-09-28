//! C6-13 acceptance: the shipped config JSON schema covers every user-facing
//! config key, and the two shipped copies never drift.

use std::path::Path;

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
}

fn schema_path() -> std::path::PathBuf {
    repo_root().join("schema/specforge.schema.json")
}

#[test]
fn shipped_schema_copies_are_identical() {
    let main = std::fs::read_to_string(schema_path()).unwrap();
    let vscode = std::fs::read_to_string(
        repo_root().join("integrations/vscode/schemas/specforge.schema.json"),
    )
    .unwrap();
    assert_eq!(
        main, vscode,
        "schema/specforge.schema.json and integrations/vscode/schemas/specforge.schema.json drifted; edit both together"
    );
}

#[test]
fn schema_covers_project_config_keys() {
    let schema = schema_path().to_string_lossy().into_owned();
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&schema).unwrap())
        .expect("config schema is valid JSON");
    let props = doc["properties"].as_object().expect("properties object");

    // Typed on ProjectConfig and user-facing (C6-13: these were formally
    // invalid under additionalProperties:false before the fix).
    for key in ["inference", "registries"] {
        assert!(props.contains_key(key), "config schema is missing '{key}'");
    }
    let inference = &props["inference"];
    assert!(
        inference["properties"]["global"].is_object()
            && inference["properties"]["kinds"].is_object()
            && inference["properties"]["density_threshold"].is_object(),
        "inference def must mirror InferenceConfig (global, kinds, density_threshold)"
    );

    // The registry entry shape mirrors RegistryConfig.
    let registry = &doc["$defs"]["registry"];
    for field in ["alias", "url", "scope_filter", "default_registry"] {
        assert!(
            registry["properties"].get(field).is_some(),
            "registry def is missing RegistryConfig field '{field}'"
        );
    }
}

#[test]
fn specforge_json_with_inference_and_registries_loads() {
    // End-to-end: the compiler consumes both keys without treating them as
    // unknown config (they parse into ProjectConfig / registry client).
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path();
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("specforge.json"),
        r#"{
            "name": "parity",
            "spec_root": "src",
            "extensions": ["@specforge/product"],
            "inference": { "density_threshold": 0.5, "kinds": { "widget": "src/" } },
            "registries": [ { "alias": "local", "url": "http://127.0.0.1:1", "default_registry": true } ]
        }"#,
    )
    .unwrap();
    std::fs::write(
        root.join("src/a.spec"),
        "type Widget {\n  id string @unique\n}\n",
    )
    .unwrap();

    let bin = env!("CARGO_BIN_EXE_specforge");
    let out = std::process::Command::new(bin)
        .args(["check", root.join("src").to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "config with inference + registries must compile: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
