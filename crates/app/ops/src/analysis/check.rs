//! `specforge check`: what a project reports, and whether it passed.
//!
//! One operation serves the CLI (`specforge check`) and MCP
//! (`specforge.validate`): it applies the diagnostic policy (lint profiles,
//! then strict), takes the verdict over everything reported, records the
//! verdict over everything reported, and says which diagnostics the caller's
//! severity filter shows. The filter never changes the verdict. Surfaces keep the choice of what was
//! compiled (`reported`), rendering, exit codes and `isError` (ADR 0018).

use serde_json::{Value, json};
use specforge_common::{Diagnostic, Severity};
use specforge_project::{DiagnosticPolicy, LINT_PROFILE_NAMES, LintProfile};

use crate::view::ProjectView;
use crate::{OpError, OpErrorKind, Refusal};

/// What to report. `Default` is a plain `specforge check`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckOptions {
    /// Promote warnings to errors, before the verdict.
    pub strict: bool,
    pub lint_profiles: Vec<LintProfile>,
    /// Show only this severity (after strict promotion). Never the verdict.
    pub severity: Option<Severity>,
}

pub use specforge_common::Counts;

/// What a check reported.
#[derive(Debug, Clone)]
pub struct CheckOutcome {
    /// Everything reported, policy applied, in `check`'s order.
    pub reported: Vec<Diagnostic>,
    pub counts: Counts,
    pub severity: Option<Severity>,
}

impl CheckOutcome {
    /// No error among everything reported (strict already promoted).
    pub fn ok(&self) -> bool {
        self.counts.errors == 0
    }

    /// What the severity filter shows, in order.
    pub fn shown(&self) -> Vec<&Diagnostic> {
        self.reported
            .iter()
            .filter(|d| self.severity.is_none_or(|severity| d.severity == severity))
            .collect()
    }

    /// `{"ok", "errors", "warnings", "infos", "shown"}`: the verdict MCP
    /// sends in `_meta`.
    pub fn verdict_json(&self) -> Value {
        json!({
            "ok": self.ok(),
            "errors": self.counts.errors,
            "warnings": self.counts.warnings,
            "infos": self.counts.infos,
            "shown": self.shown().len(),
        })
    }
}

/// The field of a check request a refusal can be about (ADR 0072).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckField {
    /// The severity filter.
    Severity,
    /// The lint profiles.
    LintProfiles,
}

/// A severity filter naming no severity (`InvalidInput`).
pub const UNKNOWN_SEVERITY: &str = "unknown_severity";
/// A lint profile naming no profile (`InvalidInput`).
pub const UNKNOWN_LINT_PROFILE: &str = "unknown_lint_profile";

/// The severities a filter names, lowercase. Matched ignoring ASCII case.
pub const SEVERITY_NAMES: &[&str] = &["error", "warning", "info"];

/// Why `check` refuses a rootless view: a lint profile needs the root (`no_project`).
const NO_ROOT: &str = "a lint profile needs a project root, and this project has none";

/// The severity `name` names, ignoring ASCII case (`"Error"`, the
/// payload's own spelling, is `error`). Any other name is
/// [`UNKNOWN_SEVERITY`], about the severity filter.
pub fn parse_severity(name: &str) -> Result<Severity, Refusal<CheckField>> {
    match name.to_ascii_lowercase().as_str() {
        "error" => Ok(Severity::Error),
        "warning" => Ok(Severity::Warning),
        "info" => Ok(Severity::Info),
        _ => {
            Err(unknown(UNKNOWN_SEVERITY, "severity", name, SEVERITY_NAMES)
                .about(CheckField::Severity))
        }
    }
}

/// The lint profiles `names` name; the first unknown one is
/// [`UNKNOWN_LINT_PROFILE`], about the lint profiles.
pub fn parse_lint_profiles<S: AsRef<str>>(
    names: &[S],
) -> Result<Vec<LintProfile>, Refusal<CheckField>> {
    names
        .iter()
        .map(|name| {
            let name = name.as_ref();
            name.parse().map_err(|_| {
                unknown(
                    UNKNOWN_LINT_PROFILE,
                    "lint profile",
                    name,
                    LINT_PROFILE_NAMES,
                )
                .about(CheckField::LintProfiles)
            })
        })
        .collect()
}

/// `Unknown {what} '{requested}' (available: {names})`, invalid input under
/// `code`, with `did you mean '{closest}'?` when a name is close to
/// `requested` in lowercase.
fn unknown(code: &'static str, what: &str, requested: &str, names: &[&str]) -> OpError {
    let error = OpError::new(
        OpErrorKind::InvalidInput,
        code,
        format!(
            "Unknown {what} '{requested}' (available: {})",
            names.join(", ")
        ),
    );
    match specforge_common::suggest::find_close_match(
        &requested.to_ascii_lowercase(),
        names.iter().copied(),
    ) {
        Some(close) => error.with_suggestion(format!("did you mean '{close}'?")),
        None => error,
    }
}

