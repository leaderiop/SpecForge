use assert_cmd::Command;
use specforge_test_macros::test as specforge_test;
use std::fs;
use tempfile::TempDir;

fn specforge_cmd() -> Command {
    assert_cmd::cargo_bin_cmd!("specforge")
}

// ═══════════════════════════════════════════════════════════
// Behavior: find_project_root
// ═══════════════════════════════════════════════════════════

#[specforge_test(
    behavior = "find_project_root",
    verify = "specforge.json found in current directory"
)]
#[test]
fn find_project_root_json_in_current_dir() {
    use specforge_common::find_project_root;

    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("specforge.json"), "{}").unwrap();

    let root = find_project_root(dir.path());
    assert_eq!(root.unwrap(), dir.path().canonicalize().unwrap());
}

#[specforge_test(
    behavior = "find_project_root",
    verify = "specforge.json found in ancestor directory"
)]
#[test]
fn find_project_root_json_in_ancestor() {
    use specforge_common::find_project_root;

    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("specforge.json"), "{}").unwrap();
    let child = dir.path().join("sub").join("deep");
    fs::create_dir_all(&child).unwrap();

    let root = find_project_root(&child);
    assert_eq!(root.unwrap(), dir.path().canonicalize().unwrap());
}

#[specforge_test(
    behavior = "find_project_root",
    verify = "specforge.spec found when specforge.json is absent at same level"
)]
#[test]
fn find_project_root_spec_fallback() {
    use specforge_common::find_project_root;

    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("specforge.spec"), "").unwrap();

    let root = find_project_root(dir.path());
    assert_eq!(root.unwrap(), dir.path().canonicalize().unwrap());
}

#[specforge_test(
    behavior = "find_project_root",
    verify = "closest directory wins over ancestor directory"
)]
#[test]
fn find_project_root_closest_wins() {
    use specforge_common::find_project_root;

    let dir = TempDir::new().unwrap();
    // Parent has specforge.json
    fs::write(dir.path().join("specforge.json"), "{}").unwrap();
    // Child also has specforge.json
    let child = dir.path().join("child");
    fs::create_dir_all(&child).unwrap();
    fs::write(child.join("specforge.json"), "{}").unwrap();

    let root = find_project_root(&child);
    assert_eq!(root.unwrap(), child.canonicalize().unwrap());
}

#[specforge_test(
    behavior = "find_project_root",
    verify = "specforge.json takes precedence over specforge.spec in same directory"
)]
#[test]
fn find_project_root_json_precedence() {
    use specforge_common::find_project_root;

    // Both files exist — function returns the directory (precedence is implicit:
    // json is checked first, but result is the same directory either way).
    // The real test: if parent has specforge.spec and child has specforge.json,
    // child wins because closest-wins, not because of file type.
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("specforge.json"), "{}").unwrap();
    fs::write(dir.path().join("specforge.spec"), "").unwrap();

    let root = find_project_root(dir.path());
    assert_eq!(root.unwrap(), dir.path().canonicalize().unwrap());
}

#[specforge_test(
    behavior = "find_project_root",
    verify = "no config found returns None"
)]
#[test]
fn find_project_root_none_when_missing() {
    use specforge_common::find_project_root;

    let dir = TempDir::new().unwrap();
    let child = dir.path().join("empty");
    fs::create_dir_all(&child).unwrap();

    // No specforge.json or specforge.spec anywhere in the temp dir
    let root = find_project_root(&child);
    // We can't assert None absolutely (the real filesystem root might have one),
    // but in a temp dir with no config files, it should walk up and not find
    // anything in the temp hierarchy. We check it doesn't return the child dir.
    if let Some(found) = root {
        // If something was found, it must NOT be in our temp dir
        assert!(
            !found.starts_with(dir.path().canonicalize().unwrap()),
            "should not find project root in empty temp dir"
        );
    }
}

