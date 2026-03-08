use specforge_common::{Diagnostic, Severity};

/// Produce a summary line like "2 errors, 1 warning, 1 info".
/// When errors exist, the line is wrapped in red ANSI escape codes.
pub fn diagnostic_summary(diagnostics: &[Diagnostic]) -> String {
    let errors = diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
    let warnings = diagnostics.iter().filter(|d| d.severity == Severity::Warning).count();
    let infos = diagnostics.iter().filter(|d| d.severity == Severity::Info).count();

    let plural = |n: usize, word: &str| -> String {
        if n == 1 {
            format!("{} {}", n, word)
        } else {
            format!("{} {}s", n, word)
        }
    };

    let text = format!(
        "{}, {}, {}",
        plural(errors, "error"),
        plural(warnings, "warning"),
        plural(infos, "info"),
    );

    if errors > 0 {
        format!("\x1b[1;31m{}\x1b[0m", text)
    } else {
        text
    }
}
