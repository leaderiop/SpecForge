use specforge_common::{Diagnostic, Severity};
use specforge_project::{
    DiagnosticPolicy, LINT_PROFILE_NAMES, LintProfile, Raises, UnknownLintProfile,
};
use specforge_test::prelude::*;

fn warning() -> Diagnostic {
    Diagnostic::untyped("W002", Severity::Warning, "unused entity")
}

/// A lint no profile answers: what a check with nothing to add gives.
fn no_lint(_: LintProfile) -> Vec<Diagnostic> {
    Vec::new()
}

/// Strict is one rule for every surface: the policy promotes warnings to
/// errors, and the exit code counts errors.
#[specforge_test(
    behavior = "exit_code_reflects_diagnostic_severity",
    verify = "exit 1 with warnings in strict mode"
)]
fn strict_promotes_warnings_so_the_exit_code_is_one() {
    let lenient = DiagnosticPolicy::strict(false).apply(vec![warning()], no_lint);
    assert_eq!(lenient[0].severity, Severity::Warning);
    assert_eq!(specforge_common::compute_exit_code(&lenient), 0);

    let strict = DiagnosticPolicy::strict(true).apply(vec![warning()], no_lint);
    assert_eq!(strict[0].severity, Severity::Error);
    assert_eq!(specforge_common::compute_exit_code(&strict), 1);

    // Nothing to promote: still 0.
    let clean = DiagnosticPolicy::strict(true).apply(Vec::new(), no_lint);
    assert_eq!(specforge_common::compute_exit_code(&clean), 0);
}

/// A raise only raises a code's severity, before strict promotes what is left.
#[specforge_test(
    behavior = "exit_code_reflects_diagnostic_severity",
    verify = "a raised severity only raises a code, before strict"
)]
fn a_raise_raises_a_namespaced_code_and_never_lowers() {
    let finding = |code: &str, severity| Diagnostic::untyped(code, severity, "finding");
    let mut raises = Raises::default();
    assert!(raises.is_empty());
    raises.raise("gsd/W304", Severity::Error);
    raises.raise("gsd/E101", Severity::Info);
    raises.raise("gsd/W305", Severity::Error);
    raises.raise("gsd/W305", Severity::Warning);
    assert!(!raises.is_empty());

    assert_eq!(
        raises
            .apply(finding("gsd/W304", Severity::Warning))
            .severity,
        Severity::Error
    );
    assert_eq!(
        raises.apply(finding("gsd/E101", Severity::Error)).severity,
        Severity::Error,
        "a raise to info leaves an error an error"
    );
    assert_eq!(
        raises
            .apply(finding("gsd/W305", Severity::Warning))
            .severity,
        Severity::Error,
        "two raises of one code keep the higher"
    );
    assert_eq!(
        raises
            .apply(finding("gsd/W306", Severity::Warning))
            .severity,
        Severity::Warning,
        "a code with no raise is unchanged"
    );

    let raised: Vec<Diagnostic> = [
        finding("gsd/W304", Severity::Warning),
        finding("gsd/W306", Severity::Warning),
    ]
    .into_iter()
    .map(|d| raises.apply(d))
    .collect();
    let strict = DiagnosticPolicy::strict(true).apply(raised, no_lint);
    assert!(strict.iter().all(|d| d.severity == Severity::Error));
}

/// The policy asks its caller for a named profile's diagnostics, once
/// however often it is named, and promotes them with the rest.
#[specforge_test(
    behavior = "check_diagnostic_policy",
    verify = "each named lint profile adds its diagnostics once, before strict promotes warnings"
)]
fn the_policy_adds_each_named_profile_once_before_strict() {
    let policy = DiagnosticPolicy {
        strict: true,
        lint_profiles: vec![LintProfile::Inferred, LintProfile::Inferred],
    };
    let mut asked = Vec::new();
    let reported = policy.apply(Vec::new(), |profile| {
        asked.push(profile);
        vec![warning()]
    });
    assert_eq!(asked, [LintProfile::Inferred]);
    assert_eq!(reported.len(), 1);
    assert_eq!(reported[0].severity, Severity::Error);

    // A profile nobody named is not asked for.
    let mut asked = Vec::new();
    DiagnosticPolicy::default().apply(Vec::new(), |profile| {
        asked.push(profile);
        Vec::new()
    });
    assert!(asked.is_empty());
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
    assert_eq!(
        LintProfile::ALL.map(LintProfile::name),
        [LINT_PROFILE_NAMES[0], LINT_PROFILE_NAMES[1]]
    );
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

    let reported = vec![
        warning(),
        Diagnostic::untyped("I067", Severity::Info, "module 'm' contains no features"),
    ];
    let pedantic = DiagnosticPolicy {
        strict: false,
        lint_profiles: vec![LintProfile::Pedantic],
    };
    assert_eq!(
        pedantic.apply(reported.clone(), no_lint),
        DiagnosticPolicy::default().apply(reported, no_lint)
    );
}

#[specforge_test(
    behavior = "check_diagnostic_policy",
    verify = "strict never promotes a machine diagnostic, an extension's included"
)]
fn strict_never_promotes_a_machine_diagnostic() {
    let machine = Diagnostic::from_extension("@acme/links", "acme/W901", Severity::Warning, "m")
        .decided_by_machine(true);
    let promoted = DiagnosticPolicy::strict(true).apply(vec![machine, warning()], no_lint);
    assert_eq!(
        promoted[0].severity,
        Severity::Warning,
        "this machine decided it"
    );
    assert_eq!(
        promoted[1].severity,
        Severity::Error,
        "the project's warning is promoted"
    );
    assert_eq!(specforge_common::compute_exit_code(&promoted[..1]), 0);
}

#[specforge_test(
    behavior = "check_diagnostic_policy",
    verify = "the value checks a run did not perform are one I206, counted by scope"
)]
fn i206_counts_each_scope_once() {
    let mut log = specforge_effects::mode::Log::default();
    assert!(
        specforge_project::effects::not_run(&log).is_none(),
        "every check ran"
    );
    log.not_run.insert("registries".into(), 3);
    log.not_run.insert("links".into(), 2);
    let i206 = specforge_project::effects::not_run(&log).expect("five not run");
    assert_eq!(i206.code, "I206");
    assert_eq!(i206.severity, Severity::Info);
    assert!(i206.machine);
    assert!(
        i206.message
            .starts_with("5 value checks not run on this machine (links 2, registries 3)"),
        "{}",
        i206.message
    );
}