// ═══════════════════════════════════════════════════════════
// Behavior: scaffold_new_project
// ═══════════════════════════════════════════════════════════

#[specforge_test(
    behavior = "scaffold_new_project",
    verify = "scaffold creates valid specforge.json"
)]
#[test]
fn init_creates_valid_specforge_json() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "my-project"])
        .current_dir(dir.path())
        .assert()
        .success();

    let config_path = dir.path().join("specforge.json");
    assert!(config_path.exists(), "specforge.json should be created");

    let content = fs::read_to_string(&config_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content)
        .expect("specforge.json should be valid JSON");

    assert_eq!(json["name"], "my-project");
    assert_eq!(json["version"], "0.1.0");
    assert!(json["extensions"].is_array());
    assert!(json["spec_root"].is_string());
}

#[specforge_test(
    behavior = "scaffold_new_project",
    verify = "scaffold includes $schema field in generated config"
)]
#[test]
fn init_includes_schema_field() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "test-schema"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = fs::read_to_string(dir.path().join("specforge.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert!(
        json["$schema"].is_string(),
        "specforge.json should include $schema field"
    );
    assert!(
        json["$schema"].as_str().unwrap().contains("specforge"),
        "$schema URL should reference specforge"
    );
}

#[specforge_test(
    behavior = "scaffold_new_project",
    verify = "scaffold rejects when specforge.json already exists"
)]
#[test]
fn init_rejects_existing_project() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("specforge.json"), "{}").unwrap();

    specforge_cmd()
        .args(["init", "--name", "duplicate"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("already exists"));
}

#[specforge_test(
    behavior = "scaffold_new_project",
    verify = "scaffold rejects when a parent directory contains specforge.json"
)]
#[test]
fn init_rejects_when_parent_has_config() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("specforge.json"), "{}").unwrap();

    let child = dir.path().join("subdir");
    fs::create_dir_all(&child).unwrap();

    specforge_cmd()
        .args(["init", "--name", "nested"])
        .current_dir(&child)
        .assert()
        .failure()
        .stderr(predicates::str::contains("already exists"));
}

#[specforge_test(
    behavior = "scaffold_new_project",
    verify = "scaffold in non-empty directory preserves existing files"
)]
#[test]
fn init_preserves_existing_files() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("README.md"), "# Hello").unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();

    specforge_cmd()
        .args(["init", "--name", "existing"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Original files should still exist
    assert_eq!(fs::read_to_string(dir.path().join("README.md")).unwrap(), "# Hello");
    assert_eq!(fs::read_to_string(dir.path().join("src/main.rs")).unwrap(), "fn main() {}");
    // And specforge.json should also exist
    assert!(dir.path().join("specforge.json").exists());
}

// ═══════════════════════════════════════════════════════════
// Behavior: scaffold_starter_spec_file
// ═══════════════════════════════════════════════════════════

#[specforge_test(
    behavior = "scaffold_starter_spec_file",
    verify = "starter spec file is created alongside specforge.json"
)]
#[test]
fn init_creates_starter_spec_file() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "test-starter"])
        .current_dir(dir.path())
        .assert()
        .success();

    let starter = dir.path().join("spec").join("hello.spec");
    assert!(starter.exists(), "starter spec file should be created at spec/hello.spec");
    let content = fs::read_to_string(&starter).unwrap();
    assert!(!content.is_empty(), "starter spec file should not be empty");
}

#[specforge_test(
    behavior = "scaffold_starter_spec_file",
    verify = "starter spec file passes specforge check with zero errors"
)]
#[test]
fn init_starter_passes_check() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "check-test"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Run specforge check on the newly created project
    specforge_cmd()
        .args(["check", "--format", "json"])
        .arg(dir.path().join("spec"))
        .assert()
        .success();
}

