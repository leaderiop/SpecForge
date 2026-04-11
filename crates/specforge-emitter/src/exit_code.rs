use specforge_common::{Diagnostic, Severity};

/// Compute the process exit code from collected diagnostics.
///
/// Returns 0 if no error-level diagnostics exist, 1 otherwise.
/// In strict mode, warnings also cause exit code 1.
pub fn compute_exit_code(diagnostics: &[Diagnostic]) -> i32 {
    if diagnostics.iter().any(|d| matches!(d.severity, Severity::Error)) {
        1
    } else {
        0
    }
}

/// Compute exit code with strict mode support.
///
/// When `strict` is true, warnings are treated as errors (exit 1).
pub fn compute_exit_code_strict(diagnostics: &[Diagnostic], strict: bool) -> i32 {
    if diagnostics.iter().any(|d| matches!(d.severity, Severity::Error)) {
        return 1;
    }
    if strict && diagnostics.iter().any(|d| matches!(d.severity, Severity::Warning)) {
        return 1;
    }
    0
}
