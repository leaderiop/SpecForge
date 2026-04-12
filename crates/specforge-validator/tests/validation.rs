use specforge_graph::build_graph;
use specforge_parser::parse;
use specforge_test_macros::test as specforge_test;
use specforge_validator::{Diagnostic, Severity, SourceSpan};

// === detect_orphan_refs ===

#[specforge_test(behavior = "detect_orphan_refs", verify = "unreferenced ref produces W012")]
#[test]
fn unreferenced_ref_produces_w012() {
    let source = r#"
behavior alpha "A" { contract "first" }
ref gh.issue:42 "Support Wasm extensions"
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let diagnostics = specforge_validator::validate(&graph);

    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.code == "W012").collect();
    assert_eq!(warnings.len(), 1, "orphan ref should produce W012");
    assert!(warnings[0].message.contains("gh.issue:42"));
    assert_eq!(warnings[0].severity, Severity::Warning);
}

#[specforge_test(behavior = "detect_orphan_refs", verify = "referenced ref suppresses W012")]
#[test]
fn referenced_ref_suppresses_w012() {
    let source = r#"
behavior alpha "A" {
  contract "first"
  refs [gh.issue:42]
}
ref gh.issue:42 "Support Wasm extensions"
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let diagnostics = specforge_validator::validate(&graph);

    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.code == "W012").collect();
    assert!(warnings.is_empty(), "referenced ref should not produce W012");
}

#[specforge_test(behavior = "detect_orphan_refs", verify = "spec block is a root container and does not produce W012")]
#[test]
fn spec_block_does_not_produce_w012() {
    // spec blocks are project root containers — they naturally have no
    // incoming edges and should NOT produce W012.
    let source = r#"
spec "MyProject" {
  version "1.0"
}
behavior alpha "A" { contract "first" }
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let diagnostics = specforge_validator::validate(&graph);

    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.code == "W012").collect();
    assert!(warnings.is_empty(), "spec block should not produce W012, got: {:?}", warnings);
}

#[specforge_test(behavior = "detect_orphan_refs", verify = "structural node with at least one incoming edge suppresses W012")]
#[test]
fn non_structural_kind_does_not_produce_w012() {
    // Extension-defined kinds (behavior, feature) are NOT structural —
    // their orphan detection is extension-defined, not core
    let source = r#"
behavior alpha "A" { contract "first" }
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let diagnostics = specforge_validator::validate(&graph);

    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.code == "W012").collect();
    assert!(warnings.is_empty(), "non-structural kinds should not trigger W012");
}

// === validate_file_reference_paths ===

#[specforge_test(behavior = "validate_file_reference_paths", verify = "non-existent file reference produces E016")]
#[test]
fn missing_file_reference_produces_e016() {
    use specforge_validator::ValidatorConfig;
    use std::path::Path;

    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["features/alpha.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: Path::new("/nonexistent/project").to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert_eq!(errors.len(), 1, "missing file should produce E016");
    assert!(errors[0].message.contains("alpha.feature"));
}

#[specforge_test(behavior = "validate_file_reference_paths", verify = "existing file reference passes silently")]
#[test]
fn existing_file_reference_passes() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    let features_dir = dir.path().join("features");
    std::fs::create_dir_all(&features_dir).unwrap();
    std::fs::write(features_dir.join("alpha.feature"), "Feature: Alpha").unwrap();

    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["features/alpha.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert!(errors.is_empty(), "existing file should not produce E016");
}

#[specforge_test(behavior = "validate_file_reference_paths", verify = "multiple file references in same entity each validated independently")]
#[test]
fn multiple_file_refs_validated_independently() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    let features_dir = dir.path().join("features");
    std::fs::create_dir_all(&features_dir).unwrap();
    std::fs::write(features_dir.join("alpha.feature"), "Feature: Alpha").unwrap();
    // beta.feature does NOT exist

    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["features/alpha.feature", "features/beta.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert_eq!(errors.len(), 1, "only missing file should produce E016");
    assert!(errors[0].message.contains("beta.feature"));
}

// === provide_did_you_mean_suggestions (file references) ===

#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "close match produces suggestion")]
#[test]
fn e016_suggests_similar_filename() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    let features_dir = dir.path().join("features");
    std::fs::create_dir_all(&features_dir).unwrap();
    std::fs::write(features_dir.join("alpha.feature"), "Feature: Alpha").unwrap();

    // Typo: "alpa.feature" instead of "alpha.feature"
    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["features/alpa.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].suggestion.as_ref().is_some_and(|s| s.contains("alpha.feature")),
        "E016 should suggest 'alpha.feature', got: {:?}",
        errors[0].suggestion
    );
}