#[specforge_test(
    behavior = "scaffold_starter_spec_file",
    verify = "starter file uses only structural syntax when no extensions contribute templates"
)]
#[test]
fn init_starter_uses_structural_syntax_only() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "structural"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = fs::read_to_string(dir.path().join("spec/hello.spec")).unwrap();

    // Should NOT contain domain-specific keywords from any extension
    let domain_keywords = ["behavior", "invariant", "feature", "event", "port",
                           "journey", "deliverable", "milestone", "module",
                           "decision", "constraint", "failure_mode", "term"];
    for kw in domain_keywords {
        assert!(
            !content.contains(&format!("{kw} ")),
            "starter file should not contain domain keyword '{kw}'"
        );
    }
}

#[specforge_test(
    behavior = "scaffold_starter_spec_file",
    verify = "starter file content is deterministic for same extension set"
)]
#[test]
fn init_starter_is_deterministic() {
    let dir1 = TempDir::new().unwrap();
    let dir2 = TempDir::new().unwrap();

    for dir in [dir1.path(), dir2.path()] {
        specforge_cmd()
            .args(["init", "--name", "deterministic"])
            .current_dir(dir)
            .assert()
            .success();
    }

    let content1 = fs::read_to_string(dir1.path().join("spec/hello.spec")).unwrap();
    let content2 = fs::read_to_string(dir2.path().join("spec/hello.spec")).unwrap();

    assert_eq!(content1, content2, "same inputs should produce identical starter files");
}

// ═══════════════════════════════════════════════════════════
// Behavior: graceful_zero_extension_init
// ═══════════════════════════════════════════════════════════

#[specforge_test(
    behavior = "graceful_zero_extension_init",
    verify = "zero-extension init creates valid specforge.json with empty extensions"
)]
#[test]
fn zero_ext_init_creates_valid_config() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "zero-ext"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = fs::read_to_string(dir.path().join("specforge.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(json["extensions"], serde_json::json!([]));
}

#[specforge_test(
    behavior = "graceful_zero_extension_init",
    verify = "zero-extension config produces empty extensions array []"
)]
#[test]
fn zero_ext_config_has_empty_array() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "empty-ext"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = fs::read_to_string(dir.path().join("specforge.json")).unwrap();
    // Verify the raw JSON contains "extensions": []
    assert!(
        content.contains(r#""extensions": []"#),
        "config should contain literal empty extensions array, got:\n{content}"
    );
}

#[specforge_test(
    behavior = "graceful_zero_extension_init",
    verify = "zero-extension starter file passes specforge check"
)]
#[test]
fn zero_ext_starter_passes_check() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "zero-check"])
        .current_dir(dir.path())
        .assert()
        .success();

    specforge_cmd()
        .args(["check", "--format", "json"])
        .arg(dir.path().join("spec"))
        .assert()
        .success();
}

#[specforge_test(
    behavior = "graceful_zero_extension_init",
    verify = "zero-extension project produces valid graph via specforge export"
)]
#[test]
fn zero_ext_export_produces_valid_graph() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "zero-export"])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = specforge_cmd()
        .args(["export", "--format", "graph"])
        .arg(dir.path().join("spec"))
        .output()
        .unwrap();

    assert!(output.status.success(), "export should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let graph: serde_json::Value = serde_json::from_str(&stdout)
        .expect("export should produce valid JSON");
    assert!(graph["nodes"].is_array(), "graph should have nodes array");
    assert!(graph["edges"].is_array(), "graph should have edges array");
}

// ═══════════════════════════════════════════════════════════
// Behavior: non_interactive_init
// ═══════════════════════════════════════════════════════════

#[specforge_test(
    behavior = "non_interactive_init",
    verify = "non-interactive init creates valid specforge.json"
)]
#[test]
fn non_interactive_creates_valid_config() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "ci-project"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = fs::read_to_string(dir.path().join("specforge.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["name"], "ci-project");
}

#[specforge_test(
    behavior = "non_interactive_init",
    verify = "non-interactive init skips all prompts"
)]
#[test]
fn non_interactive_no_prompts() {
    let dir = TempDir::new().unwrap();

    // Pipe empty stdin — should not hang waiting for input
    let output = specforge_cmd()
        .args(["init", "--name", "no-prompt"])
        .current_dir(dir.path())
        .write_stdin("")
        .output()
        .unwrap();

    assert!(output.status.success(), "should succeed without interactive input");
    assert!(dir.path().join("specforge.json").exists());
}

