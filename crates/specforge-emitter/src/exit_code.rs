use specforge_common::{Diagnostic, Severity};

/// Compute the process exit code from collected diagnostics.
///
/// Returns 0 if no error-level diagnostics exist, 1 otherwise. Strict
/// mode is not a separate rule: `specforge_project::DiagnosticPolicy`
/// promotes warnings to errors before the exit code is computed.
pub fn compute_exit_code(diagnostics: &[Diagnostic]) -> i32 {
    if diagnostics
        .iter()
        .any(|d| matches!(d.severity, Severity::Error))
    {
        1
    } else {
        0
    }
}
