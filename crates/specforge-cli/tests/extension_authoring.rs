use assert_cmd::Command;
use predicates::prelude::*;
use specforge_test_macros::test as specforge_test;
use std::fs;
use tempfile::TempDir;

fn specforge_cmd() -> Command {
    assert_cmd::cargo_bin_cmd!("specforge")
}

// ===============================================================
// Behavior: extension_scaffold_init
// ===============================================================

#[specforge_test(
    behavior = "extension_scaffold_init",
    verify = "specforge extension init creates manifest.json"
)]
#[test]
fn extension_init_creates_manifest_json() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["extension", "init", "--name", "test-ext", "--path"])
        .arg(dir.path())
        .assert()
        .success();

    let manifest_path = dir.path().join("test-ext").join("manifest.json");
    assert!(manifest_path.exists(), "manifest.json should be created");

    let content = fs::read_to_string(&manifest_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content)
        .expect("manifest.json should be valid JSON");

    assert_eq!(json["name"], "@local/test-ext");
    assert_eq!(json["version"], "0.1.0");
    assert_eq!(json["manifestVersion"], 2);
    assert!(json["wasmPath"].as_str().unwrap().contains("test_ext"));
    assert!(json["contributes"]["entities"].as_bool().unwrap());
}

#[specforge_test(
    behavior = "extension_scaffold_init",
    verify = "specforge extension init creates src/lib.rs with skeleton exports"
)]
#[test]
fn extension_init_creates_src_lib_rs() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["extension", "init", "--name", "my-ext", "--path"])
        .arg(dir.path())
        .assert()
        .success();

    let lib_path = dir.path().join("my-ext").join("src").join("lib.rs");
    assert!(lib_path.exists(), "src/lib.rs should be created");

    let content = fs::read_to_string(&lib_path).unwrap();
    assert!(
        content.contains("_start"),
        "src/lib.rs should contain _start export"
    );
    assert!(
        content.contains("no_mangle"),
        "src/lib.rs should contain #[no_mangle]"
    );
    assert!(
        content.contains("wasm32-wasip1"),
        "src/lib.rs should mention wasm32-wasip1 build target"
    );
}

#[specforge_test(
    behavior = "extension_scaffold_init",
    verify = "specforge extension init creates Cargo.toml with cdylib crate type"
)]
#[test]
fn extension_init_creates_cargo_toml() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["extension", "init", "--name", "cargo-ext", "--path"])
        .arg(dir.path())
        .assert()
        .success();

    let cargo_path = dir.path().join("cargo-ext").join("Cargo.toml");
    assert!(cargo_path.exists(), "Cargo.toml should be created");

    let content = fs::read_to_string(&cargo_path).unwrap();
    assert!(
        content.contains(r#"name = "cargo-ext""#),
        "Cargo.toml should have the extension name"
    );
    assert!(
        content.contains(r#"crate-type = ["cdylib"]"#),
        "Cargo.toml should specify cdylib crate type"
    );
    assert!(
        content.contains(r#"edition = "2024""#),
        "Cargo.toml should use edition 2024"
    );
}

#[specforge_test(
    behavior = "extension_scaffold_init",
    verify = "specforge extension init rejects when directory already exists"
)]
#[test]
fn extension_init_rejects_existing_directory() {
    let dir = TempDir::new().unwrap();
    let ext_dir = dir.path().join("existing-ext");
    fs::create_dir_all(&ext_dir).unwrap();

    specforge_cmd()
        .args(["extension", "init", "--name", "existing-ext", "--path"])
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[specforge_test(
    behavior = "extension_scaffold_init",
    verify = "specforge extension init --format=json outputs structured JSON"
)]
#[test]
fn extension_init_json_output() {
    let dir = TempDir::new().unwrap();

    let output = specforge_cmd()
        .args([
            "extension",
            "init",
            "--name",
            "json-ext",
            "--format",
            "json",
            "--path",
        ])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("--format=json should produce valid JSON");

    assert_eq!(json["name"], "json-ext");
    assert!(json["path"].as_str().unwrap().contains("json-ext"));
    let files = json["files"].as_array().unwrap();
    assert_eq!(files.len(), 3);
}

#[specforge_test(
    behavior = "extension_scaffold_init",
    verify = "specforge extension init uses default name when --name not provided"
)]
#[test]
fn extension_init_default_name() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["extension", "init", "--path"])
        .arg(dir.path())
        .assert()
        .success();

    let ext_dir = dir.path().join("my-extension");
    assert!(ext_dir.exists(), "should use default name 'my-extension'");
    assert!(ext_dir.join("manifest.json").exists());
}

// ===============================================================
// Behavior: extension_build_validate_structure
// ===============================================================

