//! What a surface reports from a project's diagnostics: lint profiles and
//! strict promotion, in one place for `check`, `analyze` and MCP
//! `validate` and `analyze`.

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use specforge_common::{Diagnostic, Severity, inference, load_project_config};

/// The inference density above which the `inferred` profile reports I202,
/// unless `inference.density_threshold` in specforge.json says otherwise.
const DEFAULT_DENSITY_THRESHOLD: f64 = 0.05;

/// The names of the lint profiles, as `specforge check --lint` and MCP
/// validate's `lint` take them. Any other name is refused.
pub const LINT_PROFILE_NAMES: &[&str] = &["inferred", "pedantic"];

/// A lint profile: a closed set of extra checks a surface may ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LintProfile {
    /// The inference-quality checks I200 and I202, when
    /// `specforge-infer.json` exists.
    Inferred,
    /// The explicit name of the default: info diagnostics are always
    /// reported, so it adds nothing.
    Pedantic,
}

impl LintProfile {
    /// The profile's name, one of [`LINT_PROFILE_NAMES`].
    pub fn name(self) -> &'static str {
        match self {
            LintProfile::Inferred => "inferred",
            LintProfile::Pedantic => "pedantic",
        }
    }
}

impl fmt::Display for LintProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for LintProfile {
    type Err = UnknownLintProfile;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "inferred" => Ok(LintProfile::Inferred),
            "pedantic" => Ok(LintProfile::Pedantic),
            _ => Err(UnknownLintProfile {
                requested: name.to_string(),
            }),
        }
    }
}

/// A lint profile name SpecForge does not define.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownLintProfile {
    pub requested: String,
}

impl fmt::Display for UnknownLintProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Unknown lint profile '{}' (available: {})",
            self.requested,
            LINT_PROFILE_NAMES.join(", ")
        )
    }
}

impl std::error::Error for UnknownLintProfile {}

/// How a surface turns a project's diagnostics into what it reports.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticPolicy {
    /// Warnings are promoted to errors.
    pub strict: bool,
    /// Extra lint profiles: their diagnostics are added before strict
    /// promotes warnings.
    pub lint_profiles: Vec<LintProfile>,
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
        if self.lint_profiles.contains(&LintProfile::Inferred) {
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
