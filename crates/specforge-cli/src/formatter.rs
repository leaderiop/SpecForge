use ariadne::{Color, Label, Report, ReportKind, Source};
use specforge_common::{Diagnostic, Severity};
use std::collections::HashMap;
use std::io::Write;

/// Format and print diagnostics to stderr using ariadne for rustc-style output.
pub fn print_diagnostics(
    diagnostics: &[Diagnostic],
    sources: &HashMap<String, String>,
) {
    let mut buf = Vec::new();
    write_diagnostics(diagnostics, sources, &mut buf);
    eprint!("{}", String::from_utf8_lossy(&buf));
}

/// Format diagnostics to a writer (testable).
pub fn write_diagnostics(
    diagnostics: &[Diagnostic],
    sources: &HashMap<String, String>,
    writer: &mut impl Write,
) {
    for diag in diagnostics {
        write_single_diagnostic(diag, sources, writer);
    }

    // Summary line
    let errors = diagnostics.iter().filter(|d| d.severity() == Severity::Error).count();
    let warnings = diagnostics.iter().filter(|d| d.severity() == Severity::Warning).count();
    let infos = diagnostics.iter().filter(|d| d.severity() == Severity::Info).count();

    if errors > 0 || warnings > 0 || infos > 0 {
        let mut parts = Vec::new();
        if errors > 0 {
            parts.push(format!("{errors} error{}", if errors == 1 { "" } else { "s" }));
        }
        if warnings > 0 {
            parts.push(format!("{warnings} warning{}", if warnings == 1 { "" } else { "s" }));
        }
        if infos > 0 {
            parts.push(format!("{infos} info"));
        }
        writeln!(writer, "{}", parts.join(", ")).ok();
    } else {
        writeln!(writer, "0 errors").ok();
    }
}

fn write_single_diagnostic(
    diag: &Diagnostic,
    sources: &HashMap<String, String>,
    writer: &mut impl Write,
) {
    let file = &diag.span.file;
    let source_text = sources.get(file).map(|s| s.as_str()).unwrap_or("");

    // Convert line/col span to byte offset
    let offset = line_col_to_offset(source_text, diag.span.start_line, diag.span.start_col);

    let kind = match diag.severity() {
        Severity::Error => ReportKind::Error,
        Severity::Warning => ReportKind::Warning,
        Severity::Info => ReportKind::Advice,
    };

    let color = match diag.severity() {
        Severity::Error => Color::Red,
        Severity::Warning => Color::Yellow,
        Severity::Info => Color::Blue,
    };

    let mut report = Report::build(kind, file.clone(), offset)
        .with_code(diag.code.to_string())
        .with_message(&diag.message);

    // Primary label at the diagnostic span
    let start = line_col_to_offset(source_text, diag.span.start_line, diag.span.start_col);
    let end = line_col_to_offset(source_text, diag.span.end_line, diag.span.end_col);
    let span_end = if end > start { end } else { start + 1 };

    report = report.with_label(
        Label::new((file.clone(), start..span_end))
            .with_color(color),
    );

    // Additional labels
    for label in &diag.labels {
        let label_source = sources.get(&label.span.file).map(|s| s.as_str()).unwrap_or("");
        let ls = line_col_to_offset(label_source, label.span.start_line, label.span.start_col);
        let le = line_col_to_offset(label_source, label.span.end_line, label.span.end_col);
        let label_end = if le > ls { le } else { ls + 1 };
        report = report.with_label(
            Label::new((label.span.file.clone(), ls..label_end))
                .with_message(&label.message)
                .with_color(Color::Cyan),
        );
    }

    if let Some(help) = &diag.help {
        report = report.with_help(help);
    }

    let report = report.finish();
    report
        .write((file.clone(), Source::from(source_text)), writer)
        .ok();
}

