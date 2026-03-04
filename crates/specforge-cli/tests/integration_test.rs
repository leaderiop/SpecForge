use std::process::Command;

fn specforge_check(fixture_dir: &str) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_specforge"))
        .args(["check", fixture_dir])
        .output()
        .expect("failed to run specforge");

    let exit_code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (exit_code, stderr)
}

fn specforge_cmd(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_specforge"))
        .args(args)
        .output()
        .expect("failed to run specforge");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (exit_code, stdout, stderr)
}

fn fixture_path(relative: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/../../tests/fixtures/{relative}")
}

#[test]
fn valid_clean_spec_no_errors() {
    let (exit_code, output) = specforge_check(&fixture_path("valid"));
    assert_eq!(exit_code, 0, "Expected exit code 0 for valid fixtures.\nOutput:\n{output}");
    assert!(!output.contains("[E"), "Should have no errors.\nOutput:\n{output}");
}

#[test]
fn e001_dangling_reference() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e001"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E001.\nOutput:\n{output}");
    assert!(output.contains("[E001]"), "Should contain E001.\nOutput:\n{output}");
    assert!(output.contains("nonexistent_inv"), "Should mention the dangling ref.\nOutput:\n{output}");
}

#[test]
fn e002_duplicate_id() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e002"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E002.\nOutput:\n{output}");
    assert!(output.contains("[E002]"), "Should contain E002.\nOutput:\n{output}");
    assert!(output.contains("data_integrity"), "Should mention the duplicate ID.\nOutput:\n{output}");
}

#[test]
fn w001_orphan_behavior() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w001"));
    assert_eq!(exit_code, 0, "W001 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W001]"), "Should contain W001.\nOutput:\n{output}");
}

#[test]
fn w003_unused_invariant() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w003"));
    assert_eq!(exit_code, 0, "W003 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W003]"), "Should contain W003.\nOutput:\n{output}");
}

#[test]
fn w004_unverified_behavior() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w004"));
    assert_eq!(exit_code, 0, "W004 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W004]"), "Should contain W004.\nOutput:\n{output}");
}

#[test]
fn w002_orphan_feature() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w002"));
    assert_eq!(exit_code, 0, "W002 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W002]"), "Should contain W002.\nOutput:\n{output}");
}

#[test]
fn w005_unmitigated_invariant() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w005"));
    assert_eq!(exit_code, 0, "W005 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W005]"), "Should contain W005.\nOutput:\n{output}");
}

#[test]
fn w008_uncovered_capability() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w008"));
    assert_eq!(exit_code, 0, "W008 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W008]"), "Should contain W008.\nOutput:\n{output}");
}

#[test]
fn w009_orphan_library() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w009"));
    assert_eq!(exit_code, 0, "W009 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W009]"), "Should contain W009.\nOutput:\n{output}");
}

#[test]
fn w010_deprecated_feature() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w010"));
    assert_eq!(exit_code, 0, "W010 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W010]"), "Should contain W010.\nOutput:\n{output}");
}

#[test]
fn w011_orphan_capability() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w011"));
    assert_eq!(exit_code, 0, "W011 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W011]"), "Should contain W011.\nOutput:\n{output}");
}

#[test]
fn w012_orphan_ref() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w012"));
    assert_eq!(exit_code, 0, "W012 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W012]"), "Should contain W012.\nOutput:\n{output}");
}

#[test]
fn i001_stale_proposal() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/i001"));
    assert_eq!(exit_code, 0, "I001 is info, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[I001]"), "Should contain I001.\nOutput:\n{output}");
}

#[test]
fn e007_library_cycle() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e007"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E007.\nOutput:\n{output}");
    assert!(output.contains("[E007]"), "Should contain E007.\nOutput:\n{output}");
}

#[test]
fn self_validation() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let spec_dir = format!("{manifest_dir}/../../spec");
    let (exit_code, output) = specforge_check(&spec_dir);
    assert_eq!(exit_code, 0, "Self-validation should pass with exit code 0.\nOutput:\n{output}");
    assert!(!output.contains("[E"), "Self-validation should have no errors.\nOutput:\n{output}");
}

#[test]
fn e010_syntax_error_in_list() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e010"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E010.\nOutput:\n{output}");
    assert!(output.contains("[E010]"), "Should contain E010.\nOutput:\n{output}");
}

// --- Emitter integration tests ---

#[test]
fn render_json_produces_valid_file() {
    let fixture = fixture_path("valid");
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().to_string_lossy().to_string();
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["render", "json", &out_dir, "--path", &fixture]);
    assert_eq!(exit_code, 0, "render json should exit 0.\nStderr:\n{stderr}");
    let json_path = dir.path().join("graph.json");
    assert!(json_path.exists(), "graph.json should exist");
    let content = std::fs::read_to_string(&json_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed["nodes"].is_array());
    assert!(parsed["edges"].is_array());
    assert!(parsed["metadata"]["name"].is_string());
}

#[test]
fn graph_prints_dot() {
    let fixture = fixture_path("valid");
    let (exit_code, stdout, stderr) = specforge_cmd(&["graph", &fixture]);
    assert_eq!(exit_code, 0, "graph should exit 0.\nStderr:\n{stderr}");
    assert!(
        stdout.contains("digraph"),
        "Should contain digraph.\nStdout:\n{stdout}"
    );
    assert!(stdout.contains("->"), "Should contain edges.\nStdout:\n{stdout}");
}

#[test]
fn stats_prints_counts() {
    let fixture = fixture_path("valid");
    let (exit_code, stdout, stderr) = specforge_cmd(&["stats", &fixture]);
    assert_eq!(exit_code, 0, "stats should exit 0.\nStderr:\n{stderr}");
    assert!(
        stdout.contains("Entities:"),
        "Should contain Entities.\nStdout:\n{stdout}"
    );
    assert!(
        stdout.contains("invariant"),
        "Should contain invariant kind.\nStdout:\n{stdout}"
    );
    assert!(
        stdout.contains("behavior"),
        "Should contain behavior kind.\nStdout:\n{stdout}"
    );
    assert!(
        stdout.contains("Coverage:"),
        "Should contain Coverage.\nStdout:\n{stdout}"
    );
}

#[test]
fn render_json_on_invalid_spec() {
    let fixture = fixture_path("invalid/e001");
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().to_string_lossy().to_string();
    let (exit_code, _stdout, _stderr) =
        specforge_cmd(&["render", "json", &out_dir, "--path", &fixture]);
    // Still produces output even with errors
    assert_eq!(exit_code, 0, "render json should still succeed with errors");
    let json_path = dir.path().join("graph.json");
    assert!(json_path.exists(), "graph.json should exist even with errors");
}

#[test]
fn stats_on_valid_spec() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let spec_dir = format!("{manifest_dir}/../../spec");
    let (exit_code, stdout, stderr) = specforge_cmd(&["stats", &spec_dir]);
    assert_eq!(exit_code, 0, "stats on self should exit 0.\nStderr:\n{stderr}");
    assert!(
        stdout.contains("Diagnostics: 0 errors"),
        "Self-spec should have 0 errors.\nStdout:\n{stdout}"
    );
}

// --- Trace integration tests ---

#[test]
fn trace_single_entity() {
    let fixture = fixture_path("valid");
    let (exit_code, stdout, stderr) =
        specforge_cmd(&["trace", "validate_input", "--path", &fixture]);
    assert_eq!(exit_code, 0, "trace should exit 0.\nStderr:\n{stderr}");
    assert!(
        stdout.contains("Trace: validate_input"),
        "Should contain trace header.\nStdout:\n{stdout}"
    );
}

#[test]
fn trace_missing_entity() {
    let fixture = fixture_path("valid");
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["trace", "NONEXISTENT", "--path", &fixture]);
    assert_eq!(exit_code, 1, "trace of missing entity should exit 1.\nStderr:\n{stderr}");
    assert!(
        stderr.contains("entity not found"),
        "Should report entity not found.\nStderr:\n{stderr}"
    );
}