#[specforge_test(
    behavior = "extension_build_validate_structure",
    verify = "specforge extension build validates project structure exists"
)]
#[test]
fn extension_build_validates_structure() {
    let dir = TempDir::new().unwrap();

    // First create a valid extension scaffold
    specforge_cmd()
        .args(["extension", "init", "--name", "build-ext", "--path"])
        .arg(dir.path())
        .assert()
        .success();

    // Then validate it
    specforge_cmd()
        .args(["extension", "build", "--path"])
        .arg(dir.path().join("build-ext"))
        .assert()
        .success()
        .stdout(predicate::str::contains("validated"));
}

#[specforge_test(
    behavior = "extension_build_validate_structure",
    verify = "specforge extension build errors on missing Cargo.toml"
)]
#[test]
fn extension_build_errors_missing_cargo_toml() {
    let dir = TempDir::new().unwrap();
    // Create manifest.json but no Cargo.toml
    fs::write(dir.path().join("manifest.json"), "{}").unwrap();

    specforge_cmd()
        .args(["extension", "build", "--path"])
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("E040"))
        .stderr(predicate::str::contains("Cargo.toml"));
}

#[specforge_test(
    behavior = "extension_build_validate_structure",
    verify = "specforge extension build errors on missing manifest.json"
)]
#[test]
fn extension_build_errors_missing_manifest() {
    let dir = TempDir::new().unwrap();
    // Create Cargo.toml but no manifest.json
    fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();

    specforge_cmd()
        .args(["extension", "build", "--path"])
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("E040"))
        .stderr(predicate::str::contains("manifest.json"));
}

#[specforge_test(
    behavior = "extension_build_validate_structure",
    verify = "specforge extension build --format=json outputs structured error"
)]
#[test]
fn extension_build_json_error() {
    let dir = TempDir::new().unwrap();

    let output = specforge_cmd()
        .args(["extension", "build", "--format", "json", "--path"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("--format=json should produce valid JSON on error");
    assert_eq!(json["code"], "E040");
    assert_eq!(json["exit_code"], 1);
}

// ===============================================================
// Behavior: extension_validate_manifest
// ===============================================================

#[specforge_test(
    behavior = "extension_validate_manifest",
    verify = "specforge extension validate checks manifest against ManifestV2 schema"
)]
#[test]
fn extension_validate_valid_manifest() {
    let dir = TempDir::new().unwrap();

    // Create a valid manifest
    let manifest = serde_json::json!({
        "name": "@local/valid-ext",
        "version": "1.0.0",
        "manifestVersion": 2,
        "wasmPath": "target/wasm32-wasip1/release/valid_ext.wasm",
        "contributes": { "entities": true },
        "entityKinds": [],
        "edgeTypes": [],
        "fields": []
    });
    fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    specforge_cmd()
        .args(["extension", "validate", "--path"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[specforge_test(
    behavior = "extension_validate_manifest",
    verify = "specforge extension validate errors on invalid manifest JSON"
)]
#[test]
fn extension_validate_invalid_json() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("manifest.json"), "not valid json {{{").unwrap();

    specforge_cmd()
        .args(["extension", "validate", "--path"])
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("E030"));
}

#[specforge_test(
    behavior = "extension_validate_manifest",
    verify = "specforge extension validate errors on manifest with wrong manifestVersion"
)]
#[test]
fn extension_validate_wrong_manifest_version() {
    let dir = TempDir::new().unwrap();

    let manifest = serde_json::json!({
        "name": "@local/bad-version",
        "version": "1.0.0",
        "manifestVersion": 1,
        "wasmPath": "x.wasm"
    });
    fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    specforge_cmd()
        .args(["extension", "validate", "--path"])
        .arg(dir.path())
        .assert()
        .failure();
}

#[specforge_test(
    behavior = "extension_validate_manifest",
    verify = "specforge extension validate errors on missing manifest.json"
)]
#[test]
fn extension_validate_missing_manifest() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["extension", "validate", "--path"])
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("E040"));
}

