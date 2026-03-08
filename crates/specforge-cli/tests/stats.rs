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

#[test]
fn stats_reports_entity_and_edge_counts() {
    let dir = setup_project(&[("main.spec", r#"
behavior alpha "A" { contract "first" }
behavior beta "B" { contract "second" }
feature gamma "G" { behaviors [alpha, beta] }
"#)]);

    let output = specforge_cmd()
        .arg("stats")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("3"), "should report 3 entities: {}", stdout);
    assert!(stdout.contains("2"), "should report 2 edges: {}", stdout);
}

#[test]
fn stats_on_empty_project() {
    let dir = setup_project(&[("main.spec", "")]);

    let output = specforge_cmd()
        .arg("stats")
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0"), "should report 0 entities: {}", stdout);
}

#[test]
fn stats_json_format() {
    let dir = setup_project(&[("main.spec", r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha] }
"#)]);

    let output = specforge_cmd()
        .args(["stats", "--format=json"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON: {}\noutput: {}", e, stdout));

    assert_eq!(parsed["total_entities"], 2);
    assert_eq!(parsed["total_edges"], 1);
}