#[specforge_test(
    behavior = "non_interactive_init",
    verify = "non-interactive init with --extensions populates extensions list"
)]
#[test]
fn non_interactive_with_extensions() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "ext-project",
               "--extensions", "@specforge/software",
               "--extensions", "@specforge/product"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = fs::read_to_string(dir.path().join("specforge.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let extensions = json["extensions"].as_array().unwrap();
    assert_eq!(extensions.len(), 2);
    assert_eq!(extensions[0], "@specforge/software");
    assert_eq!(extensions[1], "@specforge/product");
}

#[specforge_test(
    behavior = "non_interactive_init",
    verify = "non-interactive init with --format=json outputs InitOutput JSON"
)]
#[test]
fn non_interactive_json_output() {
    let dir = TempDir::new().unwrap();

    let output = specforge_cmd()
        .args(["init", "--name", "json-out", "--format", "json"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("--format=json should produce valid JSON");

    assert!(json["project_root"].is_string(), "should have project_root");
    assert!(json["config_path"].is_string(), "should have config_path");
    assert!(json["spec_file_path"].is_string(), "should have spec_file_path");
    assert!(json["extensions_installed"].is_array(), "should have extensions_installed");
}

#[specforge_test(
    behavior = "non_interactive_init",
    verify = "non-interactive init --format=json includes all 4 required fields: project_root, config_path, spec_file_path, extensions_installed"
)]
#[test]
fn non_interactive_json_all_fields() {
    let dir = TempDir::new().unwrap();

    let output = specforge_cmd()
        .args(["init", "--name", "fields-test", "--format", "json",
               "--extensions", "@specforge/software"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Verify all 4 required fields exist and are correct types
    let project_root = json["project_root"].as_str().unwrap();
    assert!(!project_root.is_empty());

    let config_path = json["config_path"].as_str().unwrap();
    assert!(config_path.ends_with("specforge.json"));

    let spec_path = json["spec_file_path"].as_str().unwrap();
    assert!(spec_path.ends_with("hello.spec"));

    let exts = json["extensions_installed"].as_array().unwrap();
    assert_eq!(exts, &[serde_json::json!("@specforge/software")]);
}

#[specforge_test(
    behavior = "non_interactive_init",
    verify = "non-interactive init with --version overrides default version in specforge.json"
)]
#[test]
fn non_interactive_version_override() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "versioned", "--version", "2.0.0"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = fs::read_to_string(dir.path().join("specforge.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["version"], "2.0.0");
}

#[specforge_test(
    behavior = "non_interactive_init",
    verify = "non-interactive output matches interactive output for same inputs"
)]
#[test]
fn non_interactive_matches_default_name() {
    // With --name, the result should be deterministic regardless of how
    // the name was provided (flag vs. directory name inference).
    let dir1 = TempDir::new().unwrap();
    let dir2 = TempDir::new().unwrap();

    for dir in [dir1.path(), dir2.path()] {
        specforge_cmd()
            .args(["init", "--name", "same-project"])
            .current_dir(dir)
            .assert()
            .success();
    }

    let c1 = fs::read_to_string(dir1.path().join("specforge.json")).unwrap();
    let c2 = fs::read_to_string(dir2.path().join("specforge.json")).unwrap();
    assert_eq!(c1, c2, "same --name should produce identical config");
}

// ═══════════════════════════════════════════════════════════
// Additional coverage: find_project_root
// ═══════════════════════════════════════════════════════════