#[specforge_test(
    behavior = "extension_validate_manifest",
    verify = "specforge extension validate --format=json outputs structured result for valid manifest"
)]
#[test]
fn extension_validate_json_output_valid() {
    let dir = TempDir::new().unwrap();

    let manifest = serde_json::json!({
        "name": "@local/json-valid",
        "version": "2.0.0",
        "manifestVersion": 2,
        "wasmPath": "ext.wasm"
    });
    fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let output = specforge_cmd()
        .args(["extension", "validate", "--format", "json", "--path"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["valid"], true);
    assert_eq!(json["name"], "@local/json-valid");
    assert_eq!(json["version"], "2.0.0");
}

#[specforge_test(
    behavior = "extension_validate_manifest",
    verify = "specforge extension validate --format=json outputs diagnostics for invalid manifest"
)]
#[test]
fn extension_validate_json_output_invalid() {
    let dir = TempDir::new().unwrap();

    let manifest = serde_json::json!({
        "name": "",
        "version": "1.0.0",
        "manifestVersion": 1,
        "wasmPath": ""
    });
    fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let output = specforge_cmd()
        .args(["extension", "validate", "--format", "json", "--path"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["valid"], false);
    let diagnostics = json["diagnostics"].as_array().unwrap();
    assert!(
        !diagnostics.is_empty(),
        "should have diagnostics for invalid manifest"
    );
}

// ===============================================================
// Behavior: extension_init_then_validate_roundtrip
// ===============================================================

#[specforge_test(
    behavior = "extension_init_then_validate_roundtrip",
    verify = "scaffolded extension passes validation"
)]
#[test]
fn extension_init_then_validate_roundtrip() {
    let dir = TempDir::new().unwrap();

    // Scaffold
    specforge_cmd()
        .args(["extension", "init", "--name", "roundtrip-ext", "--path"])
        .arg(dir.path())
        .assert()
        .success();

    // Validate the scaffolded manifest
    specforge_cmd()
        .args(["extension", "validate", "--path"])
        .arg(dir.path().join("roundtrip-ext"))
        .assert()
        .success();

    // Build (structure check) the scaffolded project
    specforge_cmd()
        .args(["extension", "build", "--path"])
        .arg(dir.path().join("roundtrip-ext"))
        .assert()
        .success();
}

// ===============================================================
// Contract tests
// ===============================================================

#[specforge_test(
    behavior = "extension_scaffold_init",
    verify = "contract: init creates exactly 3 files in a new directory"
)]
#[test]
fn contract_init_creates_three_files() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["extension", "init", "--name", "contract-ext", "--path"])
        .arg(dir.path())
        .assert()
        .success();

    let ext_dir = dir.path().join("contract-ext");
    assert!(ext_dir.join("manifest.json").exists());
    assert!(ext_dir.join("src/lib.rs").exists());
    assert!(ext_dir.join("Cargo.toml").exists());

    // Verify manifest.json is a valid ManifestV2
    let content = fs::read_to_string(ext_dir.join("manifest.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(manifest["manifestVersion"], 2);
    assert!(manifest["name"].as_str().unwrap().starts_with("@local/"));
}

#[specforge_test(
    behavior = "extension_build_validate_structure",
    verify = "contract: build requires both Cargo.toml and manifest.json"
)]
#[test]
fn contract_build_requires_both_files() {
    // Neither file
    let dir1 = TempDir::new().unwrap();
    specforge_cmd()
        .args(["extension", "build", "--path"])
        .arg(dir1.path())
        .assert()
        .failure();

    // Only Cargo.toml
    let dir2 = TempDir::new().unwrap();
    fs::write(dir2.path().join("Cargo.toml"), "[package]").unwrap();
    specforge_cmd()
        .args(["extension", "build", "--path"])
        .arg(dir2.path())
        .assert()
        .failure();

    // Only manifest.json
    let dir3 = TempDir::new().unwrap();
    fs::write(dir3.path().join("manifest.json"), "{}").unwrap();
    specforge_cmd()
        .args(["extension", "build", "--path"])
        .arg(dir3.path())
        .assert()
        .failure();

    // Both files present -> success
    let dir4 = TempDir::new().unwrap();
    fs::write(dir4.path().join("Cargo.toml"), "[package]").unwrap();
    fs::write(dir4.path().join("manifest.json"), "{}").unwrap();
    specforge_cmd()
        .args(["extension", "build", "--path"])
        .arg(dir4.path())
        .assert()
        .success();
}

#[specforge_test(
    behavior = "extension_validate_manifest",
    verify = "contract: validate returns exit 0 for valid manifest, exit 1 for invalid"
)]
#[test]
fn contract_validate_exit_codes() {
    // Valid manifest -> exit 0
    let dir1 = TempDir::new().unwrap();
    let valid = serde_json::json!({
        "name": "@local/valid",
        "version": "1.0.0",
        "manifestVersion": 2,
        "wasmPath": "x.wasm"
    });
    fs::write(
        dir1.path().join("manifest.json"),
        serde_json::to_string_pretty(&valid).unwrap(),
    )
    .unwrap();
    specforge_cmd()
        .args(["extension", "validate", "--path"])
        .arg(dir1.path())
        .assert()
        .success();

    // Invalid manifest (manifestVersion != 2) -> exit 1
    let dir2 = TempDir::new().unwrap();
    let invalid = serde_json::json!({
        "name": "@local/invalid",
        "version": "1.0.0",
        "manifestVersion": 99,
        "wasmPath": "x.wasm"
    });
    fs::write(
        dir2.path().join("manifest.json"),
        serde_json::to_string_pretty(&invalid).unwrap(),
    )
    .unwrap();
    specforge_cmd()
        .args(["extension", "validate", "--path"])
        .arg(dir2.path())
        .assert()
        .failure();

    // Missing manifest -> exit 1
    let dir3 = TempDir::new().unwrap();
    specforge_cmd()
        .args(["extension", "validate", "--path"])
        .arg(dir3.path())
        .assert()
        .failure();
}