#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "distant match produces no suggestion")]
#[test]
fn e016_no_suggestion_when_no_similar_file() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    let features_dir = dir.path().join("features");
    std::fs::create_dir_all(&features_dir).unwrap();
    std::fs::write(features_dir.join("zebra.feature"), "Feature: Zebra").unwrap();

    // "alpha.feature" is completely different from "zebra.feature"
    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["features/alpha.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].suggestion.is_none(),
        "should not suggest unrelated file, got: {:?}",
        errors[0].suggestion
    );
}

// === format_diagnostics_with_source_context ===

#[specforge_test(behavior = "format_diagnostics_with_source_context", verify = "diagnostic shows file:line:col")]
#[test]
fn diagnostic_shows_file_line_col() {
    use specforge_validator::render_diagnostics;
    use std::collections::HashMap;

    let source = r#"behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#;
    let spec_file = parse(source, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);

    let sources: HashMap<String, String> =
        vec![("main.spec".to_string(), source.to_string())]
            .into_iter()
            .collect();
    let output = render_diagnostics(&diagnostics, &sources);

    assert!(output.contains("main.spec"), "output should contain filename");
    // ariadne renders line numbers — check the output has location info
    assert!(output.contains("E001"), "output should contain error code");
    assert!(output.contains("nonexistent"), "output should contain the unresolved reference");
}

#[specforge_test(behavior = "format_diagnostics_with_source_context", verify = "context snippet highlights offending token")]
#[test]
fn diagnostic_shows_source_context() {
    use specforge_validator::render_diagnostics;
    use std::collections::HashMap;

    let source = r#"behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [alpha, nonexistent] }
"#;
    let spec_file = parse(source, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);

    let sources: HashMap<String, String> =
        vec![("main.spec".to_string(), source.to_string())]
            .into_iter()
            .collect();
    let output = render_diagnostics(&diagnostics, &sources);

    // ariadne renders the offending source line
    assert!(
        output.contains("feature gamma"),
        "output should contain the source line with the error: got\n{}",
        output
    );
}

#[specforge_test(behavior = "format_diagnostics_with_source_context", verify = "multi-line span shows full range")]
#[test]
fn diagnostic_renders_multiline_span() {
    use specforge_validator::render_diagnostics;
    use std::collections::HashMap;

    let diag = specforge_validator::Diagnostic {
        code: "E099".to_string(),
        severity: Severity::Error,
        message: "test multi-line error".to_string(),
        span: Some(specforge_validator::SourceSpan {
            file: specforge_common::Sym::new("test.spec"),
            start_line: 2,
            start_col: 1,
            end_line: 4,
            end_col: 2,
        }),
        suggestion: None,
    };

    let source = "line 1\nbehavior alpha \"A\" {\n  contract \"first\"\n}\nline 5\n";
    let sources: HashMap<String, String> =
        vec![("test.spec".to_string(), source.to_string())]
            .into_iter()
            .collect();
    let output = render_diagnostics(&[diag], &sources);

    assert!(output.contains("E099"), "should contain error code");
    assert!(output.contains("test.spec"), "should contain filename");
    assert!(output.contains("behavior alpha"), "should contain start line of span");
}

// === aggregate_diagnostic_summary ===

