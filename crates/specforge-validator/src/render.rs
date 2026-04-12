use specforge_common::{Diagnostic, Severity};
use std::collections::HashMap;
use std::ops::Range;

type Span = (String, Range<usize>);

/// Render diagnostics to a human-readable string with source context.
pub fn render_diagnostics(diagnostics: &[Diagnostic], sources: &HashMap<String, String>) -> String {
    let mut buf = Vec::new();

    let mut cache = ariadne::sources(
        sources
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );

    for diag in diagnostics {
        let kind = match diag.severity {
            Severity::Error => ariadne::ReportKind::Error,
            Severity::Warning => ariadne::ReportKind::Warning,
            Severity::Info => ariadne::ReportKind::Advice,
        };

        let (file, offset) = if let Some(span) = &diag.span {
            let byte_range = line_col_to_byte_range(
                sources.get(span.file.as_str()).map(|s| s.as_str()).unwrap_or(""),
                span.start_line,
                span.start_col,
                span.end_line,
                span.end_col,
            );
            (span.file.to_string(), byte_range)
        } else {
            let file = sources.keys().next().cloned().unwrap_or_default();
            (file, 0..1)
        };

        let span: Span = (file.clone(), offset.clone());

        let mut builder = ariadne::Report::<Span>::build(kind, span.clone())
            .with_code(diag.code.clone())
            .with_message(diag.message.clone())
            .with_label(
                ariadne::Label::new(span).with_message(diag.message.clone()),
            );

        if let Some(suggestion) = &diag.suggestion {
            builder = builder.with_help(suggestion.clone());
        }

        let report = builder.finish();
        report.write(&mut cache, &mut buf).ok();
    }

    String::from_utf8_lossy(&buf).to_string()
}

fn line_col_to_byte_range(
    source: &str,
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
) -> Range<usize> {
    let mut byte_offset = 0;
    let mut start = 0;
    let mut end = source.len();

    for (i, line) in source.lines().enumerate() {
        let line_num = i + 1; // 1-based
        if line_num == start_line {
            start = byte_offset + start_col.saturating_sub(1);
        }
        if line_num == end_line {
            end = byte_offset + end_col.saturating_sub(1);
            break;
        }
        byte_offset += line.len() + 1; // +1 for newline
    }

    let start = start.min(source.len());
    let end = end.min(source.len()).max(start + 1);
    start..end
}