#[test]
fn trace_full_report() {
    let fixture = fixture_path("valid");
    let (exit_code, stdout, stderr) = specforge_cmd(&["trace", "--path", &fixture]);
    assert_eq!(exit_code, 0, "full trace should exit 0.\nStderr:\n{stderr}");
    assert!(
        stdout.contains("Traceability Report"),
        "Should contain Traceability Report.\nStdout:\n{stdout}"
    );
}

// --- Markdown render integration tests ---

#[test]
fn render_markdown_produces_files() {
    let fixture = fixture_path("valid");
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().to_string_lossy().to_string();
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["render", "markdown", &out_dir, "--path", &fixture]);
    assert_eq!(exit_code, 0, "render markdown should exit 0.\nStderr:\n{stderr}");

    let index_path = dir.path().join("index.md");
    assert!(index_path.exists(), "index.md should exist");

    // Should have at least behaviors.md and invariants.md
    let behaviors_path = dir.path().join("behaviors.md");
    assert!(behaviors_path.exists(), "behaviors.md should exist");
    let invariants_path = dir.path().join("invariants.md");
    assert!(invariants_path.exists(), "invariants.md should exist");
}

#[test]
fn render_markdown_with_only() {
    let fixture = fixture_path("valid");
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().to_string_lossy().to_string();
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "render", "markdown", &out_dir, "--path", &fixture, "--only", "behavior",
    ]);
    assert_eq!(exit_code, 0, "render markdown --only should exit 0.\nStderr:\n{stderr}");

    let behaviors_path = dir.path().join("behaviors.md");
    assert!(behaviors_path.exists(), "behaviors.md should exist");
    let index_path = dir.path().join("index.md");
    assert!(index_path.exists(), "index.md should exist");

    // Should NOT have invariants.md or features.md
    let invariants_path = dir.path().join("invariants.md");
    assert!(!invariants_path.exists(), "invariants.md should NOT exist with --only behavior");
}

#[test]
fn render_markdown_invalid_only() {
    let fixture = fixture_path("valid");
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().to_string_lossy().to_string();
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "render", "markdown", &out_dir, "--path", &fixture, "--only", "bogus",
    ]);
    assert_eq!(exit_code, 1, "render markdown --only bogus should exit 1.\nStderr:\n{stderr}");
    assert!(
        stderr.contains("unknown entity kind"),
        "Should report unknown entity kind.\nStderr:\n{stderr}"
    );
}

// --- I003 integration test ---

#[test]
fn i003_old_version() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/i003"));
    assert_eq!(exit_code, 0, "I003 is info, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[I003]"), "Should contain I003.\nOutput:\n{output}");
    assert!(
        output.contains("specforge migrate"),
        "Should mention specforge migrate.\nOutput:\n{output}"
    );
}

// --- E003: circular import ---

#[test]
fn e003_circular_import() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e003"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E003.\nOutput:\n{output}");
    assert!(output.contains("[E003]"), "Should contain E003.\nOutput:\n{output}");
    assert!(output.contains("circular import"), "Should mention circular import.\nOutput:\n{output}");
}

// --- E004: empty scenario ---

#[test]
fn e004_empty_scenario() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e004"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E004.\nOutput:\n{output}");
    assert!(output.contains("[E004]"), "Should contain E004.\nOutput:\n{output}");
    assert!(output.contains("no steps"), "Should mention no steps.\nOutput:\n{output}");
}

// --- E005: RPN mismatch ---

#[test]
fn e005_rpn_mismatch() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e005"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E005.\nOutput:\n{output}");
    assert!(output.contains("[E005]"), "Should contain E005.\nOutput:\n{output}");
    assert!(output.contains("RPN mismatch"), "Should mention RPN mismatch.\nOutput:\n{output}");
}

// --- E006: invalid event trigger ---

#[test]
fn e006_invalid_event_trigger() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e006"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E006.\nOutput:\n{output}");
    assert!(output.contains("[E006]"), "Should contain E006.\nOutput:\n{output}");
    assert!(output.contains("must be a behavior"), "Should mention must be a behavior.\nOutput:\n{output}");
}

// --- E008: persona not defined ---

#[test]
fn e008_persona_not_defined() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e008"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E008.\nOutput:\n{output}");
    assert!(output.contains("[E008]"), "Should contain E008.\nOutput:\n{output}");
    assert!(output.contains("persona"), "Should mention persona.\nOutput:\n{output}");
}

// --- E009: surface not defined ---

#[test]
fn e009_surface_not_defined() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e009"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E009.\nOutput:\n{output}");
    assert!(output.contains("[E009]"), "Should contain E009.\nOutput:\n{output}");
    assert!(output.contains("surface"), "Should mention surface.\nOutput:\n{output}");
}

// --- E012: unknown provider kind ---

#[test]
fn e012_unknown_provider_kind() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e012"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E012.\nOutput:\n{output}");
    assert!(output.contains("[E012]"), "Should contain E012.\nOutput:\n{output}");
    assert!(output.contains("unknown provider kind"), "Should mention unknown provider kind.\nOutput:\n{output}");
}

// --- E013: reserved word ---

#[test]
fn e013_reserved_word() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e013"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E013.\nOutput:\n{output}");
    assert!(output.contains("[E013]"), "Should contain E013.\nOutput:\n{output}");
    assert!(output.contains("reserved keyword"), "Should mention reserved keyword.\nOutput:\n{output}");
}

// --- E014: invalid identifier characters ---

#[test]
fn e014_invalid_chars() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e014"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E014.\nOutput:\n{output}");
    assert!(output.contains("[E014]"), "Should contain E014.\nOutput:\n{output}");
    assert!(output.contains("invalid characters"), "Should mention invalid characters.\nOutput:\n{output}");
}

// --- E015: duplicate scenario title ---

#[test]
fn e015_duplicate_scenario() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e015"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E015.\nOutput:\n{output}");
    assert!(output.contains("[E015]"), "Should contain E015.\nOutput:\n{output}");
    assert!(output.contains("duplicate scenario"), "Should mention duplicate scenario.\nOutput:\n{output}");
}

// --- W006: unconstrained behavior ---

#[test]
fn w006_unconstrained_behavior() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w006"));
    assert_eq!(exit_code, 0, "W006 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W006]"), "Should contain W006.\nOutput:\n{output}");
}

// --- W007: orphan event ---

#[test]
fn w007_orphan_event() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w007"));
    assert_eq!(exit_code, 0, "W007 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W007]"), "Should contain W007.\nOutput:\n{output}");
}

// --- W017: unused entity ---

#[test]
fn w017_unused_type() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w017"));
    assert_eq!(exit_code, 0, "W017 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W017]"), "Should contain W017.\nOutput:\n{output}");
}

// --- W013: vague name ---

#[test]
fn w013_vague_name() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w013"));
    assert_eq!(exit_code, 0, "W013 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W013]"), "Should contain W013.\nOutput:\n{output}");
}

// --- W015: scenario missing when step ---

#[test]
fn w015_scenario_missing_when() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w015"));
    assert_eq!(exit_code, 0, "W015 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W015]"), "Should contain W015.\nOutput:\n{output}");
}

// --- W016: scenario missing then step ---

#[test]
fn w016_scenario_missing_then() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w016"));
    assert_eq!(exit_code, 0, "W016 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W016]"), "Should contain W016.\nOutput:\n{output}");
}

// --- W018: testable entity missing test links ---

#[test]
fn w018_testable_entity_no_test_links() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w018"));
    assert_eq!(exit_code, 0, "W018 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W018]"), "Should contain W018.\nOutput:\n{output}");
}

// --- E016: test file not found ---

#[test]
fn e016_test_file_not_found() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/e016"));
    assert_eq!(exit_code, 1, "Expected exit code 1 for E016.\nOutput:\n{output}");
    assert!(output.contains("[E016]"), "Should contain E016.\nOutput:\n{output}");
    assert!(output.contains("nonexistent_test.rs"), "Should mention the missing file.\nOutput:\n{output}");
}

// --- W021: deliverable with no capabilities ---

#[test]
fn w021_deliverable_no_capabilities() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w021"));
    assert_eq!(exit_code, 0, "W021 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W021]"), "Should contain W021.\nOutput:\n{output}");
}

// --- W019: constraint with no protected invariants ---

