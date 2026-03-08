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
"#;

#[test]
fn trace_produces_upstream_and_downstream_json() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    let output = specforge_cmd()
        .args(["trace", "alpha"])
        .arg("--path")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));

    assert_eq!(parsed["entity_id"], "alpha");
    assert!(parsed["upstream"].is_array());
    assert!(parsed["downstream"].is_array());
    assert!(parsed["schema_version"].is_string());

    // alpha has upstream = gamma (gamma -> alpha via behaviors)
    let upstream = parsed["upstream"].as_array().unwrap();
    assert!(upstream.iter().any(|l| l["entity_id"] == "gamma"));
}

#[test]
fn trace_nonexistent_entity_exits_one() {
    let dir = setup_project(&[("main.spec", SPEC_CONTENT)]);

    specforge_cmd()
        .args(["trace", "nonexistent"])
        .arg("--path")
        .arg(dir.path())
        .assert()
        .code(1);
}
