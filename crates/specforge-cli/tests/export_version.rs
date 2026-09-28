//! C6-02 acceptance: `--schema-version` negotiates a real same-major range.

use assert_cmd::Command;
use tempfile::TempDir;

fn specforge() -> Command {
    Command::new(env!("CARGO_BIN_EXE_specforge"))
}

fn seed(path: &std::path::Path) {
    std::fs::create_dir_all(path.join("src")).unwrap();
    std::fs::write(
        path.join("specforge.json"),
        r#"{"name":"v","spec_root":"src","extensions":["@specforge/product"]}"#,
    )
    .unwrap();
    std::fs::write(
        path.join("src/a.spec"),
        "type Widget {\n  id string @unique\n  verify unit \"widget valid\"\n}\n",
    )
    .unwrap();
}

#[test]
fn schema_version_within_same_major_is_accepted() {
    let tmp = TempDir::new().unwrap();
    seed(tmp.path());
    // Same major as the produced schema (1.x): accepted by the negotiated range.
    let out = specforge()
        .args([
            "export",
            tmp.path().join("src").to_str().unwrap(),
            "--format",
            "json",
            "--schema-version",
            "1.0.0",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "same-major request should pass: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"schema_version\":\"1.0.0\""),
        "label applied (compact)"
    );
}

#[test]
fn schema_version_other_major_is_rejected_with_range() {
    let tmp = TempDir::new().unwrap();
    seed(tmp.path());
    let out = specforge()
        .args([
            "export",
            tmp.path().join("src").to_str().unwrap(),
            "--format",
            "json",
            "--schema-version",
            "2.0.0",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("2.0.0"),
        "names the rejected version: {stderr}"
    );
    assert!(
        stderr.contains("incompatible major"),
        "explains the failure: {stderr}"
    );
}