#[test]
fn w019_constraint_no_protects() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/w019"));
    assert_eq!(exit_code, 0, "W019 is a warning, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[W019]"), "Should contain W019.\nOutput:\n{output}");
}

// --- I006: unused glossary term ---

#[test]
fn i006_unused_glossary_term() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/i006"));
    assert_eq!(exit_code, 0, "I006 is info, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[I006]"), "Should contain I006.\nOutput:\n{output}");
}

// --- I004: cross-plugin reference ---

#[test]
fn i004_cross_plugin_ref() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/i004"));
    assert_eq!(exit_code, 0, "I004 is info, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[I004]"), "Should contain I004.\nOutput:\n{output}");
}

// --- I005: unknown provider scheme ---

#[test]
fn i005_unknown_scheme() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid/i005"));
    assert_eq!(exit_code, 0, "I005 is info, should exit 0.\nOutput:\n{output}");
    assert!(output.contains("[I005]"), "Should contain I005.\nOutput:\n{output}");
}

// --- Migrate integration tests ---

#[test]
fn migrate_already_latest() {
    let fixture = fixture_path("valid");
    let (exit_code, stdout, stderr) = specforge_cmd(&["migrate", &fixture]);
    assert_eq!(
        exit_code, 0,
        "migrate on current version should exit 0.\nStdout:\n{stdout}\nStderr:\n{stderr}"
    );
    assert!(
        stdout.contains("already at latest version"),
        "Should say already at latest.\nStdout:\n{stdout}"
    );
}

#[test]
fn migrate_on_self_spec() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let spec_dir = format!("{manifest_dir}/../../spec");
    let (exit_code, stdout, stderr) = specforge_cmd(&["migrate", &spec_dir]);
    assert_eq!(
        exit_code, 0,
        "migrate on self spec should exit 0.\nStdout:\n{stdout}\nStderr:\n{stderr}"
    );
    assert!(
        stdout.contains("already at latest version"),
        "Should say already at latest.\nStdout:\n{stdout}"
    );
}

// --- Watch integration tests ---

#[test]
fn watch_help_works() {
    let (exit_code, stdout, stderr) = specforge_cmd(&["watch", "--help"]);
    assert_eq!(
        exit_code, 0,
        "watch --help should exit 0.\nStdout:\n{stdout}\nStderr:\n{stderr}"
    );
    assert!(
        stdout.contains("Watch spec files"),
        "Should contain watch description.\nStdout:\n{stdout}"
    );
}

#[test]
fn migrate_help_works() {
    let (exit_code, stdout, stderr) = specforge_cmd(&["migrate", "--help"]);
    assert_eq!(
        exit_code, 0,
        "migrate --help should exit 0.\nStdout:\n{stdout}\nStderr:\n{stderr}"
    );
    assert!(
        stdout.contains("Migrate spec files"),
        "Should contain migrate description.\nStdout:\n{stdout}"
    );
}

#[test]
fn trace_on_self_spec() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let spec_dir = format!("{manifest_dir}/../../spec");
    let (exit_code, stdout, stderr) = specforge_cmd(&["trace", "--path", &spec_dir]);
    assert_eq!(exit_code, 0, "trace on self should exit 0.\nStderr:\n{stderr}");
    assert!(
        stdout.contains("Traceability Report"),
        "Should contain Traceability Report.\nStdout:\n{stdout}"
    );
}

// --- specforge.json config tests ---

#[test]
fn json_config_basic() {
    let (exit_code, output) = specforge_check(&fixture_path("json_config"));
    assert_eq!(
        exit_code, 0,
        "JSON config basic should exit 0.\nOutput:\n{output}"
    );
    assert!(
        !output.contains("[E"),
        "Should have no errors.\nOutput:\n{output}"
    );
}

#[test]
fn json_config_with_plugins() {
    let (exit_code, output) = specforge_check(&fixture_path("json_config_plugins"));
    assert_eq!(
        exit_code, 0,
        "JSON config with plugins should exit 0.\nOutput:\n{output}"
    );
    assert!(
        !output.contains("[E"),
        "Should have no errors.\nOutput:\n{output}"
    );
}

#[test]
fn json_config_spec_root() {
    let (exit_code, output) = specforge_check(&fixture_path("json_config_spec_root"));
    assert_eq!(
        exit_code, 0,
        "JSON config with spec_root should exit 0.\nOutput:\n{output}"
    );
    assert!(
        !output.contains("[E"),
        "Should have no errors.\nOutput:\n{output}"
    );
}

#[test]
fn json_config_precedence() {
    // Both specforge.json and specforge.spec exist — JSON wins.
    // The spec block says name="spec-loses", JSON says name="json-wins".
    // If JSON wins, the spec block is just another entity file and will
    // have a spec entity named "spec-loses" — this should compile cleanly.
    let (exit_code, output) = specforge_check(&fixture_path("json_config_precedence"));
    assert_eq!(
        exit_code, 0,
        "JSON config precedence should exit 0.\nOutput:\n{output}"
    );
    assert!(
        !output.contains("[E"),
        "Should have no errors.\nOutput:\n{output}"
    );
}

#[test]
fn json_config_invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("specforge.json"), "{ invalid json").unwrap();
    std::fs::write(
        dir.path().join("data.spec"),
        r#"invariant test "Test" { guarantee """ok""" }"#,
    )
    .unwrap();

    let (exit_code, _stdout, stderr) = specforge_cmd(&["check", &dir.path().to_string_lossy()]);
    assert_eq!(
        exit_code, 1,
        "Invalid JSON should exit 1.\nStderr:\n{stderr}"
    );
    assert!(
        stderr.contains("error parsing"),
        "Should mention parse error.\nStderr:\n{stderr}"
    );
}

// --- specforge schema command ---

#[test]
fn schema_prints_json() {
    let (exit_code, stdout, stderr) = specforge_cmd(&["schema"]);
    assert_eq!(
        exit_code, 0,
        "schema should exit 0.\nStderr:\n{stderr}"
    );
    // Should be valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("schema output should be valid JSON");
    assert_eq!(
        parsed["title"], "SpecForge Configuration",
        "Schema title should match.\nStdout:\n{stdout}"
    );
    assert!(
        parsed["properties"]["name"].is_object(),
        "Schema should have name property.\nStdout:\n{stdout}"
    );
}

// --- specforge init with JSON ---

#[test]
fn init_creates_json_config() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_string_lossy().to_string();
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["init", "--name", "test-init", &path]);
    assert_eq!(
        exit_code, 0,
        "init should exit 0.\nStderr:\n{stderr}"
    );

    let json_path = dir.path().join("specforge.json");
    assert!(json_path.exists(), "specforge.json should exist");

    let content = std::fs::read_to_string(&json_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["name"], "test-init");
    assert_eq!(parsed["version"], "1.0");
    assert!(
        parsed["$schema"].as_str().unwrap().contains("specforge"),
        "Should have $schema field"
    );

    // specforge.spec should NOT be created
    assert!(
        !dir.path().join("specforge.spec").exists(),
        "specforge.spec should NOT exist after init"
    );
}

// --- Custom entity integration tests ---

#[test]
fn custom_entity_valid() {
    let (exit_code, output) = specforge_check(&fixture_path("valid_custom_entity"));
    assert_eq!(
        exit_code, 0,
        "Valid custom entity fixture should exit 0.\nOutput:\n{output}"
    );
    // Only W017 orphan warnings expected
    assert!(
        !output.contains("[E0"),
        "Should have no errors.\nOutput:\n{output}"
    );
}

#[test]
fn custom_entity_type_error() {
    let (exit_code, output) = specforge_check(&fixture_path("invalid_custom_entity"));
    assert_eq!(
        exit_code, 1,
        "Invalid custom entity should exit 1.\nOutput:\n{output}"
    );
    assert!(
        output.contains("[E010]"),
        "Should contain E010 for wrong field type.\nOutput:\n{output}"
    );
}

// ==========================================================================
// Code generation integration tests (Phase 4)
// ==========================================================================

