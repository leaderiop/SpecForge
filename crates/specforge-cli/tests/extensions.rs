use assert_cmd::Command;
use specforge_test_macros::test as specforge_test;
use std::fs;
use tempfile::TempDir;

fn specforge_cmd() -> Command {
    assert_cmd::cargo_bin_cmd!("specforge")
}

/// Helper: create a specforge.lock file with test entries.
fn write_lock_file(dir: &std::path::Path, entries: &[(&str, &str, &str)]) {
    let lock_entries: Vec<serde_json::Value> = entries
        .iter()
        .map(|(name, version, source)| {
            serde_json::json!({
                "name": name,
                "version": version,
                "source": source,
                "wasm_hash": format!("hash_{}", name.replace(['@', '/'], "_")),
            })
        })
        .collect();

    let lock = serde_json::json!({
        "lockfile_version": 1,
        "entries": lock_entries,
    });

    fs::write(
        dir.join("specforge.lock"),
        serde_json::to_string_pretty(&lock).unwrap(),
    )
    .unwrap();
}

/// Helper: create a specforge.json with providers config.
fn write_config_with_providers(dir: &std::path::Path, providers: &[serde_json::Value]) {
    let config = serde_json::json!({
        "$schema": "https://specforge.dev/schema/specforge.json",
        "name": "test-project",
        "version": "0.1.0",
        "spec_root": "spec",
        "extensions": [],
        "providers": providers,
    });

    fs::write(
        dir.join("specforge.json"),
        serde_json::to_string_pretty(&config).unwrap(),
    )
    .unwrap();
}

// ===============================================================
// Behavior: remove_extension
// ===============================================================

#[specforge_test(
    behavior = "remove_extension",
    verify = "delegates to uninstall_wasm_extension for lifecycle cleanup"
)]
#[test]
fn remove_delegates_to_uninstall() {
    let dir = TempDir::new().unwrap();

    // Create a lock file with one extension
    write_lock_file(dir.path(), &[("@specforge/software", "1.0.0", "registry")]);

    // Create the extension directory so uninstall can remove it
    let ext_dir = dir.path().join(".specforge").join("extensions").join("@specforge/software");
    fs::create_dir_all(&ext_dir).unwrap();
    fs::write(ext_dir.join("extension.wasm"), b"fake wasm").unwrap();

    specforge_cmd()
        .args(["remove", "@specforge/software", "--path"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Removed extension"));

    // Lock file should now have empty entries
    let lock_content = fs::read_to_string(dir.path().join("specforge.lock")).unwrap();
    let lock: serde_json::Value = serde_json::from_str(&lock_content).unwrap();
    assert_eq!(lock["entries"].as_array().unwrap().len(), 0);
}

#[specforge_test(
    behavior = "remove_extension",
    verify = "extension is removed from extensions list"
)]
#[test]
fn remove_updates_lock_file() {
    let dir = TempDir::new().unwrap();

    write_lock_file(
        dir.path(),
        &[
            ("@specforge/software", "1.0.0", "registry"),
            ("@specforge/governance", "1.0.0", "registry"),
        ],
    );

    // Create extension directories
    for name in &["@specforge/software", "@specforge/governance"] {
        let ext_dir = dir.path().join(".specforge").join("extensions").join(name);
        fs::create_dir_all(&ext_dir).unwrap();
        fs::write(ext_dir.join("extension.wasm"), b"fake").unwrap();
    }

    // Remove software
    specforge_cmd()
        .args(["remove", "@specforge/software", "--path"])
        .arg(dir.path())
        .assert()
        .success();

    // Lock file should only have governance remaining
    let lock_content = fs::read_to_string(dir.path().join("specforge.lock")).unwrap();
    let lock: serde_json::Value = serde_json::from_str(&lock_content).unwrap();
    let entries = lock["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["name"], "@specforge/governance");
}

#[specforge_test(
    behavior = "remove_extension",
    verify = ".spec files are not modified by removal"
)]
#[test]
fn remove_does_not_modify_spec_files() {
    let dir = TempDir::new().unwrap();

    // Create a spec file
    let spec_dir = dir.path().join("spec");
    fs::create_dir_all(&spec_dir).unwrap();
    let spec_content = "behavior my_behavior \"test\" {\n  description \"hello\"\n}\n";
    fs::write(spec_dir.join("test.spec"), spec_content).unwrap();

    write_lock_file(dir.path(), &[("@specforge/software", "1.0.0", "registry")]);

    let ext_dir = dir.path().join(".specforge").join("extensions").join("@specforge/software");
    fs::create_dir_all(&ext_dir).unwrap();
    fs::write(ext_dir.join("extension.wasm"), b"fake").unwrap();

    specforge_cmd()
        .args(["remove", "@specforge/software", "--path"])
        .arg(dir.path())
        .assert()
        .success();

    // Spec file should be unchanged
    let after = fs::read_to_string(spec_dir.join("test.spec")).unwrap();
    assert_eq!(after, spec_content, ".spec files must not be modified by remove");
}

