use assert_cmd::Command;
use specforge_test_macros::test as specforge_test;
use tempfile::TempDir;
use std::fs;

fn specforge_cmd() -> Command {
    assert_cmd::cargo_bin_cmd!("specforge")
}

// B:find_project_root — verify contract "requires/ensures consistency for project root discovery"
#[test]
#[specforge_test(behavior = "find_project_root", verify = "requires/ensures consistency for project root discovery")]
fn find_project_root_contract() {
    use specforge_common::find_project_root;

    // Requires: directory containing specforge.json (or ancestor does)
    // Ensures: correct root returned; None when not found
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("specforge.json"), "{}").unwrap();
    let child = dir.path().join("sub").join("deep");
    fs::create_dir_all(&child).unwrap();

    // From the deep child, should find the root with specforge.json
    let root = find_project_root(&child);
    assert_eq!(root.unwrap(), dir.path().canonicalize().unwrap(),
        "must find project root from nested directory");

    // From current dir, also works
    let direct = find_project_root(dir.path());
    assert_eq!(direct.unwrap(), dir.path().canonicalize().unwrap(),
        "must find project root from current directory");

    // Empty dir with no config
    let empty = TempDir::new().unwrap();
    let empty_child = empty.path().join("nothing");
    fs::create_dir_all(&empty_child).unwrap();
    let result = find_project_root(&empty_child);
    if let Some(found) = result {
        assert!(!found.starts_with(empty.path().canonicalize().unwrap()),
            "must not find root in empty hierarchy");
    }
}

// B:scaffold_new_project — verify contract "requires/ensures consistency for new project scaffolding"
#[test]
#[specforge_test(behavior = "scaffold_new_project", verify = "requires/ensures consistency for new project scaffolding")]
fn scaffold_new_project_contract() {
    // Requires: empty directory + project name
    // Ensures: specforge.json + spec/ directory created with valid content
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "contract-test"])
        .current_dir(dir.path())
        .assert()
        .success();

    // specforge.json exists and is valid
    let config_path = dir.path().join("specforge.json");
    assert!(config_path.exists(), "specforge.json must be created");
    let content = fs::read_to_string(&config_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content)
        .expect("specforge.json must be valid JSON");
    assert_eq!(json["name"], "contract-test", "name must match input");
    assert!(json["version"].is_string(), "version must be present");
    assert!(json["extensions"].is_array(), "extensions must be an array");
    assert!(json["spec_root"].is_string(), "spec_root must be present");

    // spec/ directory with starter file
    let spec_dir = dir.path().join("spec");
    assert!(spec_dir.exists(), "spec/ directory must be created");
    let starter = spec_dir.join("hello.spec");
    assert!(starter.exists(), "starter spec file must be created");
    assert!(!fs::read_to_string(&starter).unwrap().is_empty(), "starter must not be empty");
}

// B:non_interactive_init — verify contract "requires/ensures consistency for non-interactive init"
#[test]
#[specforge_test(behavior = "non_interactive_init", verify = "requires/ensures consistency for non-interactive init")]
fn non_interactive_init_contract() {
    // Requires: --name flag provided (no interactive prompts)
    // Ensures: project created without requiring stdin input
    let dir = TempDir::new().unwrap();

    let output = specforge_cmd()
        .args(["init", "--name", "ci-contract"])
        .current_dir(dir.path())
        .write_stdin("") // empty stdin — must not hang
        .output()
        .unwrap();

    assert!(output.status.success(), "must succeed without interactive input");
    assert!(dir.path().join("specforge.json").exists(), "config must be created");

    let content = fs::read_to_string(dir.path().join("specforge.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["name"], "ci-contract");
}

// B:graceful_zero_extension_init — verify contract "requires/ensures consistency for zero-extension init"
#[test]
#[specforge_test(behavior = "graceful_zero_extension_init", verify = "requires/ensures consistency for zero-extension init")]
fn graceful_zero_extension_init_contract() {
    // Requires: init with no --extensions flag
    // Ensures: specforge.json has empty extensions array, project still functional
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "zero-ext-contract"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = fs::read_to_string(dir.path().join("specforge.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["extensions"], serde_json::json!([]),
        "extensions must be empty array");

    // Project must still be functional: check + export succeed
    specforge_cmd()
        .args(["check", "--format", "json"])
        .arg(dir.path().join("spec"))
        .assert()
        .success();

    let export_output = specforge_cmd()
        .args(["export", "--format", "graph"])
        .arg(dir.path().join("spec"))
        .output()
        .unwrap();
    assert!(export_output.status.success(), "export must succeed with zero extensions");
    let stdout = String::from_utf8_lossy(&export_output.stdout);
    let graph: serde_json::Value = serde_json::from_str(&stdout)
        .expect("export must produce valid JSON");
    assert!(graph["nodes"].is_array(), "graph must have nodes array");
}