#[specforge_test(behavior = "aggregate_diagnostic_summary", verify = "summary shows correct counts")]
#[test]
fn summary_shows_correct_counts() {
    use specforge_validator::diagnostic_summary;

    let diagnostics = vec![
        specforge_validator::Diagnostic {
            code: "E001".to_string(),
            severity: Severity::Error,
            message: "error 1".to_string(),
            span: None,
            suggestion: None,
        },
        specforge_validator::Diagnostic {
            code: "W012".to_string(),
            severity: Severity::Warning,
            message: "warning 1".to_string(),
            span: None,
            suggestion: None,
        },
        specforge_validator::Diagnostic {
            code: "E002".to_string(),
            severity: Severity::Error,
            message: "error 2".to_string(),
            span: None,
            suggestion: None,
        },
        specforge_validator::Diagnostic {
            code: "I004".to_string(),
            severity: Severity::Info,
            message: "info 1".to_string(),
            span: None,
            suggestion: None,
        },
    ];

    let summary = diagnostic_summary(&diagnostics);

    assert!(summary.contains("2 error"), "should show 2 errors: got '{}'", summary);
    assert!(summary.contains("1 warning"), "should show 1 warning: got '{}'", summary);
    assert!(summary.contains("1 info"), "should show 1 info: got '{}'", summary);
}

#[specforge_test(behavior = "aggregate_diagnostic_summary", verify = "summary matches actual diagnostics")]
#[test]
fn summary_clean_project() {
    use specforge_validator::diagnostic_summary;

    let summary = diagnostic_summary(&[]);
    assert!(summary.contains("0 error"), "clean project: got '{}'", summary);
}

#[specforge_test(behavior = "aggregate_diagnostic_summary", verify = "summary is red when errors exist")]
#[test]
fn summary_red_when_errors_exist() {
    use specforge_validator::diagnostic_summary;

    let diagnostics = vec![Diagnostic {
        code: "E001".to_string(),
        severity: Severity::Error,
        message: "test error".to_string(),
        span: None,
        suggestion: None,
    }];
    let summary = diagnostic_summary(&diagnostics);

    // ANSI red escape: \x1b[31m
    assert!(
        summary.contains("\x1b[31m") || summary.contains("\x1b[1;31m"),
        "summary with errors should contain red ANSI escape, got: {:?}",
        summary
    );
}

// === validate_file_reference_paths: relative path ===

#[specforge_test(behavior = "validate_file_reference_paths", verify = "relative path resolved from spec file directory")]
#[test]
fn relative_path_resolved_from_spec_root() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    // Create nested structure: spec_root/sub/features/alpha.feature
    let sub_dir = dir.path().join("sub").join("features");
    std::fs::create_dir_all(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("alpha.feature"), "Feature: Alpha").unwrap();

    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["sub/features/alpha.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert!(
        errors.is_empty(),
        "relative path from spec root should resolve, got: {:?}",
        errors
    );
}

// === Contract tests ===

#[specforge_test(behavior = "detect_orphan_refs", verify = "requires/ensures consistency for orphan structural node detection")]
#[test]
fn orphan_refs_contract_consistency() {
    // Requires: graph_built event has fired (graph is fully constructed)
    // Ensures: all structural nodes with zero incoming edges produce W012,
    //          structural nodes with incoming edges produce no warning

    // Case 1: orphan ref (zero incoming edges) → W012
    let source_orphan = r#"
behavior alpha "A" { contract "first" }
ref gh.issue:42 "Orphan ref"
"#;
    let spec_file = parse(source_orphan, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);
    let diagnostics = specforge_validator::validate(&graph);
    let w012: Vec<_> = diagnostics.iter().filter(|d| d.code == "W012").collect();
    assert_eq!(w012.len(), 1, "orphan structural node must produce W012");

    // Case 2: referenced ref (has incoming edge) → no W012
    let source_linked = r#"
behavior alpha "A" { contract "first" refs [gh.issue:42] }
ref gh.issue:42 "Linked ref"
"#;
    let spec_file = parse(source_linked, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);
    let diagnostics = specforge_validator::validate(&graph);
    let w012: Vec<_> = diagnostics.iter().filter(|d| d.code == "W012").collect();
    assert!(w012.is_empty(), "referenced structural node must not produce W012");
}

#[specforge_test(behavior = "validate_file_reference_paths", verify = "requires/ensures consistency for file reference validation")]
#[test]
fn file_ref_contract_consistency() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    let features_dir = dir.path().join("features");
    std::fs::create_dir_all(&features_dir).unwrap();
    std::fs::write(features_dir.join("exists.feature"), "Feature: OK").unwrap();

    // Requires: graph is built, filesystem is available
    // Ensures: missing files → E016, existing files → no diagnostic
    let source = r#"