/// Report what the compile of `view` reported (`view.reported()`) under
/// `options`: the policy applied and the verdict taken over all of it. The one
/// refusal is of the project, so it is about no field: a lint profile on a view
/// with no root.
pub fn check(view: &ProjectView<'_>, options: &CheckOptions) -> Result<CheckOutcome, OpError> {
    if view.root().is_none() && !options.lint_profiles.is_empty() {
        return Err(OpError::no_project(NO_ROOT));
    }
    let policy = DiagnosticPolicy {
        strict: options.strict,
        lint_profiles: options.lint_profiles.clone(),
    };
    let reported = policy.apply(view.reported(), |profile| match profile {
        LintProfile::Inferred => crate::read::infer::lint(view),
        LintProfile::Pedantic => Vec::new(),
    });
    let counts = Counts::of(&reported);
    Ok(CheckOutcome {
        reported,
        counts,
        severity: options.severity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::codes;
    use specforge_project::CompiledProject;
    use specforge_registry::RegistryBuild;
    use specforge_test_macros::test as specforge_test;
    use tempfile::TempDir;

    const CONFIG: &str =
        r#"{"extensions": ["@specforge/software", "@specforge/governance", "@specforge/product"]}"#;

    /// A feature with a lifecycle state: infos only (no error, no warning).
    const CLEAN: &str = "feature alpha \"A\" {\n  problem \"p\"\n  status done\n}\n";

    /// An unreferenced invariant: a warning (W003), and no error.
    const WARNING: &str = "invariant lonely \"Lonely\" {\n  guarantee \"g\"\n}\n";

    /// A behavior that names an invariant nobody declares: an error (E003).
    const ERROR: &str =
        "behavior act \"Act\" {\n  contract \"MUST act\"\n  invariants [missing]\n}\n";

    fn project(files: &[&str]) -> TempDir {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("specforge.json"), CONFIG).unwrap();
        for (i, text) in files.iter().enumerate() {
            std::fs::write(dir.path().join(format!("f{i}.spec")), text).unwrap();
        }
        dir
    }

    fn compile(dir: &TempDir) -> CompiledProject {
        let runtime = specforge_component::ComponentRuntime::with_user_cache();
        CompiledProject::compile(
            &specforge_common::ProjectRoot::of(dir.path()).unwrap(),
            Some(std::sync::Arc::new(runtime)),
        )
    }

    fn run(compiled: &CompiledProject, options: &CheckOptions) -> CheckOutcome {
        check(&ProjectView::of(compiled), options).unwrap()
    }

    fn severities(diagnostics: &[&Diagnostic]) -> Vec<Severity> {
        diagnostics.iter().map(|d| d.severity).collect()
    }

    #[specforge_test(
        behavior = "check_diagnostic_policy",
        verify = "the verdict is taken over every reported diagnostic, never the filtered ones"
    )]
    fn the_verdict_ignores_the_severity_filter() {
        let dir = project(&[CLEAN, ERROR]);
        let compiled = compile(&dir);
        let options = CheckOptions {
            severity: Some(Severity::Info),
            ..CheckOptions::default()
        };
        let outcome = run(&compiled, &options);

        assert!(outcome.counts.errors > 0, "{:?}", outcome.reported);
        assert!(outcome.counts.infos > 0, "{:?}", outcome.reported);
        assert!(!outcome.ok(), "an error the filter hides still fails");
        let shown = outcome.shown();
        assert_eq!(
            severities(&shown),
            vec![Severity::Info; outcome.counts.infos]
        );
        assert_eq!(
            outcome.verdict_json(),
            json!({
                "ok": false,
                "errors": outcome.counts.errors,
                "warnings": outcome.counts.warnings,
                "infos": outcome.counts.infos,
                "shown": outcome.counts.infos,
            })
        );
    }

    #[specforge_test(
        behavior = "check_diagnostic_policy",
        verify = "strict promotes warnings before the verdict, so a strict check with warnings is not clean"
    )]
    fn strict_promotes_warnings_before_the_verdict() {
        let dir = project(&[WARNING]);
        let compiled = compile(&dir);

        let lenient = run(&compiled, &CheckOptions::default());
        assert_eq!(lenient.counts.errors, 0, "{:?}", lenient.reported);
        assert!(lenient.counts.warnings > 0, "{:?}", lenient.reported);
        assert!(lenient.ok());

        let strict = run(
            &compiled,
            &CheckOptions {
                strict: true,
                ..CheckOptions::default()
            },
        );
        assert_eq!(strict.counts.warnings, 0);
        assert_eq!(strict.counts.errors, lenient.counts.warnings);
        assert!(!strict.ok());
        // The filter sees the promoted severity.
        let errors_only = run(
            &compiled,
            &CheckOptions {
                strict: true,
                severity: Some(Severity::Error),
                ..CheckOptions::default()
            },
        );
        assert_eq!(errors_only.shown().len(), lenient.counts.warnings);
    }

    #[test]
    fn no_root_with_lint_is_refused() {
        let reported = vec![Diagnostic::untyped("W002", Severity::Warning, "unused")];
        let project = specforge_project::testing::Assembled::over(RegistryBuild::default())
            .reporting(reported);
        let view = crate::testing::Viewed::rootless_view(&project);

        let options = CheckOptions {
            lint_profiles: vec![LintProfile::Inferred],
            ..CheckOptions::default()
        };
        let refused = check(&view, &options).unwrap_err();
        assert_eq!(refused.code, "no_project");
        assert_eq!(refused.kind, OpErrorKind::PreconditionFailed);
        // Nothing that needs the root: a rootless view checks fine.
        let outcome = check(&view, &CheckOptions::default()).unwrap();
        assert_eq!(
            outcome.counts,
            Counts {
                errors: 0,
                warnings: 1,
                infos: 0
            }
        );
    }

    #[test]
    fn parse_severity_ignores_case_and_refuses_others() {
        for (name, severity) in [
            ("error", Severity::Error),
            ("Error", Severity::Error),
            ("ERROR", Severity::Error),
            ("warning", Severity::Warning),
            ("Warning", Severity::Warning),
            ("info", Severity::Info),
            ("Info", Severity::Info),
        ] {
            assert_eq!(parse_severity(name), Ok(severity), "{name}");
        }
        for name in SEVERITY_NAMES {
            assert_eq!(parse_severity(name).unwrap().to_string(), *name);
        }
        let refused = parse_severity("errors").unwrap_err();
        assert_eq!(
            refused.error.message,
            "Unknown severity 'errors' (available: error, warning, info)"
        );
        assert_eq!(refused.error.code, UNKNOWN_SEVERITY);
        assert_eq!(
            refused.error.suggestion.as_deref(),
            Some("did you mean 'error'?")
        );
        assert_eq!(refused.subject, Some(CheckField::Severity));
        assert!(parse_severity("").is_err());
    }

    #[specforge_test(
        behavior = "provide_mcp_validate_tool",
        verify = "an unknown severity_filter or lint profile is invalid input"
    )]
    fn check_refusals_have_their_own_codes() {
        let severity = parse_severity("errors").unwrap_err().error;
        assert_eq!(severity.code, UNKNOWN_SEVERITY);
        assert_eq!(severity.kind, OpErrorKind::InvalidInput);
        assert_eq!(
            severity.suggestion.as_deref(),
            Some("did you mean 'error'?")
        );
        let profile = parse_lint_profiles(&["pedantik"]).unwrap_err().error;
        assert_eq!(profile.code, UNKNOWN_LINT_PROFILE);
        assert_eq!(profile.kind, OpErrorKind::InvalidInput);
        assert_eq!(
            profile.suggestion.as_deref(),
            Some("did you mean 'pedantic'?")
        );
    }

    #[test]
    fn parse_lint_profiles_refuses_the_first_unknown_name() {
        assert_eq!(
            parse_lint_profiles(&["inferred", "pedantic"]),
            Ok(vec![LintProfile::Inferred, LintProfile::Pedantic])
        );
        assert_eq!(parse_lint_profiles::<&str>(&[]), Ok(vec![]));
        let refused = parse_lint_profiles(&["inferred", "pedantik", "nonsense"]).unwrap_err();
        assert_eq!(
            refused.error.message,
            "Unknown lint profile 'pedantik' (available: inferred, pedantic)"
        );
        assert_eq!(
            refused.error.suggestion.as_deref(),
            Some("did you mean 'pedantic'?")
        );
    }

    #[test]
    fn counts_of_mixed() {
        let diagnostics = [
            Diagnostic::new(codes::E003, "a"),
            Diagnostic::untyped("W003", Severity::Warning, "b"),
            Diagnostic::untyped("W004", Severity::Warning, "c"),
            Diagnostic::untyped("I067", Severity::Info, "d"),
            Diagnostic::untyped("I068", Severity::Info, "e"),
            Diagnostic::untyped("I080", Severity::Info, "f"),
        ];
        assert_eq!(
            Counts::of(&diagnostics),
            Counts {
                errors: 1,
                warnings: 2,
                infos: 3
            }
        );
        assert_eq!(Counts::of(&[]), Counts::default());
    }

    #[specforge_test(
        invariant = "mcp_structured_error_responses",
        verify = "an operation states the request field each refusal is about, and a refusal of the project or the environment is about none"
    )]
    fn check_states_which_field_a_refusal_is_about() {
        assert_eq!(
            parse_severity("errors").unwrap_err().subject,
            Some(CheckField::Severity)
        );
        assert_eq!(
            parse_lint_profiles(&["pedantik"]).unwrap_err().subject,
            Some(CheckField::LintProfiles)
        );
    }
}