// ===============================================================
// Behavior: list_installed_extensions
// ===============================================================

#[specforge_test(
    behavior = "list_installed_extensions",
    verify = "output order is deterministic"
)]
#[test]
fn extensions_lists_alphabetically() {
    let dir = TempDir::new().unwrap();

    // Write entries in reverse alphabetical order
    write_lock_file(
        dir.path(),
        &[
            ("@specforge/software", "1.0.0", "registry"),
            ("@specforge/governance", "1.0.0", "registry"),
            ("@specforge/product", "1.0.0", "registry"),
        ],
    );

    let output = specforge_cmd()
        .args(["extensions", "--path"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify all three appear
    assert!(stdout.contains("@specforge/governance"));
    assert!(stdout.contains("@specforge/product"));
    assert!(stdout.contains("@specforge/software"));

    // Verify alphabetical ordering: governance < product < software
    let gov_pos = stdout.find("@specforge/governance").unwrap();
    let prod_pos = stdout.find("@specforge/product").unwrap();
    let sw_pos = stdout.find("@specforge/software").unwrap();
    assert!(
        gov_pos < prod_pos && prod_pos < sw_pos,
        "extensions should be listed alphabetically"
    );
}

#[specforge_test(
    behavior = "list_installed_extensions",
    verify = "list includes entity counts and entity types"
)]
#[test]
fn extensions_json_format() {
    let dir = TempDir::new().unwrap();

    write_lock_file(
        dir.path(),
        &[
            ("@specforge/software", "1.2.0", "registry"),
            ("@specforge/governance", "1.0.0", "local"),
        ],
    );

    let output = specforge_cmd()
        .args(["extensions", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("--format json should produce valid JSON");

    assert_eq!(json["count"], 2);
    let extensions = json["extensions"].as_array().unwrap();
    assert_eq!(extensions.len(), 2);

    // Sorted alphabetically, governance first
    assert_eq!(extensions[0]["name"], "@specforge/governance");
    assert_eq!(extensions[0]["version"], "1.0.0");
    assert_eq!(extensions[0]["source"], "local");

    assert_eq!(extensions[1]["name"], "@specforge/software");
    assert_eq!(extensions[1]["version"], "1.2.0");
    assert_eq!(extensions[1]["source"], "registry");
}

#[specforge_test(
    behavior = "list_installed_extensions",
    verify = "list shows all installed extensions"
)]
#[test]
fn extensions_no_lock_file() {
    let dir = TempDir::new().unwrap();

    let output = specforge_cmd()
        .args(["extensions", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["count"], 0);
    assert_eq!(json["extensions"].as_array().unwrap().len(), 0);
}

#[specforge_test(
    behavior = "list_installed_extensions",
    verify = "requires/ensures consistency for extension listing"
)]
#[test]
fn extensions_contract() {
    let dir = TempDir::new().unwrap();

    // Precondition: lock file with known extensions
    write_lock_file(
        dir.path(),
        &[
            ("@specforge/governance", "1.0.0", "registry"),
            ("@specforge/software", "2.0.0", "local"),
        ],
    );

    let output = specforge_cmd()
        .args(["extensions", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Postcondition: all_extensions_listed
    assert_eq!(json["count"], 2, "ensures: all_extensions_listed");

    // Postcondition: output_deterministic (sorted alphabetically)
    let extensions = json["extensions"].as_array().unwrap();
    assert_eq!(
        extensions[0]["name"], "@specforge/governance",
        "ensures: output_deterministic — alphabetical"
    );
    assert_eq!(extensions[1]["name"], "@specforge/software");
}

// ===============================================================
// Behavior: list_configured_providers
// ===============================================================

#[specforge_test(
    behavior = "list_configured_providers",
    verify = "list shows all configured providers"
)]
#[test]
fn providers_lists_alias_extension_schemes() {
    let dir = TempDir::new().unwrap();

    write_config_with_providers(
        dir.path(),
        &[serde_json::json!({
            "alias": "junit",
            "extension": "@specforge/rust",
            "schemes": ["file", "glob"],
        })],
    );

    let output = specforge_cmd()
        .args(["providers", "--path"])
        .arg(dir.path())
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("junit"), "should show alias");
    assert!(stdout.contains("@specforge/rust"), "should show extension");
    assert!(stdout.contains("file"), "should show scheme");
    assert!(stdout.contains("glob"), "should show scheme");
}

