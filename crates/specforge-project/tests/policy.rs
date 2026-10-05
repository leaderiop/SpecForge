use specforge_common::{Diagnostic, Severity};
use specforge_project::{DiagnosticPolicy, LINT_PROFILE_NAMES, LintProfile, UnknownLintProfile};
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
        lint_profiles: vec![LintProfile::Inferred],
    };
    assert!(policy.apply(root.path(), Vec::new()).is_empty());
}

/// The lint profiles are a closed set: each name parses to its profile,
/// any other is refused with the name it was given, and `pedantic`, the
/// explicit name of the default, adds nothing.
#[specforge_test(
    behavior = "check_diagnostic_policy",
    verify = "an unknown lint profile is refused by name, and pedantic adds nothing"
)]
fn lint_profiles_are_a_closed_set() {
    for name in LINT_PROFILE_NAMES {
        let profile: LintProfile = name.parse().unwrap();
        assert_eq!(profile.name(), *name);
    }
    for unknown in ["nonsense", "Inferred", "pedantic,inferred", ""] {
        let refused = unknown.parse::<LintProfile>().unwrap_err();
        assert_eq!(
            refused,
            UnknownLintProfile {
                requested: unknown.to_string()
            }
        );
        assert_eq!(
            refused.to_string(),
            format!("Unknown lint profile '{unknown}' (available: inferred, pedantic)")
        );
    }

    let root = tempfile::TempDir::new().unwrap();
    let reported = vec![
        warning(),
        Diagnostic::info("I067", "module 'm' contains no features"),
    ];
    let pedantic = DiagnosticPolicy {
        strict: false,
        lint_profiles: vec![LintProfile::Pedantic],
    };
    assert_eq!(
        pedantic.apply(root.path(), reported.clone()),
        DiagnosticPolicy::default().apply(root.path(), reported)
    );
}