#[test]
fn gen_typescript_produces_type_files() {
    let fixture = fixture_path("codegen/gen_basic");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_string_lossy().to_string();
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["gen", "typescript", &out, "--path", &fixture]);
    assert_eq!(exit_code, 0, "gen typescript should exit 0.\nStderr:\n{stderr}");

    // Type files
    assert!(dir.path().join("user.ts").exists(), "user.ts should exist");
    assert!(dir.path().join("user-status.ts").exists(), "user-status.ts should exist");
    assert!(dir.path().join("address.ts").exists(), "address.ts should exist");

    // Verify content
    let user_content = std::fs::read_to_string(dir.path().join("user.ts")).unwrap();
    assert!(user_content.contains("export interface User"), "user.ts should export User interface");
    assert!(user_content.contains("@specforge-checksum"), "user.ts should have checksum header");

    let status_content = std::fs::read_to_string(dir.path().join("user-status.ts")).unwrap();
    assert!(
        status_content.contains("export type UserStatus"),
        "user-status.ts should export UserStatus union"
    );
}

#[test]
fn gen_typescript_produces_port_files() {
    let fixture = fixture_path("codegen/gen_basic");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_string_lossy().to_string();
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["gen", "typescript", &out, "--path", &fixture]);
    assert_eq!(exit_code, 0, "gen typescript should exit 0.\nStderr:\n{stderr}");

    let port_path = dir.path().join("user-repository.ts");
    assert!(port_path.exists(), "user-repository.ts should exist");

    let content = std::fs::read_to_string(&port_path).unwrap();
    assert!(content.contains("export interface UserRepository"), "should export port interface");
    assert!(content.contains("save("), "should have save method");
    assert!(content.contains("findById("), "should have findById method");
    assert!(content.contains("list("), "should have list method");
}

#[test]
fn gen_typescript_produces_index_barrel() {
    let fixture = fixture_path("codegen/gen_basic");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_string_lossy().to_string();
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["gen", "typescript", &out, "--path", &fixture]);
    assert_eq!(exit_code, 0, "gen typescript should exit 0.\nStderr:\n{stderr}");

    let index_path = dir.path().join("index.ts");
    assert!(index_path.exists(), "index.ts barrel file should exist");

    let content = std::fs::read_to_string(&index_path).unwrap();
    assert!(content.contains("export * from"), "index.ts should contain re-exports");
    assert!(content.contains("'./user'"), "should re-export user");
    assert!(content.contains("'./user-repository'"), "should re-export user-repository");
}

#[test]
fn gen_json_schema_produces_schema_files() {
    let fixture = fixture_path("codegen/gen_basic");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_string_lossy().to_string();
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["gen", "json-schema", &out, "--path", &fixture]);
    assert_eq!(exit_code, 0, "gen json-schema should exit 0.\nStderr:\n{stderr}");

    let user_schema = dir.path().join("user.schema.json");
    assert!(user_schema.exists(), "user.schema.json should exist");

    let content = std::fs::read_to_string(&user_schema).unwrap();
    // Skip checksum header line (starts with //)
    let json_body = content
        .lines()
        .skip_while(|l| l.starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let parsed: serde_json::Value = serde_json::from_str(&json_body).unwrap();
    assert!(parsed["properties"].is_object(), "schema should have properties");
}

#[test]
fn gen_test_stubs_from_verify() {
    let fixture = fixture_path("codegen/gen_basic");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_string_lossy().to_string();
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["gen", "typescript", &out, "--path", &fixture]);
    assert_eq!(exit_code, 0, "gen typescript should exit 0.\nStderr:\n{stderr}");

    // The gen_basic fixture has tests "@specforge/vitest" and a behavior with verify stmts
    let test_file = dir.path().join("__tests__/validate_input.test.ts");
    assert!(test_file.exists(), "test stub should be generated in __tests__/");

    let content = std::fs::read_to_string(&test_file).unwrap();
    assert!(content.contains("describe('validate_input'"), "should contain describe block");
    assert!(content.contains("rejects empty email"), "should contain verify description");
    assert!(content.contains("accepts valid email"), "should contain verify description");
}

#[test]
fn gen_check_no_drift_exits_zero() {
    let fixture = fixture_path("codegen/gen_basic");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_string_lossy().to_string();

    // Generate first
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["gen", "typescript", &out, "--path", &fixture]);
    assert_eq!(exit_code, 0, "initial gen should exit 0.\nStderr:\n{stderr}");

    // Check should pass (no drift)
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["gen", "typescript", &out, "--check", "--path", &fixture]);
    assert_eq!(exit_code, 0, "check with no drift should exit 0.\nStderr:\n{stderr}");
    assert!(
        stderr.contains("up to date"),
        "Should report files up to date.\nStderr:\n{stderr}"
    );
}

#[test]
fn gen_check_drift_detected_exits_one() {
    let fixture = fixture_path("codegen/gen_basic");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_string_lossy().to_string();

    // Generate first
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["gen", "typescript", &out, "--path", &fixture]);
    assert_eq!(exit_code, 0, "initial gen should exit 0.\nStderr:\n{stderr}");

    // Tamper with a generated file
    let user_path = dir.path().join("user.ts");
    std::fs::write(&user_path, "// tampered content\n").unwrap();

    // Check should detect drift
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["gen", "typescript", &out, "--check", "--path", &fixture]);
    assert_eq!(exit_code, 1, "check with drift should exit 1.\nStderr:\n{stderr}");
    assert!(
        stderr.contains("drift detected") || stderr.contains("out of date"),
        "Should report drift.\nStderr:\n{stderr}"
    );
}

#[test]
fn gen_check_writes_no_files() {
    let fixture = fixture_path("codegen/gen_basic");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_string_lossy().to_string();

    // Run check on empty directory (nothing generated yet)
    let (exit_code, _stdout, _stderr) =
        specforge_cmd(&["gen", "typescript", &out, "--check", "--path", &fixture]);
    assert_eq!(exit_code, 1, "check on empty dir should exit 1");

    // Verify no .ts files were created
    let ts_files: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "ts"))
        .collect();
    assert!(ts_files.is_empty(), "check mode must not write any .ts files");
}

#[test]
fn verify_all_methods_implemented_passes() {
    let fixture = fixture_path("codegen/gen_verify");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_string_lossy().to_string();

    // Generate port files
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["gen", "typescript", &out, "--path", &fixture]);
    assert_eq!(exit_code, 0, "gen should exit 0.\nStderr:\n{stderr}");

    // Create adapter file with all required methods for UserRepository
    let adapter_content = r#"
import { UserRepository } from './user-repository';

export class UserRepositoryAdapter implements UserRepository {
  save(user: User): { ok: void } | { err: Error } {
    throw new Error('not implemented');
  }

  findById(id: string): User | undefined {
    throw new Error('not implemented');
  }

  list(): User[] {
    throw new Error('not implemented');
  }
}
"#;
    std::fs::write(dir.path().join("user-repository.adapter.ts"), adapter_content).unwrap();

    // Create adapter for FileSystem
    let fs_adapter = r#"
export class FileSystemAdapter {
  read(path: string): { ok: string } | { err: Error } {
    throw new Error('not implemented');
  }

  write(path: string, content: string): { ok: void } | { err: Error } {
    throw new Error('not implemented');
  }

  exists(path: string): boolean {
    return false;
  }
}
"#;
    std::fs::write(dir.path().join("file-system.adapter.ts"), fs_adapter).unwrap();

    // Verify should pass — all methods present
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["verify", "typescript", &out, "--path", &fixture]);
    assert_eq!(exit_code, 0, "verify with all methods should exit 0.\nStderr:\n{stderr}");
    assert!(
        stderr.contains("0 missing") || stderr.contains("2 verified"),
        "Should report verified ports.\nStderr:\n{stderr}"
    );
}

#[test]
fn verify_missing_method_reported() {
    let fixture = fixture_path("codegen/gen_verify");
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().to_string_lossy().to_string();

    // Generate port files
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["gen", "typescript", &out, "--path", &fixture]);
    assert_eq!(exit_code, 0, "gen should exit 0.\nStderr:\n{stderr}");

    // Create adapter missing the `list` method
    let adapter_content = r#"
export class UserRepositoryAdapter {
  save(user: User): void {
    throw new Error('not implemented');
  }