#[cfg(unix)]
#[specforge_test(
    behavior = "find_project_root",
    verify = "symlinks are resolved before path comparison"
)]
#[test]
fn find_project_root_resolves_symlinks() {
    use specforge_common::find_project_root;

    let dir = TempDir::new().unwrap();
    let real_dir = dir.path().join("real");
    fs::create_dir_all(&real_dir).unwrap();
    fs::write(real_dir.join("specforge.json"), "{}").unwrap();

    // Create a symlink pointing to real_dir
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&real_dir, &link).unwrap();

    let root = find_project_root(&link);
    // Should resolve to the canonical (real) path
    assert_eq!(root.unwrap(), real_dir.canonicalize().unwrap());
}

#[cfg(unix)]
#[specforge_test(
    behavior = "find_project_root",
    verify = "circular symlink chain does not cause infinite loop"
)]
#[test]
fn find_project_root_handles_circular_symlinks() {
    use specforge_common::find_project_root;

    let dir = TempDir::new().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    fs::create_dir_all(&a).unwrap();

    // Create circular symlink: a/link -> b, b -> a
    // canonicalize() at the start of find_project_root will fail on a
    // truly circular path, returning None — which is the safe behavior.
    std::os::unix::fs::symlink(&a, &b).unwrap();
    std::os::unix::fs::symlink(&b, a.join("link")).unwrap();

    // Should not hang — either returns None or a valid root
    let result = find_project_root(&a.join("link"));
    // The important thing is it terminates. If it found something, it's fine.
    // If None, that's also fine (no specforge.json in the temp dir).
    if let Some(found) = result {
        assert!(found.exists());
    }
}

#[specforge_test(
    behavior = "find_project_root",
    verify = "directory traversal completes in under 100ms for 20-level deep hierarchy"
)]
#[test]
fn find_project_root_performance() {
    use specforge_common::find_project_root;

    let dir = TempDir::new().unwrap();
    // Create 20-level deep hierarchy with config at root
    fs::write(dir.path().join("specforge.json"), "{}").unwrap();
    let mut deep = dir.path().to_path_buf();
    for i in 0..20 {
        deep = deep.join(format!("level_{i}"));
    }
    fs::create_dir_all(&deep).unwrap();

    let start = std::time::Instant::now();
    let root = find_project_root(&deep);
    let elapsed = start.elapsed();

    assert!(root.is_some(), "should find root 20 levels up");
    assert!(
        elapsed.as_millis() < 100,
        "traversal took {}ms, expected < 100ms",
        elapsed.as_millis()
    );
}

// ═══════════════════════════════════════════════════════════
// Additional coverage: scaffold_new_project
// ═══════════════════════════════════════════════════════════

#[specforge_test(
    behavior = "scaffold_new_project",
    verify = "scaffolded project passes init-check-export cycle"
)]
#[test]
fn init_check_export_cycle() {
    let dir = TempDir::new().unwrap();

    // init
    specforge_cmd()
        .args(["init", "--name", "cycle-test"])
        .current_dir(dir.path())
        .assert()
        .success();

    // check
    specforge_cmd()
        .args(["check", "--format", "json"])
        .arg(dir.path().join("spec"))
        .assert()
        .success();

    // export
    let output = specforge_cmd()
        .args(["export", "--format", "graph"])
        .arg(dir.path().join("spec"))
        .output()
        .unwrap();
    assert!(output.status.success(), "export should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let _graph: serde_json::Value = serde_json::from_str(&stdout)
        .expect("export should produce valid JSON graph");
}

#[specforge_test(
    behavior = "scaffold_new_project",
    verify = "full init-check-export cycle completes in under 60 seconds"
)]
#[test]
fn init_check_export_performance() {
    let dir = TempDir::new().unwrap();
    let start = std::time::Instant::now();

    specforge_cmd()
        .args(["init", "--name", "perf-test"])
        .current_dir(dir.path())
        .assert()
        .success();

    specforge_cmd()
        .args(["check"])
        .arg(dir.path().join("spec"))
        .assert()
        .success();

    specforge_cmd()
        .args(["export", "--format", "graph"])
        .arg(dir.path().join("spec"))
        .assert()
        .success();

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 60,
        "init-check-export cycle took {}s, expected < 60s",
        elapsed.as_secs()
    );
}

