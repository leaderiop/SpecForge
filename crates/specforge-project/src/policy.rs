//! What a surface reports from a project's diagnostics: lint profiles and
//! strict promotion, in one place for `check`, `analyze` and MCP
//! `validate` and `analyze`.

use std::path::Path;

use specforge_common::{Diagnostic, Severity, inference, load_project_config};

/// The inference density above which the `inferred` profile reports I202,
/// unless `inference.density_threshold` in specforge.json says otherwise.
const DEFAULT_DENSITY_THRESHOLD: f64 = 0.05;

/// How a surface turns a project's diagnostics into what it reports.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticPolicy {
    /// Warnings are promoted to errors.
    pub strict: bool,
    /// Extra lint profiles (`inferred`: the inference-quality checks
    /// I200 and I202, when `specforge-infer.json` exists).
    pub lint_profiles: Vec<String>,
}

impl DiagnosticPolicy {
    /// Strict or not, with no lint profile.
    pub fn strict(strict: bool) -> Self {
        DiagnosticPolicy {
            strict,
            lint_profiles: Vec::new(),
        }
    }

    /// The diagnostics the project at `root` reports under this policy:
    /// the lint profiles' diagnostics are added, then strict promotes
    /// warnings (the added ones too).
    pub fn apply(&self, root: &Path, mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
        if self.lint_profiles.iter().any(|p| p == "inferred") {
            diagnostics.extend(inferred_lint(root));
        }
        self.promote(&mut diagnostics);
        diagnostics
    }

    /// Promote warnings to errors when strict: for findings that have no
    /// project-level lints (analysis passes).
    pub fn promote(&self, diagnostics: &mut [Diagnostic]) {
        if self.strict {
            for diagnostic in diagnostics {
                if diagnostic.severity == Severity::Warning {
                    diagnostic.severity = Severity::Error;
                }
            }
        }
    }
}

/// [`DiagnosticPolicy::apply`].
pub fn apply_policy(
    root: &Path,
    diagnostics: Vec<Diagnostic>,
    policy: &DiagnosticPolicy,
) -> Vec<Diagnostic> {
    policy.apply(root, diagnostics)
}

/// The `inferred` profile: I200 and I202 from `specforge-infer.json`, when
/// it exists and parses.
fn inferred_lint(root: &Path) -> Vec<Diagnostic> {
    let Ok(manifest) = inference::load_inference_manifest(root) else {
        return Vec::new();
    };
    let density_threshold = load_project_config(root)
        .inference
        .density_threshold
        .unwrap_or(DEFAULT_DENSITY_THRESHOLD);
    inference::compute_inference_diagnostics(root, &manifest, density_threshold)
}