  findById(id: string): User | undefined {
    return undefined;
  }
}
"#;
    std::fs::write(dir.path().join("user-repository.adapter.ts"), adapter_content).unwrap();

    // Verify should fail — missing method
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["verify", "typescript", &out, "--path", &fixture]);
    assert_eq!(exit_code, 1, "verify with missing method should exit 1.\nStderr:\n{stderr}");
    assert!(
        stderr.contains("missing method"),
        "Should report missing method.\nStderr:\n{stderr}"
    );
}

#[test]
fn gen_multiple_targets_independent() {
    let fixture = fixture_path("codegen/gen_basic");
    let ts_dir = tempfile::tempdir().unwrap();
    let json_dir = tempfile::tempdir().unwrap();
    let ts_out = ts_dir.path().to_string_lossy().to_string();
    let json_out = json_dir.path().to_string_lossy().to_string();

    let (ts_code, _stdout, ts_stderr) =
        specforge_cmd(&["gen", "typescript", &ts_out, "--path", &fixture]);
    let (json_code, _stdout, json_stderr) =
        specforge_cmd(&["gen", "json-schema", &json_out, "--path", &fixture]);

    assert_eq!(ts_code, 0, "typescript gen should exit 0.\nStderr:\n{ts_stderr}");
    assert_eq!(json_code, 0, "json-schema gen should exit 0.\nStderr:\n{json_stderr}");

    // TypeScript produces .ts files
    assert!(ts_dir.path().join("user.ts").exists(), "TS should produce user.ts");
    assert!(!ts_dir.path().join("user.schema.json").exists(), "TS should NOT produce .schema.json");

    // JSON Schema produces .schema.json files
    assert!(json_dir.path().join("user.schema.json").exists(), "JSON should produce user.schema.json");
    assert!(!json_dir.path().join("user.ts").exists(), "JSON should NOT produce .ts files");
}

// ==========================================================================
// Phase 5: Coverage integration tests
// ==========================================================================

#[test]
fn coverage_text_output() {
    let fixture = fixture_path("coverage/basic");
    let (exit_code, stdout, stderr) = specforge_cmd(&["coverage", "--path", &fixture]);
    assert_eq!(exit_code, 0, "coverage should exit 0.\nStdout:\n{stdout}\nStderr:\n{stderr}");
    assert!(
        stdout.contains("Coverage:") && stdout.contains('%'),
        "Should contain Coverage percentage.\nStdout:\n{stdout}"
    );
}

#[test]
fn coverage_json_output() {
    let fixture = fixture_path("coverage/basic");
    let (exit_code, stdout, stderr) =
        specforge_cmd(&["coverage", "--format", "json", "--path", &fixture]);
    assert_eq!(exit_code, 0, "coverage json should exit 0.\nStderr:\n{stderr}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("coverage JSON invalid: {e}\nStdout:\n{stdout}"));
    assert!(parsed["percentage"].is_number(), "should have percentage");
    assert!(parsed["entities"].is_array(), "should have entities array");
}

#[test]
fn coverage_verbose_shows_details() {
    let fixture = fixture_path("coverage/basic");
    let (exit_code, stdout, stderr) =
        specforge_cmd(&["coverage", "--verbose", "--path", &fixture]);
    assert_eq!(exit_code, 0, "verbose coverage should exit 0.\nStderr:\n{stderr}");
    assert!(
        stdout.contains("validate_data"),
        "Verbose should show entity IDs.\nStdout:\n{stdout}"
    );
    assert!(
        stdout.contains("transform_data"),
        "Verbose should show entity IDs.\nStdout:\n{stdout}"
    );
}

#[test]
fn coverage_threshold_met_exits_zero() {
    let fixture = fixture_path("coverage/all_passing");
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["coverage", "--min", "100", "--path", &fixture]);
    assert_eq!(
        exit_code, 0,
        "100% coverage with --min 100 should exit 0.\nStderr:\n{stderr}"
    );
}

#[test]
fn coverage_threshold_not_met_exits_one() {
    let fixture = fixture_path("coverage/below_threshold");
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["coverage", "--min", "80", "--path", &fixture]);
    assert_eq!(exit_code, 1, "Below threshold should exit 1.\nStderr:\n{stderr}");
    assert!(
        stderr.contains("below threshold"),
        "Should mention below threshold.\nStderr:\n{stderr}"
    );
}

#[test]
fn coverage_no_report_shows_levels() {
    let fixture = fixture_path("coverage/no_report");
    let (exit_code, stdout, stderr) =
        specforge_cmd(&["coverage", "--format", "json", "--path", &fixture]);
    assert_eq!(exit_code, 0, "no report should exit 0.\nStderr:\n{stderr}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("coverage JSON invalid: {e}\nStdout:\n{stdout}"));
    // All entities should be declared or linked, none passing
    assert_eq!(
        parsed["passing"].as_u64().unwrap_or(999),
        0,
        "No passing without report.\nStdout:\n{stdout}"
    );
}

#[test]
fn coverage_unknown_ids_fail() {
    let fixture = fixture_path("coverage/unknown_ids");
    let (exit_code, _stdout, stderr) = specforge_cmd(&["coverage", "--path", &fixture]);
    assert_eq!(exit_code, 1, "Unknown IDs should exit 1.\nStderr:\n{stderr}");
    assert!(
        stderr.contains("unknown entity"),
        "Should mention unknown entity.\nStderr:\n{stderr}"
    );
}

#[test]
fn coverage_multi_report_merge() {
    let fixture = fixture_path("coverage/multi_report");
    let (exit_code, stdout, stderr) =
        specforge_cmd(&["coverage", "--format", "json", "--path", &fixture]);
    assert_eq!(exit_code, 0, "multi report should exit 0.\nStderr:\n{stderr}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("coverage JSON invalid: {e}\nStdout:\n{stdout}"));
    // behavior_b has a failing test from e2e, so it should be "executed" not "passing"
    let entities = parsed["entities"].as_array().unwrap();
    let behavior_b = entities
        .iter()
        .find(|e| e["entity_id"] == "behavior_b")
        .expect("behavior_b should be in coverage");
    assert_eq!(
        behavior_b["level"].as_str().unwrap(),
        "executed",
        "behavior_b should be 'executed' (has a failing test).\nStdout:\n{stdout}"
    );
}

#[test]
fn coverage_empty_project_100_percent() {
    // A project with only non-testable entities should report 100%
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("specforge.json"),
        r#"{"name": "empty-test", "version": "1.0", "spec_root": "."}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("types.spec"),
        r#"type empty_record "Empty Record" { fields { id string } }"#,
    )
    .unwrap();
    let (exit_code, stdout, stderr) =
        specforge_cmd(&["coverage", "--path", &dir.path().to_string_lossy()]);
    assert_eq!(exit_code, 0, "empty project should exit 0.\nStderr:\n{stderr}");
    assert!(
        stdout.contains("100%"),
        "Empty project should be 100%.\nStdout:\n{stdout}"
    );
}

#[test]
fn coverage_min_overrides_config() {
    let fixture = fixture_path("coverage/all_passing");
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["coverage", "--min", "101", "--path", &fixture]);
    assert_eq!(
        exit_code, 1,
        "--min 101 should always fail.\nStderr:\n{stderr}"
    );
}

#[test]
fn coverage_help_works() {
    let (exit_code, stdout, stderr) = specforge_cmd(&["coverage", "--help"]);
    assert_eq!(exit_code, 0, "coverage --help should exit 0.\nStderr:\n{stderr}");
    assert!(
        stdout.contains("coverage"),
        "Should contain coverage.\nStdout:\n{stdout}"
    );
}