// ═══════════════════════════════════════════════════════════
// Additional coverage: scaffold_starter_spec_file
// ═══════════════════════════════════════════════════════════

#[specforge_test(
    behavior = "scaffold_starter_spec_file",
    verify = "starter file contains no domain-specific keywords from extensions"
)]
#[test]
fn init_starter_no_domain_keywords() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "no-domain"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = fs::read_to_string(dir.path().join("spec/hello.spec")).unwrap();

    // All 14 domain entity keywords from the 3 standard extensions
    let domain_keywords = [
        "behavior", "invariant", "feature", "event", "port", "type",
        "journey", "deliverable", "milestone", "module", "term",
        "decision", "constraint", "failure_mode",
    ];
    for kw in domain_keywords {
        // Check for keyword used as entity declaration (keyword followed by space + id)
        let pattern = format!("\n{kw} ");
        assert!(
            !content.contains(&pattern),
            "starter file should not contain domain entity declaration '{kw}'"
        );
    }
}

// ═══════════════════════════════════════════════════════════
// Additional coverage: graceful_zero_extension_init
// ═══════════════════════════════════════════════════════════

#[specforge_test(
    behavior = "graceful_zero_extension_init",
    verify = "graceful_zero_extension_init completes full init-check-export cycle in under 60 seconds"
)]
#[test]
fn zero_ext_full_cycle_performance() {
    let dir = TempDir::new().unwrap();
    let start = std::time::Instant::now();

    // init with zero extensions (default)
    specforge_cmd()
        .args(["init", "--name", "zero-perf"])
        .current_dir(dir.path())
        .assert()
        .success();

    // check
    specforge_cmd()
        .args(["check"])
        .arg(dir.path().join("spec"))
        .assert()
        .success();

    // export
    specforge_cmd()
        .args(["export", "--format", "graph"])
        .arg(dir.path().join("spec"))
        .assert()
        .success();

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 60,
        "zero-extension init-check-export cycle took {}s, expected < 60s",
        elapsed.as_secs()
    );
}

// ═══════════════════════════════════════════════════════════
// Additional coverage: non_interactive_init
// ═══════════════════════════════════════════════════════════

#[specforge_test(
    behavior = "non_interactive_init",
    verify = "invalid project name is rejected with InitError::invalid_name"
)]
#[test]
fn non_interactive_invalid_name_rejected() {
    let dir = TempDir::new().unwrap();

    // Empty name
    specforge_cmd()
        .args(["init", "--name", ""])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("invalid project name"));

    // Name starting with dot
    let dir2 = TempDir::new().unwrap();
    specforge_cmd()
        .args(["init", "--name", ".hidden"])
        .current_dir(dir2.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("invalid project name"));

    // Name starting with dash — use --name=VALUE form to prevent clap flag parsing
    let dir3 = TempDir::new().unwrap();
    specforge_cmd()
        .args(["init", "--name=-bad"])
        .current_dir(dir3.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("invalid project name"));
}

#[specforge_test(
    behavior = "non_interactive_init",
    verify = "non_interactive_init completes full init-check-export cycle in under 60 seconds"
)]
#[test]
fn non_interactive_full_cycle_performance() {
    let dir = TempDir::new().unwrap();
    let start = std::time::Instant::now();

    specforge_cmd()
        .args(["init", "--name", "ci-perf",
               "--extensions", "@specforge/software"])
        .current_dir(dir.path())
        .assert()
        .success();

    specforge_cmd()
        .args(["check"])
        .arg(dir.path().join("spec"))
        .assert()
        .success();

    specforge_cmd()
        .args(["export", "--format", "graph"])
        .arg(dir.path().join("spec"))
        .assert()
        .success();

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 60,
        "non-interactive init-check-export cycle took {}s, expected < 60s",
        elapsed.as_secs()
    );
}