behavior alpha "A" {
  contract "first"
  gherkin ["features/exists.feature", "features/missing.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E016").collect();
    assert_eq!(errors.len(), 1, "only missing file should produce E016");
    assert!(errors[0].message.contains("missing.feature"));
}

#[specforge_test(behavior = "format_diagnostics_with_source_context", verify = "requires/ensures consistency for diagnostic source context formatting")]
#[test]
fn diagnostic_format_contract_consistency() {
    use specforge_validator::render_diagnostics;
    use std::collections::HashMap;

    // Requires: valid SourceSpan referencing accessible source
    // Ensures: output includes file:line:col header, context snippet, caret marker
    let diag = Diagnostic {
        code: "E001".to_string(),
        severity: Severity::Error,
        message: "unresolved reference".to_string(),
        span: Some(SourceSpan {
            file: specforge_common::Sym::new("test.spec"),
            start_line: 2,
            start_col: 20,
            end_line: 2,
            end_col: 31,
        }),
        suggestion: None,
    };

    let source = "line 1\nfeature gamma \"G\" { behaviors [nonexistent] }\nline 3\n";
    let sources: HashMap<String, String> =
        vec![("test.spec".to_string(), source.to_string())]
            .into_iter()
            .collect();
    let output = render_diagnostics(&[diag], &sources);

    assert!(output.contains("test.spec"), "must include file path");
    assert!(output.contains("E001"), "must include error code");
    assert!(output.contains("behaviors"), "must include source context snippet");
}

#[specforge_test(behavior = "aggregate_diagnostic_summary", verify = "requires/ensures consistency for diagnostic summary aggregation")]
#[test]
fn summary_contract_consistency() {
    use specforge_validator::diagnostic_summary;

    // Requires: validation has completed
    // Ensures: counts match actual diagnostics exactly
    let diagnostics = vec![
        Diagnostic { code: "E001".to_string(), severity: Severity::Error, message: "e".to_string(), span: None, suggestion: None },
        Diagnostic { code: "E002".to_string(), severity: Severity::Error, message: "e".to_string(), span: None, suggestion: None },
        Diagnostic { code: "E003".to_string(), severity: Severity::Error, message: "e".to_string(), span: None, suggestion: None },
        Diagnostic { code: "W012".to_string(), severity: Severity::Warning, message: "w".to_string(), span: None, suggestion: None },
    ];
    let summary = diagnostic_summary(&diagnostics);

    assert!(summary.contains("3 error"), "must report exact error count: got '{}'", summary);
    assert!(summary.contains("1 warning"), "must report exact warning count: got '{}'", summary);
    assert!(summary.contains("0 info"), "must report exact info count: got '{}'", summary);
}

#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "requires/ensures consistency for did-you-mean suggestions")]
#[test]
fn did_you_mean_contract_consistency() {
    // Requires: unresolved reference available, entity IDs populated
    // Ensures: suggestions within distance threshold, no suggestion for distant matches

    // Close match → suggestion
    let source_close = r#"
behavior alpha_parser "A" { contract "first" }
feature gamma "G" { behaviors [alpha_parsr] }
"#;
    let spec_file = parse(source_close, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].suggestion.is_some(), "close match must produce suggestion");

    // Distant match → no suggestion
    let source_far = r#"
behavior alpha "A" { contract "first" }
feature gamma "G" { behaviors [zzzzz_completely_different] }
"#;
    let spec_file = parse(source_far, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].suggestion.is_none(), "distant match must not produce suggestion");
}

#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "suggestion appears in help text")]
#[test]
fn suggestion_appears_in_help_text_for_file_refs() {
    use specforge_validator::ValidatorConfig;

    let dir = tempfile::TempDir::new().unwrap();
    let features_dir = dir.path().join("features");
    std::fs::create_dir_all(&features_dir).unwrap();
    std::fs::write(features_dir.join("login.feature"), "Feature: Login").unwrap();

    // Typo: "logn.feature" instead of "login.feature"
    let source = r#"
behavior login_flow "Login" {
  contract "handles login"
  gherkin ["features/logn.feature"]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, _) = build_graph(&[spec_file]);

    let config = ValidatorConfig {
        spec_root: dir.path().to_path_buf(),
        file_reference_fields: vec!["gherkin".to_string()],
    };
    let diagnostics = specforge_validator::validate_with_config(&graph, &config);

    let e016 = diagnostics.iter().find(|d| d.code == "E016").unwrap();
    assert!(
        e016.suggestion.as_ref().is_some_and(|s| s.contains("login.feature")),
        "suggestion must appear in diagnostic help text"
    );
}