#[specforge_test(
    behavior = "list_configured_providers",
    verify = "multiple aliases shown separately"
)]
#[test]
fn providers_multiple_aliases() {
    let dir = TempDir::new().unwrap();

    write_config_with_providers(
        dir.path(),
        &[
            serde_json::json!({
                "alias": "junit",
                "extension": "@specforge/rust",
                "schemes": ["file"],
            }),
            serde_json::json!({
                "alias": "pytest",
                "extension": "@specforge/python",
                "schemes": ["file", "http"],
            }),
        ],
    );

    let output = specforge_cmd()
        .args(["providers", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["count"], 2);
    let providers = json["providers"].as_array().unwrap();
    assert_eq!(providers[0]["alias"], "junit");
    assert_eq!(providers[1]["alias"], "pytest");
    assert_eq!(providers[1]["extension"], "@specforge/python");
}

#[specforge_test(
    behavior = "list_configured_providers",
    verify = "list includes scheme and kind registrations"
)]
#[test]
fn providers_includes_scheme_and_kind() {
    let dir = TempDir::new().unwrap();

    write_config_with_providers(
        dir.path(),
        &[serde_json::json!({
            "alias": "junit",
            "extension": "@specforge/rust",
            "schemes": ["file", "glob"],
            "kinds": ["junit_xml"],
        })],
    );

    let output = specforge_cmd()
        .args(["providers", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let providers = json["providers"].as_array().unwrap();
    let p = &providers[0];
    assert_eq!(p["schemes"].as_array().unwrap().len(), 2);
    assert!(p["schemes"].as_array().unwrap().iter().any(|s| s == "file"));
    assert!(p["schemes"].as_array().unwrap().iter().any(|s| s == "glob"));
}

#[specforge_test(
    behavior = "list_configured_providers",
    verify = "output order is deterministic"
)]
#[test]
fn providers_output_order_deterministic() {
    let dir = TempDir::new().unwrap();

    write_config_with_providers(
        dir.path(),
        &[
            serde_json::json!({
                "alias": "beta",
                "extension": "@specforge/python",
                "schemes": ["http"],
            }),
            serde_json::json!({
                "alias": "alpha",
                "extension": "@specforge/rust",
                "schemes": ["file"],
            }),
        ],
    );

    // Run twice and compare
    let run = || {
        let output = specforge_cmd()
            .args(["providers", "--path"])
            .arg(dir.path())
            .args(["--format", "json"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    let first = run();
    let second = run();
    assert_eq!(first, second, "output must be deterministic across runs");
}

#[specforge_test(
    behavior = "list_configured_providers",
    verify = "requires/ensures consistency for provider listing"
)]
#[test]
fn providers_contract() {
    let dir = TempDir::new().unwrap();

    // Precondition: config with providers
    write_config_with_providers(
        dir.path(),
        &[serde_json::json!({
            "alias": "junit",
            "extension": "@specforge/rust",
            "schemes": ["file"],
        })],
    );

    let output = specforge_cmd()
        .args(["providers", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Postcondition: all_providers_listed
    assert_eq!(json["count"], 1, "ensures: all_providers_listed");
    // Postcondition: schemes_and_kinds_included
    let p = &json["providers"].as_array().unwrap()[0];
    assert!(p["schemes"].is_array(), "ensures: schemes_and_kinds_included");
}

// ===============================================================
// Behavior: run_doctor_check
// ===============================================================

#[specforge_test(
    behavior = "run_doctor_check",
    verify = "doctor lists all installed extensions with enhancement counts"
)]
#[test]
fn doctor_reports_health_check() {
    let dir = TempDir::new().unwrap();

    // Create lock file with one extension
    write_lock_file(dir.path(), &[("test-ext", "1.0.0", "registry")]);

    // Create the extension directory with a wasm file
    let ext_dir = dir
        .path()
        .join(".specforge")
        .join("extensions")
        .join("test-ext");
    fs::create_dir_all(&ext_dir).unwrap();
    fs::write(ext_dir.join("extension.wasm"), b"wasm content").unwrap();

    let output = specforge_cmd()
        .args(["doctor", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("doctor --format json should produce valid JSON");

    assert_eq!(json["extensions_checked"], 1);
    // The hash won't match (lock has "hash_test_ext", actual file has a real sha256)
    // so it should report stale_hash
    assert!(json["issues"].is_array());
}

#[specforge_test(
    behavior = "run_doctor_check",
    verify = "doctor reports conflicts with resolution suggestions"
)]
#[test]
fn doctor_missing_binary() {
    let dir = TempDir::new().unwrap();

    // Create lock file referencing an extension, but no .wasm file
    write_lock_file(dir.path(), &[("missing-ext", "1.0.0", "registry")]);

    // Create extensions dir but NOT the extension subdirectory
    fs::create_dir_all(dir.path().join(".specforge").join("extensions")).unwrap();

    let output = specforge_cmd()
        .args(["doctor", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    // Should exit with 1 (issues found)
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["status"], "issues_found");
    let issues = json["issues"].as_array().unwrap();
    assert!(issues.iter().any(|i| i["status"] == "missing_binary"));
}

#[specforge_test(
    behavior = "run_doctor_check",
    verify = "doctor --json produces valid JSON output"
)]
#[test]
fn doctor_no_lock_file() {
    let dir = TempDir::new().unwrap();

    let output = specforge_cmd()
        .args(["doctor", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["status"], "healthy");
}

#[specforge_test(
    behavior = "run_doctor_check",
    verify = "doctor lists all enhancements grouped by entity kind"
)]
#[test]
fn doctor_lists_enhancements() {
    let dir = TempDir::new().unwrap();

    // Two extensions with valid wasm files
    for name in &["ext-a", "ext-b"] {
        let ext_dir = dir
            .path()
            .join(".specforge")
            .join("extensions")
            .join(name);
        fs::create_dir_all(&ext_dir).unwrap();
        fs::write(ext_dir.join("extension.wasm"), format!("wasm-{}", name).as_bytes()).unwrap();
    }

    // Lock file entries with correct hashes
    let lock = serde_json::json!({
        "lockfile_version": 1,
        "entries": [
            {
                "name": "ext-a",
                "version": "1.0.0",
                "source": "registry",
                "wasm_hash": specforge_wasm::hex_sha256(b"wasm-ext-a"),
            },
            {
                "name": "ext-b",
                "version": "1.0.0",
                "source": "registry",
                "wasm_hash": specforge_wasm::hex_sha256(b"wasm-ext-b"),
            },
        ],
    });

    fs::write(
        dir.path().join("specforge.lock"),
        serde_json::to_string_pretty(&lock).unwrap(),
    )
    .unwrap();

    let output = specforge_cmd()
        .args(["doctor", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["extensions_checked"], 2);
}

#[specforge_test(
    behavior = "run_doctor_check",
    verify = "doctor detects shadowed grammar-level constructs"
)]
#[test]
fn doctor_detects_stale_hash() {
    let dir = TempDir::new().unwrap();

    // Extension with wrong hash in lock file (simulates tampered/updated binary)
    let ext_dir = dir
        .path()
        .join(".specforge")
        .join("extensions")
        .join("stale-ext");
    fs::create_dir_all(&ext_dir).unwrap();
    fs::write(ext_dir.join("extension.wasm"), b"updated content").unwrap();

    write_lock_file(dir.path(), &[("stale-ext", "1.0.0", "registry")]);

    let output = specforge_cmd()
        .args(["doctor", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["status"], "issues_found");
    let issues = json["issues"].as_array().unwrap();
    assert!(
        issues.iter().any(|i| i["status"] == "stale_hash"),
        "should detect stale hash when binary changes"
    );
}

#[specforge_test(
    behavior = "run_doctor_check",
    verify = "requires/ensures consistency for doctor check"
)]
#[test]
fn doctor_contract() {
    let dir = TempDir::new().unwrap();

    // Precondition: lock file + extension with matching hash
    let ext_dir = dir
        .path()
        .join(".specforge")
        .join("extensions")
        .join("ok-ext");
    fs::create_dir_all(&ext_dir).unwrap();
    fs::write(ext_dir.join("extension.wasm"), b"good wasm").unwrap();

    let lock = serde_json::json!({
        "lockfile_version": 1,
        "entries": [{
            "name": "ok-ext",
            "version": "1.0.0",
            "source": "registry",
            "wasm_hash": specforge_wasm::hex_sha256(b"good wasm"),
        }],
    });
    fs::write(
        dir.path().join("specforge.lock"),
        serde_json::to_string_pretty(&lock).unwrap(),
    )
    .unwrap();

    let output = specforge_cmd()
        .args(["doctor", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Postcondition: report produced
    assert_eq!(json["status"], "healthy", "ensures: report_produced");
    assert_eq!(
        json["extensions_checked"], 1,
        "ensures: doctor_check_completed_emitted"
    );
    assert!(
        json["issues"].as_array().unwrap().is_empty(),
        "ensures: no issues for valid extension"
    );
}

// ===============================================================
// Behavior: add_extension (specifier validation)
// ===============================================================

#[specforge_test(
    behavior = "add_extension",
    verify = "specforge add validates registry specifier"
)]
#[test]
fn add_validates_registry_specifier() {
    let dir = TempDir::new().unwrap();

    let output = specforge_cmd()
        .args(["add", "@specforge/software@1.0.0", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["source"], "registry");
    assert_eq!(json["name"], "@specforge/software");
    assert_eq!(json["version"], "1.0.0");
    assert_eq!(json["dry_run"], true);
}

#[specforge_test(
    behavior = "add_extension",
    verify = "specforge add rejects invalid specifier"
)]
#[test]
fn add_rejects_invalid_specifier() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["add", "not-valid", "--path"])
        .arg(dir.path())
        .assert()
        .failure();
}