/// Convert 1-based line and column (u32) to byte offset in source text.
fn line_col_to_offset(source: &str, line: u32, col: u32) -> usize {
    let line = line as usize;
    let col = col as usize;
    let mut current_line = 1;
    let mut line_start = 0;

    for (i, ch) in source.char_indices() {
        if current_line == line {
            // Found the target line, add column offset
            let target_col = col.saturating_sub(1); // col is 1-based
            for (col_offset, (j, _)) in source[line_start..].char_indices().enumerate() {
                if col_offset >= target_col {
                    return line_start + j;
                }
            }
            return source.len();
        }
        if ch == '\n' {
            current_line += 1;
            line_start = i + 1;
        }
    }

    // If we're looking for the last line and it has no newline at end
    if current_line == line {
        let target_col = col.saturating_sub(1);
        for (col_offset, (j, _)) in source[line_start..].char_indices().enumerate() {
            if col_offset >= target_col {
                return line_start + j;
            }
        }
    }

    // If line is past the end, return the end of source
    source.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::{SourceSpan, ValidationCode};

    fn format_summary(errors: usize, warnings: usize, infos: usize) -> String {
        let mut parts = Vec::new();
        if errors > 0 {
            parts.push(format!("{errors} error{}", if errors == 1 { "" } else { "s" }));
        }
        if warnings > 0 {
            parts.push(format!("{warnings} warning{}", if warnings == 1 { "" } else { "s" }));
        }
        if infos > 0 {
            parts.push(format!("{infos} info"));
        }
        if parts.is_empty() {
            "0 errors".to_string()
        } else {
            parts.join(", ")
        }
    }

    fn make_sources() -> HashMap<String, String> {
        let mut sources = HashMap::new();
        sources.insert(
            "test.spec".to_string(),
            "invariant data_integrity \"Test\" {\n  guarantee \"\"\"must hold\"\"\"\n}\n".to_string(),
        );
        sources
    }

    #[test]
    fn line_col_offset_conversion() {
        let source = "line1\nline2\nline3\n";
        assert_eq!(line_col_to_offset(source, 1, 1), 0);
        assert_eq!(line_col_to_offset(source, 2, 1), 6);
        assert_eq!(line_col_to_offset(source, 3, 1), 12);
        assert_eq!(line_col_to_offset(source, 2, 3), 8); // 'n' in "line2"
    }

    #[test]
    fn format_error_diagnostic() {
        let sources = make_sources();
        let diag = Diagnostic::new(
            ValidationCode::E001,
            SourceSpan::new("test.spec", 1, 1, 1, 20),
            "unresolved reference `nonexistent_inv`",
        )
        .with_help("did you mean `data_integrity`?");

        let mut buf = Vec::new();
        write_diagnostics(&[diag], &sources, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("E001"));
        assert!(output.contains("unresolved reference"));
        assert!(output.contains("1 error"));
    }

    #[test]
    fn format_warning_diagnostic() {
        let sources = make_sources();
        let diag = Diagnostic::new(
            ValidationCode::W001,
            SourceSpan::new("test.spec", 1, 1, 1, 20),
            "orphan behavior",
        );

        let mut buf = Vec::new();
        write_diagnostics(&[diag], &sources, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("W001"));
        assert!(output.contains("1 warning"));
    }

    #[test]
    fn format_multiple_diagnostics() {
        let sources = make_sources();
        let diags = vec![
            Diagnostic::new(
                ValidationCode::E001,
                SourceSpan::new("test.spec", 1, 1, 1, 20),
                "error one",
            ),
            Diagnostic::new(
                ValidationCode::W001,
                SourceSpan::new("test.spec", 2, 1, 2, 10),
                "warning one",
            ),
            Diagnostic::new(
                ValidationCode::I003,
                SourceSpan::new("test.spec", 1, 1, 1, 5),
                "info one",
            ),
        ];

        let mut buf = Vec::new();
        write_diagnostics(&diags, &sources, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("1 error, 1 warning, 1 info"));
    }

    #[test]
    fn format_zero_diagnostics() {
        let sources = make_sources();
        let mut buf = Vec::new();
        write_diagnostics(&[], &sources, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("0 errors"));
    }

    #[test]
    fn summary_format() {
        assert_eq!(format_summary(2, 1, 0), "2 errors, 1 warning");
        assert_eq!(format_summary(0, 0, 3), "3 info");
        assert_eq!(format_summary(1, 0, 0), "1 error");
        assert_eq!(format_summary(0, 0, 0), "0 errors");
    }

    // --- Snapshot tests ---

    /// Strip ANSI escape codes for deterministic snapshots.
    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // Skip until 'm' (end of SGR sequence)
                for inner in chars.by_ref() {
                    if inner == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn render(diagnostics: &[Diagnostic], sources: &HashMap<String, String>) -> String {
        let mut buf = Vec::new();
        write_diagnostics(diagnostics, sources, &mut buf);
        strip_ansi(&String::from_utf8(buf).unwrap())
    }

    #[test]
    fn snapshot_error_diagnostic() {
        let sources = make_sources();
        let diag = Diagnostic::new(
            ValidationCode::E001,
            SourceSpan::new("test.spec", 1, 1, 1, 20),
            "unresolved reference `nonexistent_inv`",
        )
        .with_help("did you mean `data_integrity`?");

        insta::assert_snapshot!(render(&[diag], &sources));
    }

    #[test]
    fn snapshot_warning_diagnostic() {
        let sources = make_sources();
        let diag = Diagnostic::new(
            ValidationCode::W001,
            SourceSpan::new("test.spec", 1, 1, 1, 20),
            "orphan behavior",
        );

        insta::assert_snapshot!(render(&[diag], &sources));
    }

    #[test]
    fn snapshot_info_diagnostic() {
        let sources = make_sources();
        let diag = Diagnostic::new(
            ValidationCode::I003,
            SourceSpan::new("test.spec", 1, 1, 1, 5),
            "newer format available",
        );

        insta::assert_snapshot!(render(&[diag], &sources));
    }

    #[test]
    fn snapshot_error_with_label() {
        let mut sources = HashMap::new();
        sources.insert(
            "types.spec".to_string(),
            "type UserProfile \"User Profile\" {\n}\ntype UserProfile \"Duplicate\" {\n}\n".to_string(),
        );
        let diag = Diagnostic::new(
            ValidationCode::E002,
            SourceSpan::new("types.spec", 3, 6, 3, 17),
            "duplicate entity name `UserProfile`",
        )
        .with_label(
            SourceSpan::new("types.spec", 1, 6, 1, 17),
            "first defined here",
        );

        insta::assert_snapshot!(render(&[diag], &sources));
    }

    #[test]
    fn snapshot_multiple_mixed() {
        let sources = make_sources();
        let diags = vec![
            Diagnostic::new(
                ValidationCode::E001,
                SourceSpan::new("test.spec", 1, 1, 1, 20),
                "error one",
            ),
            Diagnostic::new(
                ValidationCode::W001,
                SourceSpan::new("test.spec", 2, 3, 2, 10),
                "warning one",
            ),
            Diagnostic::new(
                ValidationCode::I003,
                SourceSpan::new("test.spec", 1, 1, 1, 5),
                "info one",
            ),
        ];

        insta::assert_snapshot!(render(&diags, &sources));
    }

    #[test]
    fn snapshot_zero_diagnostics() {
        let sources = make_sources();
        insta::assert_snapshot!(render(&[], &sources));
    }

    #[test]
    fn snapshot_help_and_suggestion() {
        let sources = make_sources();
        let diag = Diagnostic::new(
            ValidationCode::E001,
            SourceSpan::new("test.spec", 1, 11, 1, 25),
            "unresolved reference `data_persistance`",
        )
        .with_help("did you mean `data_persistence`?");

        insta::assert_snapshot!(render(&[diag], &sources));
    }

    #[test]
    fn snapshot_multiline_span() {
        let mut sources = HashMap::new();
        sources.insert(
            "behaviors.spec".to_string(),
            "behavior auth_login \"Auth Login\" {\n  contract \"\"\"users MUST authenticate\n    before accessing resources\"\"\"\n}\n".to_string(),
        );
        let diag = Diagnostic::new(
            ValidationCode::W004,
            SourceSpan::new("behaviors.spec", 1, 1, 3, 30),
            "unverified behavior `auth_login`",
        )
        .with_help("add a `verify` or `scenario` block");

        insta::assert_snapshot!(render(&[diag], &sources));
    }
}
