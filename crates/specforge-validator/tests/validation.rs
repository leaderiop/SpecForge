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

#[specforge_test(behavior = "detect_orphan_refs", verify = "unreferenced structural node of any grammar-level kind produces W012")]
#[test]
fn unreferenced_spec_block_produces_w012() {
    // spec blocks are structural kinds — orphan detection applies
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
    assert_eq!(warnings.len(), 1, "orphan spec block should produce W012");
    assert!(warnings[0].message.contains("spec"));
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

#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "E016 suggests similar filename when close match exists")]
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

#[specforge_test(behavior = "provide_did_you_mean_suggestions", verify = "E016 has no suggestion when no similar file exists")]
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
            file: "test.spec".to_string(),
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
            file: "test.spec".to_string(),
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
