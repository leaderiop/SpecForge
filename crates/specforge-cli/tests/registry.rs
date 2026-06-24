use assert_cmd::Command;
use tempfile::TempDir;

fn specforge_cmd() -> Command {
    assert_cmd::cargo_bin_cmd!("specforge")
}

#[test]
fn add_invalid_specifier_fails() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .arg("add")
        .arg("") // empty specifier
        .arg("--path")
        .arg(dir.path())
        .assert()
        .failure();
}

#[test]
fn add_local_nonexistent_file_fails() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .arg("add")
        .arg("./nonexistent.wasm")
        .arg("--path")
        .arg(dir.path())
        .assert()
        .failure();
}

#[test]
fn add_local_file_succeeds() {
    let dir = TempDir::new().unwrap();

    // Create a minimal wasm file (valid magic bytes)
    let wasm = b"\x00asm\x01\x00\x00\x00";
    let wasm_path = dir.path().join("test-ext.wasm");
    std::fs::write(&wasm_path, wasm).unwrap();

    specforge_cmd()
        .arg("add")
        .arg(wasm_path.to_str().unwrap())
        .arg("--path")
        .arg(dir.path())
        .assert()
        .success();

    // Verify lock file was created
    assert!(dir.path().join("specforge.lock").exists());
}

#[test]
fn add_local_file_json_output() {
    let dir = TempDir::new().unwrap();

    let wasm = b"\x00asm\x01\x00\x00\x00";
    let wasm_path = dir.path().join("my-ext.wasm");
    std::fs::write(&wasm_path, wasm).unwrap();

    let output = specforge_cmd()
        .arg("add")
        .arg(wasm_path.to_str().unwrap())
        .arg("--path")
        .arg(dir.path())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["action"], "add");
    assert_eq!(json["source"], "local");
}

#[test]
fn publish_without_manifest_fails() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .arg("publish")
        .arg("--path")
        .arg(dir.path())
        .assert()
        .failure();
}

#[test]
fn publish_without_manifest_json_output() {
    let dir = TempDir::new().unwrap();
    let output = specforge_cmd()
        .arg("publish")
        .arg("--path")
        .arg(dir.path())
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["code"], "E-PUB-001");
}

#[test]
fn update_without_lockfile_fails() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .arg("update")
        .arg("--path")
        .arg(dir.path())
        .assert()
        .failure();
}

#[test]
fn logout_without_credentials_succeeds() {
    specforge_cmd()
        .arg("logout")
        .assert()
        .success();
}

#[test]
fn login_without_token_fails() {
    let dir = TempDir::new().unwrap();
    specforge_cmd()
        .arg("login")
        .arg("--path")
        .arg(dir.path())
        .assert()
        .failure();
}