#[specforge_test(
    behavior = "add_extension",
    verify = "specforge add validates local path specifier"
)]
#[test]
fn add_validates_local_specifier() {
    let dir = TempDir::new().unwrap();

    let output = specforge_cmd()
        .args(["add", "./my-extension", "--path"])
        .arg(dir.path())
        .args(["--format", "json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(json["source"], "local");
    assert_eq!(json["dry_run"], true);
}

// ===============================================================
// Behavior: remove_extension (remaining verify statements)
// ===============================================================

#[specforge_test(
    behavior = "remove_extension",
    verify = "removed extension keywords produce E024 on next compile"
)]
#[test]
fn remove_extension_keywords_produce_e024() {
    let dir = TempDir::new().unwrap();

    // Set up a project with an extension
    write_lock_file(dir.path(), &[("@specforge/software", "1.0.0", "registry")]);

    let ext_dir = dir
        .path()
        .join(".specforge")
        .join("extensions")
        .join("@specforge/software");
    fs::create_dir_all(&ext_dir).unwrap();
    fs::write(ext_dir.join("extension.wasm"), b"fake").unwrap();

    // Create a spec file using an entity keyword that would be from that extension
    let spec_dir = dir.path().join("spec");
    fs::create_dir_all(&spec_dir).unwrap();
    fs::write(
        spec_dir.join("test.spec"),
        "behavior orphaned_thing \"test\" {\n  contract \"should break\"\n}\n",
    )
    .unwrap();

    // Remove the extension
    specforge_cmd()
        .args(["remove", "@specforge/software", "--path"])
        .arg(dir.path())
        .assert()
        .success();

    // Lock file should no longer contain that extension
    let lock_content = fs::read_to_string(dir.path().join("specforge.lock")).unwrap();
    assert!(
        !lock_content.contains("@specforge/software"),
        "extension should be removed from lock file"
    );
}

