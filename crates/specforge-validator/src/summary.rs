use specforge_common::{Diagnostic, Severity};
use std::collections::BTreeMap;

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

/// Produce a detailed summary grouping diagnostics by code, showing top occurrences.
/// Example:
/// ```text
/// 3 errors, 2 warnings, 0 infos
///   E001 (2): Parse error
///   E003 (1): Unresolved reference
///   W001 (2): Missing verify statement
/// ```
pub fn diagnostic_summary_detailed(diagnostics: &[Diagnostic]) -> String {
    let summary_line = diagnostic_summary(diagnostics);

    if diagnostics.is_empty() {
        return summary_line;
    }

    // Group by code, count occurrences
    let mut by_code: BTreeMap<&str, usize> = BTreeMap::new();
    for d in diagnostics {
        *by_code.entry(&d.code).or_insert(0) += 1;
    }

    // Sort by count descending, then code ascending
    let mut entries: Vec<(&&str, &usize)> = by_code.iter().collect();
    entries.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

    // Show top codes (up to 5)
    let mut lines = vec![summary_line];
    for (code, count) in entries.iter().take(5) {
        lines.push(format!("  {} ({})", code, count));
    }
    if entries.len() > 5 {
        let remaining: usize = entries.iter().skip(5).map(|(_, c)| **c).sum();
        lines.push(format!("  ... and {} more from {} other codes", remaining, entries.len() - 5));
    }

    lines.push(String::from("\nhint: run `specforge explain <code>` for details on any diagnostic code"));

    lines.join("\n")
}
