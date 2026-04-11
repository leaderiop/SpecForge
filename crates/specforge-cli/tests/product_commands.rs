use assert_cmd::cargo_bin_cmd;
use std::fs;
use tempfile::TempDir;

fn setup_product_project() -> TempDir {
    let dir = TempDir::new().unwrap();

    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../extensions/product")
        .canonicalize()
        .unwrap();

    let config = serde_json::json!({
        "name": "test-project",
        "version": "0.1.0",
        "extensions": [manifest_dir.to_str().unwrap()]
    });
    fs::write(dir.path().join("specforge.json"), config.to_string()).unwrap();

    fs::write(
        dir.path().join("spec.spec"),
        r#"persona dev "Developer" {
    description "A software developer"
    status active
}

channel cli "CLI" {
    description "Command-line interface"
    status active
}

feature f1 "Core Feature" {
    status proposed
    priority high
}

feature f2 "Secondary Feature" {
    status done
    priority medium
    depends_on [f1]
}

journey j1 "Dev Flow" {
    persona dev
    channels [cli]
    features [f1, f2]
    description "Developer uses CLI"
}

module mod1 "Core Module" {
    features [f1]
}

milestone m1 "Launch" {
    status planned
    features [f1, f2]
    modules [mod1]
}

deliverable d1 "CLI App" {
    artifact_type cli
    status draft
    journeys [j1]
    modules [mod1]
    milestones [m1]
}

term spec "Specification" {
    definition "A formal description of system behavior"
}

release r1 "Initial Release" {
    version "1.0.0"
    status planned
    deliverables [d1]
    milestones [m1]
}
"#,
    )
    .unwrap();

    dir
}

#[test]
fn test_product_features_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "features", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 2);
}

#[test]
fn test_product_features_filter_status() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "features", "--path", dir.path().to_str().unwrap(), "--status", "proposed", "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
    assert_eq!(result["entities"][0]["id"], "f1");
}

#[test]
fn test_product_milestones_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "milestones", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
}

#[test]
fn test_product_milestone_completion() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "milestone-completion", "m1", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["milestone_id"], "m1");
    assert_eq!(result["total_features"], 2);
    assert_eq!(result["done_features"], 1); // f2 is done
}

#[test]
fn test_product_feature_impact() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "feature-impact", "f1", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["feature_id"], "f1");
    assert!(!result["referenced_by_journeys"].as_array().unwrap().is_empty());
    assert!(!result["referenced_by_milestones"].as_array().unwrap().is_empty());
    assert!(!result["referenced_by_modules"].as_array().unwrap().is_empty());
}

#[test]
fn test_product_feature_dependents() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "feature-dependents", "f1", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let deps = result.as_array().unwrap();
    assert!(deps.iter().any(|v| v == "f2"), "f2 depends on f1: {:?}", deps);
}

#[test]
fn test_product_health() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "health", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(result["score"]["overall"].as_f64().unwrap() > 0.0);
    assert!(!result["entity_counts"].as_array().unwrap().is_empty());
}

#[test]
fn test_product_modules_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "modules", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
}

#[test]
fn test_product_terms_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "terms", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
}

#[test]
fn test_product_releases_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "releases", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
}

#[test]
fn test_product_personas_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "personas", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
}

#[test]
fn test_product_channels_json() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "channels", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["total"], 1);
}

#[test]
fn test_product_journey_coverage() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "journey-coverage", "j1", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(result["journey_id"], "j1");
    assert_eq!(result["total_features"], 2);
    assert_eq!(result["covered_by_modules"], 1); // f1 is in mod1
}

#[test]
fn test_product_persona_features() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "persona-features", "dev", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let features = result.as_array().unwrap();
    assert_eq!(features.len(), 2); // f1 and f2 via journey j1
}

#[test]
fn test_product_channel_features() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "channel-features", "cli", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let features = result.as_array().unwrap();
    assert_eq!(features.len(), 2); // f1 and f2 via journey j1
}

#[test]
fn test_product_bulk_status() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "bulk-status", "--path", dir.path().to_str().unwrap(), "--format", "json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let arr = result.as_array().unwrap();
    // Should have entries for feature, milestone, deliverable, persona, channel, release
    assert!(arr.len() >= 4, "Expected at least 4 status-bearing kinds, got {}", arr.len());
}

#[test]
fn test_product_nonexistent_milestone_exits_one() {
    let dir = setup_product_project();
    let mut cmd = cargo_bin_cmd!("specforge");
    cmd.args(["product", "milestone-completion", "nonexistent", "--path", dir.path().to_str().unwrap()]);
    cmd.assert().failure();
}