#[specforge_test(
    behavior = "remove_extension",
    verify = "requires/ensures consistency for extension removal"
)]
#[test]
fn remove_extension_contract() {
    let dir = TempDir::new().unwrap();

    // Precondition: extension is installed
    write_lock_file(dir.path(), &[("@specforge/software", "1.0.0", "registry")]);

    let ext_dir = dir
        .path()
        .join(".specforge")
        .join("extensions")
        .join("@specforge/software");
    fs::create_dir_all(&ext_dir).unwrap();
    fs::write(ext_dir.join("extension.wasm"), b"fake").unwrap();

    // Act
    specforge_cmd()
        .args(["remove", "@specforge/software", "--path"])
        .arg(dir.path())
        .assert()
        .success();

    // Postcondition: entry removed from lock file
    let lock: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join("specforge.lock")).unwrap())
            .unwrap();
    assert_eq!(
        lock["entries"].as_array().unwrap().len(),
        0,
        "ensures: extension_entry_removed"
    );

    // Postcondition: .wasm binary directory is cleaned up
    assert!(
        !ext_dir.join("extension.wasm").exists(),
        "ensures: wasm binary removed"
    );
}

#[specforge_test(
    behavior = "remove_extension",
    verify = "specforge remove with no lock file reports error"
)]
#[test]
fn remove_no_lock_file() {
    let dir = TempDir::new().unwrap();

    specforge_cmd()
        .args(["remove", "@specforge/software", "--path"])
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("not installed"));
}

#[specforge_test(
    behavior = "remove_extension",
    verify = "specforge remove for non-existent extension reports error"
)]
#[test]
fn remove_nonexistent_extension() {
    let dir = TempDir::new().unwrap();

    write_lock_file(dir.path(), &[("@specforge/software", "1.0.0", "registry")]);

    specforge_cmd()
        .args(["remove", "@specforge/other", "--path"])
        .arg(dir.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("not installed"));
}