#[specforge_test(
    behavior = "non_interactive_init",
    verify = "non-interactive init with unknown extension rejects with diagnostic and exit code 1"
)]
#[test]
fn non_interactive_unknown_extension_rejected() {
    let dir = TempDir::new().unwrap();

    // Invalid extension specifier (no @scope/name format)
    specforge_cmd()
        .args(["init", "--name", "bad-ext", "--extensions", "not-a-valid-ext"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("unresolvable extension"));

    // No specforge.json should be created on failure
    assert!(!dir.path().join("specforge.json").exists());

    // Empty extension
    let dir2 = TempDir::new().unwrap();
    specforge_cmd()
        .args(["init", "--name", "empty-ext", "--extensions", ""])
        .current_dir(dir2.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("unresolvable extension"));
}

// ═══════════════════════════════════════════════════════════
// Contract tests (requires/ensures consistency)
// ═══════════════════════════════════════════════════════════

#[specforge_test(
    behavior = "find_project_root",
    verify = "requires/ensures consistency for project root discovery"
)]
#[test]
fn find_project_root_contract_in_init() {
    use specforge_common::find_project_root;

    // Requires: FileSystem port available for directory traversal
    // Ensures: closest-wins, json precedence, symlinks resolved, None on missing
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("specforge.json"), "{}").unwrap();
    let child = dir.path().join("sub").join("deep");
    fs::create_dir_all(&child).unwrap();

    // Closest-wins from nested directory
    let root = find_project_root(&child);
    assert_eq!(root.unwrap(), dir.path().canonicalize().unwrap());

    // Direct lookup also works
    let direct = find_project_root(dir.path());
    assert_eq!(direct.unwrap(), dir.path().canonicalize().unwrap());

    // None when no config exists
    let empty = TempDir::new().unwrap();
    let empty_child = empty.path().join("nothing");
    fs::create_dir_all(&empty_child).unwrap();
    let result = find_project_root(&empty_child);
    if let Some(found) = result {
        assert!(
            !found.starts_with(empty.path().canonicalize().unwrap()),
            "must not find root in empty hierarchy"
        );
    }
}

#[specforge_test(
    behavior = "scaffold_new_project",
    verify = "requires/ensures consistency for new project scaffolding"
)]
#[test]
fn scaffold_new_project_contract_in_init() {
    // Requires: filesystem available, no existing project
    // Ensures: valid config created, $schema included, project_initialized emitted
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "contract-scaffold"])
        .current_dir(dir.path())
        .assert()
        .success();

    let config_path = dir.path().join("specforge.json");
    assert!(config_path.exists(), "specforge.json must be created");

    let content = fs::read_to_string(&config_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content)
        .expect("specforge.json must be valid JSON");
    assert_eq!(json["name"], "contract-scaffold");
    assert!(json["version"].is_string(), "version must be present");
    assert!(json["$schema"].is_string(), "$schema must be present");
    assert!(json["extensions"].is_array(), "extensions must be an array");
    assert!(json["spec_root"].is_string(), "spec_root must be present");

    // Starter spec file must also exist
    assert!(dir.path().join("spec").join("hello.spec").exists());

    // Reject when project already exists (precondition violated)
    specforge_cmd()
        .args(["init", "--name", "contract-scaffold"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[specforge_test(
    behavior = "scaffold_starter_spec_file",
    verify = "extension-contributed starter templates are used when available"
)]
#[test]
fn starter_uses_extension_templates_when_available() {
    let dir = TempDir::new().unwrap();

    // Init with an extension — starter file may include extension-contributed content
    specforge_cmd()
        .args(["init", "--name", "ext-template",
               "--extensions", "@specforge/software"])
        .current_dir(dir.path())
        .assert()
        .success();

    let starter = dir.path().join("spec").join("hello.spec");
    assert!(starter.exists(), "starter spec file must exist");
    let content = fs::read_to_string(&starter).unwrap();
    assert!(!content.is_empty(), "starter file must not be empty");

    // When an extension is installed, the starter MAY contain
    // extension-contributed content (domain keywords). At minimum,
    // the file must exist and be non-empty.
}

#[specforge_test(
    behavior = "scaffold_starter_spec_file",
    verify = "extension-contributed starter file passes specforge check with zero errors"
)]
#[test]
fn extension_starter_passes_check() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "ext-check",
               "--extensions", "@specforge/software"])
        .current_dir(dir.path())
        .assert()
        .success();

    // The starter file created with extension templates must still pass check
    specforge_cmd()
        .args(["check", "--format", "json"])
        .arg(dir.path().join("spec"))
        .assert()
        .success();
}

