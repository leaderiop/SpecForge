use specforge_common::{Diagnostic, Severity};
use specforge_project::DiagnosticPolicy;
use specforge_test::prelude::*;

fn warning() -> Diagnostic {
    Diagnostic::warning("W002", "unused entity")
}

/// Strict is one rule for every surface: the policy promotes warnings to
/// errors, and the exit code counts errors.
#[specforge_test(
    behavior = "exit_code_reflects_diagnostic_severity",
    verify = "exit 1 with warnings in strict mode"
)]
fn strict_promotes_warnings_so_the_exit_code_is_one() {
    let root = tempfile::TempDir::new().unwrap();
    let lenient = DiagnosticPolicy::strict(false).apply(root.path(), vec![warning()]);
    assert_eq!(lenient[0].severity, Severity::Warning);
    assert_eq!(specforge_common::compute_exit_code(&lenient), 0);

    let strict = DiagnosticPolicy::strict(true).apply(root.path(), vec![warning()]);
    assert_eq!(strict[0].severity, Severity::Error);
    assert_eq!(specforge_common::compute_exit_code(&strict), 1);

    // Nothing to promote: still 0.
    let clean = DiagnosticPolicy::strict(true).apply(root.path(), Vec::new());
    assert_eq!(specforge_common::compute_exit_code(&clean), 0);
}

/// A lint profile adds nothing when its input is absent: no
/// specforge-infer.json, no I200/I202.
#[test]
fn the_inferred_profile_needs_an_inference_manifest() {
    let root = tempfile::TempDir::new().unwrap();
    let policy = DiagnosticPolicy {
        strict: false,
        lint_profiles: vec!["inferred".to_string()],
    };
    assert!(policy.apply(root.path(), Vec::new()).is_empty());
}
