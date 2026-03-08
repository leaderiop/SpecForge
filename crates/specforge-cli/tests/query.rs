use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn setup_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (path, content) in files {
        let full = dir.path().join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full, content).unwrap();
    }
    dir
}

fn specforge_cmd() -> Command {
    assert_cmd::cargo_bin_cmd!("specforge")
}

const SPEC_CONTENT: &str = r#"
behavior alpha "Alpha" { contract "first" }
behavior beta "Beta" { contract "second" }
feature gamma "Gamma" { behaviors [alpha, beta] }
invariant delta "Delta" { guarantee "always" }
"#;

#[test]
fn query_depth_0_returns_single_entity() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["query", "alpha", "--depth=0"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["id"], "alpha");
}

#[test]
fn query_depth_1_returns_neighbors() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["query", "gamma", "--depth=1"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    let ids: Vec<&str> = nodes.iter().map(|n| n["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"gamma"));
    assert!(ids.contains(&"alpha"));
    assert!(ids.contains(&"beta"));
}

#[test]
fn query_with_kind_filter() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["query", "gamma", "--depth=1", "--kind=behavior"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let nodes = parsed["nodes"].as_array().unwrap();
    for node in nodes {
        let kind = node["kind"].as_str().unwrap();
        // Root (gamma=feature) is always included + behavior nodes
        assert!(kind == "feature" || kind == "behavior", "unexpected kind: {}", kind);
    }
}

#[test]
fn query_nonexistent_entity_exits_one() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    specforge_cmd()
        .args(["query", "nonexistent", "--depth=1"])
        .arg("--path")
        .arg(dir.path())
        .assert()
        .code(1);
}
