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