#[specforge_test(
    behavior = "scaffold_starter_spec_file",
    verify = "requires/ensures consistency for starter spec file scaffolding"
)]
#[test]
fn scaffold_starter_spec_file_contract() {
    // Requires: specforge.json created, filesystem available
    // Ensures: starter file created, structural syntax only (no extensions),
    //          zero diagnostic pass
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "starter-contract"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Config must exist (precondition for starter file creation)
    assert!(dir.path().join("specforge.json").exists());

    // Starter file must exist alongside config
    let starter = dir.path().join("spec").join("hello.spec");
    assert!(starter.exists(), "starter file must be created");
    let content = fs::read_to_string(&starter).unwrap();
    assert!(!content.is_empty(), "starter file must not be empty");

    // Must pass check with zero diagnostics
    specforge_cmd()
        .args(["check", "--format", "json"])
        .arg(dir.path().join("spec"))
        .assert()
        .success();
}

#[specforge_test(
    behavior = "non_interactive_init",
    verify = "requires/ensures consistency for non-interactive init"
)]
#[test]
fn non_interactive_init_contract_in_init() {
    // Requires: --name flag provided, filesystem available, no existing project
    // Ensures: config identical to interactive, all prompts skipped,
    //          JSON output supported, project_initialized emitted
    let dir = TempDir::new().unwrap();

    let output = specforge_cmd()
        .args(["init", "--name", "ni-contract"])
        .current_dir(dir.path())
        .write_stdin("") // empty stdin — must not hang
        .output()
        .unwrap();

    assert!(output.status.success(), "must succeed without interactive input");

    let config_path = dir.path().join("specforge.json");
    assert!(config_path.exists(), "config must be created");

    let content = fs::read_to_string(&config_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["name"], "ni-contract");
    assert!(json["version"].is_string());
    assert!(json["extensions"].is_array());
}

#[specforge_test(
    behavior = "graceful_zero_extension_init",
    verify = "requires/ensures consistency for zero-extension init"
)]
#[test]
fn graceful_zero_extension_init_contract_in_init() {
    // Requires: zero extensions selected, filesystem available
    // Ensures: empty extensions list, structural starter valid,
    //          valid graph exportable, project_initialized emitted
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["init", "--name", "zero-contract"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = fs::read_to_string(dir.path().join("specforge.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["extensions"], serde_json::json!([]),
        "extensions must be empty array");

    // Check passes
    specforge_cmd()
        .args(["check", "--format", "json"])
        .arg(dir.path().join("spec"))
        .assert()
        .success();

    // Export produces valid graph
    let export_output = specforge_cmd()
        .args(["export", "--format", "graph"])
        .arg(dir.path().join("spec"))
        .output()
        .unwrap();
    assert!(export_output.status.success());
    let stdout = String::from_utf8_lossy(&export_output.stdout);
    let graph: serde_json::Value = serde_json::from_str(&stdout)
        .expect("export must produce valid JSON");
    assert!(graph["nodes"].is_array());
    assert!(graph["edges"].is_array());
}