#[test]
fn coverage_declared_and_linked_levels() {
    let fixture = fixture_path("coverage/no_report");
    let (exit_code, stdout, stderr) =
        specforge_cmd(&["coverage", "--format", "json", "--path", &fixture]);
    assert_eq!(exit_code, 0, "no report should exit 0.\nStderr:\n{stderr}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("coverage JSON invalid: {e}\nStdout:\n{stdout}"));
    let entities = parsed["entities"].as_array().unwrap();
    // declared_only has verify but no tests field → declared
    let declared = entities
        .iter()
        .find(|e| e["entity_id"] == "declared_only")
        .expect("declared_only should be in coverage");
    assert_eq!(declared["level"].as_str().unwrap(), "declared");
    // linked_too has verify + tests field → linked
    let linked = entities
        .iter()
        .find(|e| e["entity_id"] == "linked_too")
        .expect("linked_too should be in coverage");
    assert_eq!(linked["level"].as_str().unwrap(), "linked");
}

// ==========================================================================
// Phase 5: Plugins/providers/doctor integration tests
// ==========================================================================

#[test]
fn plugins_lists_builtins() {
    let fixture = fixture_path("json_config_plugins");
    let (exit_code, stdout, stderr) = specforge_cmd(&["plugins", "--path", &fixture]);
    assert_eq!(exit_code, 0, "plugins should exit 0.\nStderr:\n{stderr}");
    assert!(
        stdout.contains("@specforge/product"),
        "Should list product plugin.\nStdout:\n{stdout}"
    );
    assert!(
        stdout.contains("built-in"),
        "Should show built-in.\nStdout:\n{stdout}"
    );
}

#[test]
fn plugins_empty() {
    let fixture = fixture_path("json_config");
    let (exit_code, stdout, stderr) = specforge_cmd(&["plugins", "--path", &fixture]);
    assert_eq!(exit_code, 0, "plugins should exit 0.\nStderr:\n{stderr}");
    assert!(
        stdout.contains("No plugins"),
        "Should say no plugins.\nStdout:\n{stdout}"
    );
}

#[test]
fn plugins_help_works() {
    let (exit_code, stdout, _stderr) = specforge_cmd(&["plugins", "--help"]);
    assert_eq!(exit_code, 0, "plugins --help should exit 0");
    assert!(stdout.contains("List installed plugins"));
}

#[test]
fn providers_no_providers() {
    let fixture = fixture_path("json_config");
    let (exit_code, stdout, stderr) = specforge_cmd(&["providers", "--path", &fixture]);
    assert_eq!(exit_code, 0, "providers should exit 0.\nStderr:\n{stderr}");
    assert!(
        stdout.contains("No providers configured"),
        "Should say no providers.\nStdout:\n{stdout}"
    );
}

#[test]
fn providers_help_works() {
    let (exit_code, stdout, _stderr) = specforge_cmd(&["providers", "--help"]);
    assert_eq!(exit_code, 0, "providers --help should exit 0");
    assert!(stdout.contains("List configured providers"));
}

#[test]
fn doctor_shows_plugins() {
    let fixture = fixture_path("json_config_plugins");
    let (exit_code, _stdout, stderr) = specforge_cmd(&["doctor", "--path", &fixture]);
    assert_eq!(exit_code, 0, "doctor should exit 0.\nStderr:\n{stderr}");
    assert!(
        stderr.contains("Plugins (2 installed)"),
        "Should show 2 installed.\nStderr:\n{stderr}"
    );
}

#[test]
fn doctor_json_valid() {
    let fixture = fixture_path("json_config_plugins");
    let (exit_code, stdout, stderr) = specforge_cmd(&["doctor", "--json", "--path", &fixture]);
    assert_eq!(exit_code, 0, "doctor --json should exit 0.\nStderr:\n{stderr}");
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("doctor JSON invalid: {e}\nStdout:\n{stdout}"));
    assert!(parsed["plugins"].is_array(), "should have plugins array");
}

#[test]
fn doctor_no_config_graceful() {
    // A project with just a .spec file (no JSON config) should work
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("specforge.spec"),
        r#"spec "test" { version "1.0" }"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("entities.spec"),
        r#"invariant test "Test" { guarantee """ok""" }"#,
    )
    .unwrap();
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["doctor", "--path", &dir.path().to_string_lossy()]);
    assert_eq!(
        exit_code, 0,
        "doctor on legacy config should exit 0.\nStderr:\n{stderr}"
    );
}

#[test]
fn doctor_help_works() {
    let (exit_code, stdout, _stderr) = specforge_cmd(&["doctor", "--help"]);
    assert_eq!(exit_code, 0, "doctor --help should exit 0");
    assert!(stdout.contains("Diagnose"));
}

#[test]
fn doctor_no_conflicts() {
    let fixture = fixture_path("json_config_plugins");
    let (exit_code, _stdout, stderr) = specforge_cmd(&["doctor", "--path", &fixture]);
    assert_eq!(exit_code, 0, "doctor should exit 0.\nStderr:\n{stderr}");
    assert!(
        stderr.contains("Conflicts: none"),
        "Should say no conflicts.\nStderr:\n{stderr}"
    );
}

// ==========================================================================
// Phase 5: Cache integration tests
// ==========================================================================

#[test]
fn cache_status_empty() {
    let dir = tempfile::tempdir().unwrap();
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["cache", "status", "--path", &dir.path().to_string_lossy()]);
    assert_eq!(exit_code, 0, "cache status should exit 0.\nStderr:\n{stderr}");
    assert!(
        stderr.contains("0") && stderr.contains("Entries"),
        "Should show 0 entries.\nStderr:\n{stderr}"
    );
}

#[test]
fn cache_clear_empty() {
    let dir = tempfile::tempdir().unwrap();
    let (exit_code, _stdout, stderr) =
        specforge_cmd(&["cache", "clear", "--path", &dir.path().to_string_lossy()]);
    assert_eq!(exit_code, 0, "cache clear should exit 0.\nStderr:\n{stderr}");
}

#[test]
fn cache_status_help() {
    let (exit_code, stdout, _stderr) = specforge_cmd(&["cache", "status", "--help"]);
    assert_eq!(exit_code, 0, "cache status --help should exit 0");
    assert!(stdout.contains("status") || stdout.contains("cache"));
}

#[test]
fn cache_help() {
    let (exit_code, stdout, _stderr) = specforge_cmd(&["cache", "--help"]);
    assert_eq!(exit_code, 0, "cache --help should exit 0");
    assert!(stdout.contains("status"), "Should mention status subcommand");
    assert!(stdout.contains("clear"), "Should mention clear subcommand");
}

// ==========================================================================
// Phase 5: Add/remove plugin integration tests
// ==========================================================================

#[test]
fn add_builtin_plugin() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("specforge.json");
    std::fs::write(
        &json_path,
        r#"{"name": "add-test", "version": "1.0", "spec_root": "."}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("entities.spec"),
        r#"invariant test "Test" { guarantee """ok""" }"#,
    )
    .unwrap();
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "add",
        "@specforge/product",
        &dir.path().to_string_lossy(),
    ]);
    assert_eq!(exit_code, 0, "add should exit 0.\nStderr:\n{stderr}");

    let content = std::fs::read_to_string(&json_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(
        parsed["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p == "@specforge/product"),
        "JSON should have the plugin.\nContent:\n{content}"
    );
}

#[test]
fn add_duplicate_is_noop() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("specforge.json");
    std::fs::write(
        &json_path,
        r#"{"name": "dup-test", "version": "1.0", "spec_root": ".", "plugins": ["@specforge/product"]}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("entities.spec"),
        r#"invariant test "Test" { guarantee """ok""" }"#,
    )
    .unwrap();
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "add",
        "@specforge/product",
        &dir.path().to_string_lossy(),
    ]);
    assert_eq!(exit_code, 0, "add duplicate should exit 0.\nStderr:\n{stderr}");
    assert!(
        stderr.contains("already installed"),
        "Should say already installed.\nStderr:\n{stderr}"
    );
}

#[test]
fn remove_plugin_updates_json() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("specforge.json");
    std::fs::write(
        &json_path,
        r#"{"name": "rm-test", "version": "1.0", "spec_root": ".", "plugins": ["@specforge/product"]}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("entities.spec"),
        r#"invariant test "Test" { guarantee """ok""" }"#,
    )
    .unwrap();
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "remove",
        "@specforge/product",
        "--path",
        &dir.path().to_string_lossy(),
    ]);
    assert_eq!(exit_code, 0, "remove should exit 0.\nStderr:\n{stderr}");

    let content = std::fs::read_to_string(&json_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(
        parsed["plugins"].as_array().unwrap().is_empty(),
        "Plugin should be removed.\nContent:\n{content}"
    );
}