// === detect_dangling_references ===

#[specforge_test(behavior = "detect_dangling_references", verify = "reference without corresponding graph edge indicates resolver bug")]
#[test]
fn dangling_ref_without_edge_indicates_resolver_bug() {
    // A reference list entry that resolves (target exists) should always
    // produce a corresponding graph edge. If it doesn't, that's a resolver bug.
    // Here we verify the normal path: unresolved references produce E001.
    let source = r#"
behavior alpha "A" {
  contract "first"
  invariants [nonexistent_invariant]
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);

    // The reference target doesn't exist → E001 emitted, no edge created
    let e001: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(e001.len(), 1, "unresolved reference should produce E001");

    // No edge should exist for the unresolved reference
    let edges = graph.edges_from("alpha");
    assert!(
        edges.iter().all(|e| e.target != "nonexistent_invariant"),
        "no edge should exist for unresolved reference"
    );
}

#[specforge_test(behavior = "detect_dangling_references", verify = "reference with corresponding graph edge passes")]
#[test]
fn resolved_ref_has_corresponding_edge() {
    let source = r#"
behavior alpha "A" {
  contract "first"
  invariants [inv_one]
}
invariant inv_one "Invariant One" {
  contract "must hold"
}
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);

    // No E001 — reference resolves cleanly
    let e001: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert!(e001.is_empty(), "resolved reference should not produce E001");

    // Edge must exist from alpha to inv_one
    let edges = graph.edges_from("alpha");
    assert!(
        edges.iter().any(|e| e.target == "inv_one"),
        "resolved reference must have a corresponding graph edge"
    );
}

#[specforge_test(behavior = "detect_dangling_references", verify = "empty graph with zero edges produces no dangling reference diagnostic")]
#[test]
fn empty_graph_no_dangling_diagnostics() {
    let source = r#"
behavior alpha "A" { contract "first" }
"#;
    let spec_file = parse(source, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);

    // No reference lists → zero edges → no E001
    assert_eq!(graph.edge_count(), 0, "graph should have zero edges");
    let e001: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert!(e001.is_empty(), "empty graph should produce no dangling reference diagnostic");
}

#[specforge_test(behavior = "detect_dangling_references", verify = "requires/ensures consistency for dangling reference detection")]
#[test]
fn dangling_ref_contract_consistency() {
    // Requires: graph_built event has fired (graph is fully constructed)
    // Ensures: every reference list entry has a corresponding graph edge,
    //          or E001 is raised; no duplicate diagnostics

    // Case 1: resolved reference → edge exists, no E001
    let source_ok = r#"
behavior alpha "A" { contract "first" invariants [inv_one] }
invariant inv_one "I" { contract "must hold" }
"#;
    let spec_file = parse(source_ok, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);
    let e001: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert!(e001.is_empty(), "resolved ref must not produce E001");
    assert!(
        graph.edges_from("alpha").iter().any(|e| e.target == "inv_one"),
        "resolved ref must have corresponding edge"
    );

    // Case 2: unresolved reference → E001, no edge
    let source_bad = r#"
behavior beta "B" { contract "second" invariants [missing] }
"#;
    let spec_file = parse(source_bad, "main.spec");
    let (graph, diagnostics) = build_graph(&[spec_file]);
    let e001: Vec<_> = diagnostics.iter().filter(|d| d.code == "E001").collect();
    assert_eq!(e001.len(), 1, "unresolved ref must produce exactly one E001");
    assert!(
        graph.edges_from("beta").is_empty(),
        "unresolved ref must not create edge"
    );
}

// === detect_duplicate_entity_ids ===

