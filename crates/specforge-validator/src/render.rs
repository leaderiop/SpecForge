use specforge_common::{Diagnostic, Severity};
use std::collections::HashMap;
use std::ops::Range;

type Span = (String, Range<usize>);

/// Render diagnostics to a human-readable string with source context. The
/// output carries no ANSI escape.
pub fn render_diagnostics(diagnostics: &[Diagnostic], sources: &HashMap<String, String>) -> String {
    render_diagnostics_colored(diagnostics, sources, false)
}

/// Render diagnostics as [`render_diagnostics`] does, colour-coded when
/// `color` is true: each severity heading is red for an error, yellow for a
/// warning and blue for an info. With `color` false the output is
/// byte-identical to [`render_diagnostics`]. The caller decides, since only
/// it knows whether the text reaches a terminal.
pub fn render_diagnostics_colored(
    diagnostics: &[Diagnostic],
    sources: &HashMap<String, String>,
    color: bool,
) -> String {
    let mut buf = Vec::new();

    let mut cache = ariadne::sources(sources.iter().map(|(k, v)| (k.clone(), v.clone())));

    for diag in diagnostics {
        let kind = match diag.severity {
            Severity::Error => ariadne::ReportKind::Error,
            Severity::Warning => ariadne::ReportKind::Warning,
            // ariadne colours advice lavender; a custom kind with the same
            // label is blue. Only when colouring: a custom kind's colour
            // ignores `with_color(false)`.
            Severity::Info if color => ariadne::ReportKind::Custom("Advice", ariadne::Color::Blue),
            Severity::Info => ariadne::ReportKind::Advice,
        };

        let (file, offset) = if let Some(span) = &diag.span {
            let byte_range = line_col_to_byte_range(
                sources
                    .get(span.file.as_str())
                    .map(|s| s.as_str())
                    .unwrap_or(""),
                span.start_line,
                span.start_col,
                span.end_line,
                span.end_col,
            );
            (span.file.to_string(), byte_range)
        } else {
            // C14-13: anchor spanless diagnostics deterministically — the
            // lexicographically first source, never HashMap iteration order.
            let file = sources.keys().min().cloned().unwrap_or_default();
            let span_end = sources.get(&file).map(|s| s.len().min(1)).unwrap_or(0);
            (file, 0..span_end)
        };

        let span: Span = (file.clone(), offset.clone());

        // The message goes in the heading only: repeated as the label, a
        // long one runs through the snippet and wraps the box apart. The
        // label's text is empty rather than absent because ariadne draws
        // no underline or range for a label without a message.
        let mut builder = ariadne::Report::<Span>::build(kind, span.clone())
            .with_code(diag.code.clone())
            .with_message(diag.message.clone())
            .with_label(ariadne::Label::new(span).with_message(""));

        if let Some(suggestion) = &diag.suggestion {
            builder = builder.with_help(suggestion.clone());
        }

        // Colour only on request: ANSI escapes corrupt piped/agent-facing
        // output (and split substrings for consumers matching rendered
        // lines), so callers ask for it only when writing to a terminal.
        // Spans are byte ranges; ariadne counts chars unless told otherwise,
        // which shifts every span after a multi-byte character.
        let report = builder
            .with_config(
                ariadne::Config::default()
                    .with_color(color)
                    .with_index_type(ariadne::IndexType::Byte),
            )
            .finish();
        // A blank line between reports, so each one reads on its own.
        if !buf.is_empty() {
            buf.push(b'\n');
        }
        report.write(&mut cache, &mut buf).ok();
    }

    // ariadne pads rows with spaces; drop them so the output is clean to
    // copy, diff or store.
    String::from_utf8_lossy(&buf)
        .lines()
        .map(|line| format!("{}\n", line.trim_end()))
        .collect()
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
    let src_bytes = source.as_bytes();

    for (i, line) in source.lines().enumerate() {
        let line_num = i + 1; // 1-based
        if line_num == start_line {
            start = byte_offset + start_col.saturating_sub(1);
        }
        if line_num == end_line {
            end = byte_offset + end_col.saturating_sub(1);
            break;
        }
        byte_offset += line.len();
        // Detect actual line ending: \r\n adds 2, \n adds 1
        if src_bytes.get(byte_offset) == Some(&b'\r')
            && src_bytes.get(byte_offset + 1) == Some(&b'\n')
        {
            byte_offset += 2;
        } else if byte_offset < source.len() {
            byte_offset += 1;
        }
    }

    // C14-13: clamp both ends into the buffer; never emit an empty or
    // past-EOF range (ariadne slices into the source).
    let len = source.len();
    if len == 0 {
        return 0..0;
    }
    let mut start = start.min(len - 1);
    // Spans are byte columns; snap mid-char values to boundaries so ariadne
    // can slice the source (floor the start, ceil the end).
    while start > 0 && !source.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = end.clamp(start + 1, len);
    while end < len && !source.is_char_boundary(end) {
        end += 1;
    }
    start..end
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_common::codes;

    #[test]
    fn line_col_to_byte_range_unix_newlines() {
        // "abc\ndef\nghi\n" — line 2 col 1..3 should be bytes 4..6
        let source = "abc\ndef\nghi\n";
        let range = line_col_to_byte_range(source, 2, 1, 2, 3);
        assert_eq!(range, 4..6);
    }

    #[test]
    fn line_col_to_byte_range_crlf_newlines() {
        // "abc\r\ndef\r\nghi\r\n" — line 2 col 1..3 should be bytes 5..7
        let source = "abc\r\ndef\r\nghi\r\n";
        let range = line_col_to_byte_range(source, 2, 1, 2, 3);
        assert_eq!(range, 5..7);
        assert_eq!(&source[range], "de");
    }

    #[test]
    fn line_col_to_byte_range_crlf_multiline() {
        // Verify that offsets accumulate correctly across multiple CRLF lines
        let source = "line1\r\nline2\r\nline3\r\n";
        // line 3 starts at byte 14 (5+2+5+2), col 1..5 -> bytes 14..18
        let range = line_col_to_byte_range(source, 3, 1, 3, 5);
        assert_eq!(range, 14..18);
        assert_eq!(&source[range], "line");
    }

    #[test]
    fn line_col_to_byte_range_mixed_newlines() {
        // "abc\ndef\r\nghi\n" — line 3 col 1..3 should be bytes 9..11
        // abc(3) + \n(1) + def(3) + \r\n(2) = 9
        let source = "abc\ndef\r\nghi\n";
        let range = line_col_to_byte_range(source, 3, 1, 3, 3);
        assert_eq!(range, 9..11);
        assert_eq!(&source[range], "gh");
    }

    // C14-13: past-EOF spans must clamp into the buffer, never past it.
    #[test]
    fn line_col_past_eof_clamps_inside_buffer() {
        let source = "abc\ndef\n";
        let range = line_col_to_byte_range(source, 50, 1, 50, 5);
        assert!(range.end <= source.len());
        assert!(!range.is_empty());
        assert_eq!(&source[range.clone()], source);
    }

    // C14-13: multibyte columns are byte columns — a column landing inside
    // a multibyte char must not split it (range stays on char boundaries).
    #[test]
    fn line_col_multibyte_never_splits_char() {
        let source = "héllo wörld\n";
        let range = line_col_to_byte_range(source, 1, 1, 1, 3);
        assert!(
            source.get(range.clone()).is_some(),
            "range {range:?} splits a char"
        );
    }

    // C14-13: spanless diagnostics anchor to the first source (sorted),
    // regardless of HashMap iteration order.
    #[test]
    fn spanless_diagnostics_anchor_deterministically() {
        let mut sources = HashMap::new();
        sources.insert("zzz.spec".to_string(), "content z\n".to_string());
        sources.insert("aaa.spec".to_string(), "content a\n".to_string());
        let diag = Diagnostic::untyped("W001", Severity::Warning, "spanless".to_string());
        let out1 = render_diagnostics(std::slice::from_ref(&diag), &sources);
        let out2 = render_diagnostics(&[diag], &sources);
        assert_eq!(out1, out2);
        assert!(
            out1.contains("aaa.spec"),
            "anchor must be the first sorted source"
        );
    }

    // C14-13: rendering a span whose lines exceed EOF must not panic.
    #[test]
    fn render_past_eof_span_does_not_panic() {
        let mut sources = HashMap::new();
        sources.insert("t.spec".to_string(), "abc\n".to_string());
        let diag = Diagnostic::new(codes::E001, "beyond eof".to_string()).with_span(
            specforge_common::SourceSpan {
                file: "t.spec".into(),
                start_line: 999,
                start_col: 1,
                end_line: 999,
                end_col: 10,
            },
        );
        let out = render_diagnostics(&[diag], &sources);
        assert!(out.contains("beyond eof"));
    }
}