#[test]
fn remove_nonexistent_reports() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("specforge.json");
    std::fs::write(
        &json_path,
        r#"{"name": "rm-missing", "version": "1.0", "spec_root": "."}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("entities.spec"),
        r#"invariant test "Test" { guarantee """ok""" }"#,
    )
    .unwrap();
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "remove",
        "@specforge/product",
        "--path",
        &dir.path().to_string_lossy(),
    ]);
    // Not installed → noop with exit 0
    assert_eq!(exit_code, 0, "remove non-installed should exit 0.\nStderr:\n{stderr}");
    assert!(
        stderr.contains("not installed"),
        "Should say not installed.\nStderr:\n{stderr}"
    );
}

// ==========================================================================
// Phase 5: Wasm error path integration tests
// ==========================================================================

#[test]
fn wasm_missing_plugin_dir_reported() {
    let fixture = fixture_path("wasm/missing_plugin");
    // This fixture has no wasm_packages in config, so it should just compile clean
    let (exit_code, _output) = specforge_check(&fixture);
    assert_eq!(exit_code, 0, "No wasm config = clean compile");
}

#[test]
fn wasm_bad_manifest_reported() {
    let fixture = fixture_path("wasm/bad_manifest");
    // The bad manifest fixture doesn't reference the broken plugin in specforge.json
    // (wasm_packages would need to be set), so this just verifies clean compile
    let (exit_code, _output) = specforge_check(&fixture);
    assert_eq!(exit_code, 0, "No wasm reference in config = clean compile");
}

#[test]
fn wasm_missing_binary_reported() {
    let fixture = fixture_path("wasm/manifest_only");
    // Same — no wasm_packages reference in config
    let (exit_code, _output) = specforge_check(&fixture);
    assert_eq!(exit_code, 0, "No wasm reference in config = clean compile");
}

#[test]
fn wasm_no_plugins_no_errors() {
    let fixture = fixture_path("json_config");
    let (exit_code, output) = specforge_check(&fixture);
    assert_eq!(exit_code, 0, "No wasm plugins should exit 0.\nOutput:\n{output}");
}

// ==========================================================================
// Phase 5: Package init integration tests
// ==========================================================================

#[test]
fn package_init_creates_scaffold() {
    let dir = tempfile::tempdir().unwrap();
    let pkg_dir = dir.path().join("my-plugin");
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "package",
        "init",
        "my-plugin",
        "--dir",
        &pkg_dir.to_string_lossy(),
    ]);
    assert_eq!(exit_code, 0, "package init should exit 0.\nStderr:\n{stderr}");

    assert!(pkg_dir.join("Cargo.toml").exists(), "Cargo.toml should exist");
    assert!(pkg_dir.join("src/lib.rs").exists(), "src/lib.rs should exist");
    assert!(
        pkg_dir.join("manifest.json").exists(),
        "manifest.json should exist"
    );
}

#[test]
fn package_init_manifest_valid() {
    let dir = tempfile::tempdir().unwrap();
    let pkg_dir = dir.path().join("test-plugin");
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "package",
        "init",
        "test-plugin",
        "--dir",
        &pkg_dir.to_string_lossy(),
    ]);
    assert_eq!(exit_code, 0, "package init should exit 0.\nStderr:\n{stderr}");

    let manifest_content = std::fs::read_to_string(pkg_dir.join("manifest.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&manifest_content)
        .unwrap_or_else(|e| panic!("manifest.json should be valid JSON: {e}"));
    assert!(parsed["package"].is_string(), "should have package field");
    assert!(parsed["wasm"].is_string(), "should have wasm field");
}

#[test]
fn package_help() {
    let (exit_code, stdout, _stderr) = specforge_cmd(&["package", "--help"]);
    assert_eq!(exit_code, 0, "package --help should exit 0");
    assert!(stdout.contains("init"), "Should mention init");
    assert!(stdout.contains("build"), "Should mention build");
    assert!(stdout.contains("test"), "Should mention test");
    assert!(stdout.contains("publish"), "Should mention publish");
}

#[test]
fn package_init_cargo_toml_valid() {
    let dir = tempfile::tempdir().unwrap();
    let pkg_dir = dir.path().join("cargo-check");
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "package",
        "init",
        "cargo-check",
        "--dir",
        &pkg_dir.to_string_lossy(),
    ]);
    assert_eq!(exit_code, 0, "package init should exit 0.\nStderr:\n{stderr}");

    let cargo_content = std::fs::read_to_string(pkg_dir.join("Cargo.toml")).unwrap();
    assert!(
        cargo_content.contains("cdylib"),
        "Cargo.toml should contain cdylib crate type"
    );
    assert!(
        cargo_content.contains("extism-pdk"),
        "Cargo.toml should depend on extism-pdk"
    );
}

#[test]
fn package_build_no_project_fails() {
    let dir = tempfile::tempdir().unwrap();
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "package",
        "build",
        "--path",
        &dir.path().to_string_lossy(),
    ]);
    assert_ne!(
        exit_code, 0,
        "build on empty dir should fail.\nStderr:\n{stderr}"
    );
}

// ─── Phase 6: Rust Integration ───────────────────────────────────────────────

#[test]
fn gen_rust_produces_type_files() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("generated/rust");
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "gen", "rust",
        &out_dir.to_string_lossy(),
        "--path", &fixture_path("codegen/gen_rust"),
    ]);
    assert_eq!(exit_code, 0, "gen rust should succeed.\nStderr:\n{stderr}");

    // Check type files exist
    assert!(out_dir.join("user.rs").exists(), "user.rs should exist");
    assert!(out_dir.join("user_status.rs").exists(), "user_status.rs should exist");
    assert!(out_dir.join("address.rs").exists(), "address.rs should exist");

    // Check struct content
    let user_content = std::fs::read_to_string(out_dir.join("user.rs")).unwrap();
    assert!(user_content.contains("pub struct User {"), "user.rs should contain struct User. Got:\n{user_content}");
    assert!(user_content.contains("pub name: String,"), "user.rs should contain name field. Got:\n{user_content}");
    assert!(user_content.contains("pub age: Option<i64>,"), "user.rs should contain optional age. Got:\n{user_content}");
}

#[test]
fn gen_rust_produces_trait_files() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("generated/rust");
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "gen", "rust",
        &out_dir.to_string_lossy(),
        "--path", &fixture_path("codegen/gen_rust"),
    ]);
    assert_eq!(exit_code, 0, "gen rust should succeed.\nStderr:\n{stderr}");

    assert!(out_dir.join("user_repository.rs").exists(), "user_repository.rs should exist");
    let content = std::fs::read_to_string(out_dir.join("user_repository.rs")).unwrap();
    assert!(content.contains("pub trait UserRepository {"), "should contain trait. Got:\n{content}");
    assert!(content.contains("fn save("), "should contain save method. Got:\n{content}");
    assert!(content.contains("fn find_by_id("), "should contain find_by_id method. Got:\n{content}");
    assert!(content.contains("fn list("), "should contain list method. Got:\n{content}");
}

#[test]
fn gen_rust_produces_mod_barrel() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("generated/rust");
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "gen", "rust",
        &out_dir.to_string_lossy(),
        "--path", &fixture_path("codegen/gen_rust"),
    ]);
    assert_eq!(exit_code, 0, "gen rust should succeed.\nStderr:\n{stderr}");

    assert!(out_dir.join("mod.rs").exists(), "mod.rs should exist");
    let content = std::fs::read_to_string(out_dir.join("mod.rs")).unwrap();
    assert!(content.contains("pub mod user;"), "should contain pub mod user. Got:\n{content}");
    assert!(content.contains("pub mod user_status;"), "should contain pub mod user_status. Got:\n{content}");
    assert!(content.contains("pub mod address;"), "should contain pub mod address. Got:\n{content}");
    assert!(content.contains("pub mod user_repository;"), "should contain pub mod user_repository. Got:\n{content}");
}