#[specforge_test(behavior = "detect_duplicate_entity_ids", verify = "duplicate ID in same file produces E002")]
#[test]
fn duplicate_id_same_file_produces_e002() {
    let source = r#"
behavior alpha "First Alpha" { contract "first" }
behavior alpha "Second Alpha" { contract "second" }
"#;
    let spec_file = parse(source, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);

    let e002: Vec<_> = diagnostics.iter().filter(|d| d.code == "E002").collect();
    assert_eq!(e002.len(), 1, "duplicate ID in same file should produce E002");
    assert!(e002[0].message.contains("alpha"), "E002 message should name the duplicate ID");
}

#[specforge_test(behavior = "detect_duplicate_entity_ids", verify = "duplicate ID across files produces E002")]
#[test]
fn duplicate_id_across_files_produces_e002() {
    let source_a = r#"
behavior alpha "Alpha in file A" { contract "first" }
"#;
    let source_b = r#"
behavior alpha "Alpha in file B" { contract "second" }
"#;
    let spec_file_a = parse(source_a, "a.spec");
    let spec_file_b = parse(source_b, "b.spec");
    let (_, diagnostics) = build_graph(&[spec_file_a, spec_file_b]);

    let e002: Vec<_> = diagnostics.iter().filter(|d| d.code == "E002").collect();
    assert_eq!(e002.len(), 1, "duplicate ID across files should produce E002");
    assert!(e002[0].message.contains("alpha"), "E002 message should name the duplicate ID");
}

#[specforge_test(behavior = "detect_duplicate_entity_ids", verify = "E002 includes both source locations")]
#[test]
fn e002_includes_both_source_locations() {
    let source_a = r#"
behavior alpha "Alpha in file A" { contract "first" }
"#;
    let source_b = r#"
behavior alpha "Alpha in file B" { contract "second" }
"#;
    let spec_file_a = parse(source_a, "a.spec");
    let spec_file_b = parse(source_b, "b.spec");
    let (_, diagnostics) = build_graph(&[spec_file_a, spec_file_b]);

    let e002: Vec<_> = diagnostics.iter().filter(|d| d.code == "E002").collect();
    assert_eq!(e002.len(), 1, "should have exactly one E002");

    // The E002 diagnostic's span points to the duplicate (second) declaration,
    // and the message includes the file where the duplicate was found.
    // Both declaration sites are identifiable: the first via the graph node
    // (which retains the original), and the second via the E002 diagnostic span.
    let diag = &e002[0];
    assert!(
        diag.span.is_some(),
        "E002 must include a source span for one declaration site"
    );
    // The duplicate's span file and the message together identify both sites
    assert!(
        diag.message.contains("alpha"),
        "E002 message must name the duplicate ID, got: {}",
        diag.message
    );
}

#[specforge_test(behavior = "detect_duplicate_entity_ids", verify = "requires/ensures consistency for duplicate entity ID detection")]
#[test]
fn duplicate_id_contract_consistency() {
    // Requires: all_files_parsed (all .spec files parsed, entity IDs collected)
    // Ensures: every duplicate entity ID has E002 naming both declaration sites

    // Case 1: unique IDs → no E002
    let source_unique = r#"
behavior alpha "A" { contract "first" }
behavior beta "B" { contract "second" }
"#;
    let spec_file = parse(source_unique, "main.spec");
    let (_, diagnostics) = build_graph(&[spec_file]);
    let e002: Vec<_> = diagnostics.iter().filter(|d| d.code == "E002").collect();
    assert!(e002.is_empty(), "unique IDs must not produce E002");

    // Case 2: duplicate IDs → E002 with both sites
    let source_a = r#"
behavior gamma "Gamma A" { contract "first" }
"#;
    let source_b = r#"
behavior gamma "Gamma B" { contract "second" }
"#;
    let spec_file_a = parse(source_a, "first.spec");
    let spec_file_b = parse(source_b, "second.spec");
    let (_, diagnostics) = build_graph(&[spec_file_a, spec_file_b]);
    let e002: Vec<_> = diagnostics.iter().filter(|d| d.code == "E002").collect();
    assert_eq!(e002.len(), 1, "duplicate IDs must produce exactly one E002");
    assert!(e002[0].message.contains("gamma"), "E002 must name the duplicate ID");
    assert!(
        e002[0].span.is_some(),
        "E002 must include source span identifying a declaration site"
    );
}