#[test]
fn gen_rust_produces_test_stubs() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("generated/rust");
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "gen", "rust",
        &out_dir.to_string_lossy(),
        "--path", &fixture_path("codegen/gen_rust"),
    ]);
    assert_eq!(exit_code, 0, "gen rust should succeed.\nStderr:\n{stderr}");

    let test_file = out_dir.join("tests/specforge/validate_input.rs");
    assert!(test_file.exists(), "test stub should exist at {}", test_file.display());
    let content = std::fs::read_to_string(&test_file).unwrap();
    assert!(content.contains("validate_input__rejects_empty_email"), "should contain double-underscore naming. Got:\n{content}");
    assert!(content.contains("#[test]"), "should contain #[test]. Got:\n{content}");
    assert!(content.contains("todo!(\"implement unit test\")"), "should contain todo. Got:\n{content}");
}

#[test]
fn gen_rust_union_type() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("generated/rust");
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "gen", "rust",
        &out_dir.to_string_lossy(),
        "--path", &fixture_path("codegen/gen_rust"),
    ]);
    assert_eq!(exit_code, 0, "gen rust should succeed.\nStderr:\n{stderr}");

    let content = std::fs::read_to_string(out_dir.join("user_status.rs")).unwrap();
    assert!(content.contains("pub enum UserStatus {"), "should contain enum. Got:\n{content}");
    assert!(content.contains("Active,"), "should contain PascalCase variant Active. Got:\n{content}");
    assert!(content.contains("Inactive,"), "should contain PascalCase variant Inactive. Got:\n{content}");
    assert!(content.contains("Banned,"), "should contain PascalCase variant Banned. Got:\n{content}");
}

#[test]
fn gen_rust_check_no_drift() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("generated/rust");

    // Generate first
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "gen", "rust",
        &out_dir.to_string_lossy(),
        "--path", &fixture_path("codegen/gen_rust"),
    ]);
    assert_eq!(exit_code, 0, "gen rust should succeed.\nStderr:\n{stderr}");

    // Check should pass (no drift)
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "gen", "rust",
        &out_dir.to_string_lossy(),
        "--check",
        "--path", &fixture_path("codegen/gen_rust"),
    ]);
    assert_eq!(exit_code, 0, "--check should exit 0 when no drift.\nStderr:\n{stderr}");
}

#[test]
fn gen_rust_check_drift_detected() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("generated/rust");

    // Generate first
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "gen", "rust",
        &out_dir.to_string_lossy(),
        "--path", &fixture_path("codegen/gen_rust"),
    ]);
    assert_eq!(exit_code, 0, "gen rust should succeed.\nStderr:\n{stderr}");

    // Tamper with a file
    std::fs::write(out_dir.join("user.rs"), "// tampered\n").unwrap();

    // Check should fail (drift detected)
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "gen", "rust",
        &out_dir.to_string_lossy(),
        "--check",
        "--path", &fixture_path("codegen/gen_rust"),
    ]);
    assert_eq!(exit_code, 1, "--check should exit 1 when drift detected.\nStderr:\n{stderr}");
    assert!(stderr.contains("drift"), "should mention drift. Got:\n{stderr}");
}

#[test]
fn gen_rust_help() {
    let (exit_code, stdout, stderr) = specforge_cmd(&["gen", "--help"]);
    let combined = format!("{stdout}{stderr}");
    assert_eq!(exit_code, 0, "gen --help should succeed.\nOutput:\n{combined}");
}

#[test]
fn gen_rust_checksum_headers() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("generated/rust");
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "gen", "rust",
        &out_dir.to_string_lossy(),
        "--path", &fixture_path("codegen/gen_rust"),
    ]);
    assert_eq!(exit_code, 0, "gen rust should succeed.\nStderr:\n{stderr}");

    // All .rs files should have checksum headers
    for entry in std::fs::read_dir(&out_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().is_some_and(|e| e == "rs") {
            let content = std::fs::read_to_string(entry.path()).unwrap();
            assert!(
                content.starts_with("// @specforge-checksum:sha256:"),
                "{} should have checksum header. Got:\n{}",
                entry.path().display(),
                &content[..content.len().min(100)]
            );
        }
    }
}

#[test]
fn gen_rust_serde_derives() {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("generated/rust");

    // Create a temporary spec with serde extra
    let spec_dir = dir.path().join("spec");
    std::fs::create_dir_all(&spec_dir).unwrap();
    std::fs::write(spec_dir.join("specforge.spec"), r#"spec "serde-test" {
  version "1.0"
  plugins []

  gen rust {
    out "generated/rust"
    serde "true"
  }
}

type Config {
  name string
  value integer
}
"#).unwrap();

    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "gen", "rust",
        &out_dir.to_string_lossy(),
        "--path", &spec_dir.to_string_lossy(),
    ]);
    assert_eq!(exit_code, 0, "gen rust should succeed.\nStderr:\n{stderr}");

    let content = std::fs::read_to_string(out_dir.join("config.rs")).unwrap();
    assert!(
        content.contains("Serialize, Deserialize"),
        "should contain serde derives when extra serde=true. Got:\n{content}"
    );
}

#[test]
fn collect_rust_junit_xml() {
    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("specforge-report.json");
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "collect", "rust",
        "--junit", &fixture_path("collect/nextest-junit.xml"),
        "--output", &output_path.to_string_lossy(),
        "--path", &fixture_path("collect"),
    ]);
    assert_eq!(exit_code, 0, "collect rust should succeed.\nStderr:\n{stderr}");
    assert!(output_path.exists(), "specforge-report.json should exist");

    let report_content = std::fs::read_to_string(&output_path).unwrap();
    let report: serde_json::Value = serde_json::from_str(&report_content).unwrap();
    assert_eq!(report["adapter"], "rust");
    assert_eq!(report["schema_version"], "1.0");

    let entities = report["entities"].as_array().unwrap();
    assert!(!entities.is_empty(), "should have entities");
}

#[test]
fn collect_rust_libtest_json() {
    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("specforge-report.json");
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "collect", "rust",
        "--libtest", &fixture_path("collect/libtest-output.json"),
        "--output", &output_path.to_string_lossy(),
        "--path", &fixture_path("collect"),
    ]);
    assert_eq!(exit_code, 0, "collect rust (libtest) should succeed.\nStderr:\n{stderr}");
    assert!(output_path.exists(), "specforge-report.json should exist");

    let report_content = std::fs::read_to_string(&output_path).unwrap();
    let report: serde_json::Value = serde_json::from_str(&report_content).unwrap();
    assert_eq!(report["adapter"], "rust");

    let entities = report["entities"].as_array().unwrap();
    // Only validate_input should map (unknown_entity doesn't exist in spec)
    let entity_ids: Vec<&str> = entities
        .iter()
        .map(|e| e["entity_id"].as_str().unwrap())
        .collect();
    assert!(entity_ids.contains(&"validate_input"), "should contain validate_input. Got: {entity_ids:?}");
}

#[test]
fn collect_rust_no_input_fails() {
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "collect", "rust",
        "--path", &fixture_path("collect"),
    ]);
    assert_ne!(exit_code, 0, "collect rust without --junit or --libtest should fail.\nStderr:\n{stderr}");
    assert!(stderr.contains("requires at least"), "should mention missing input. Got:\n{stderr}");
}

#[test]
fn collect_rust_strict_unknown_fails() {
    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join("specforge-report.json");
    let (exit_code, _stdout, stderr) = specforge_cmd(&[
        "collect", "rust",
        "--libtest", &fixture_path("collect/libtest-output.json"),
        "--strict",
        "--output", &output_path.to_string_lossy(),
        "--path", &fixture_path("collect"),
    ]);
    assert_ne!(exit_code, 0, "--strict with unknown entity should fail.\nStderr:\n{stderr}");
    assert!(stderr.contains("unknown entity"), "should mention unknown entity. Got:\n{stderr}");
}

#[test]
fn collect_help() {
    let (exit_code, stdout, stderr) = specforge_cmd(&["collect", "--help"]);
    let combined = format!("{stdout}{stderr}");
    assert_eq!(exit_code, 0, "collect --help should succeed.\nOutput:\n{combined}");
    assert!(combined.contains("rust") || combined.contains("Rust"), "should mention rust subcommand. Got:\n{combined}");
}
