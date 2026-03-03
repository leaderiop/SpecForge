use specforge_common::SourceSpan;
use tower_lsp::lsp_types::{Position, Range, Url};

/// Convert a SpecForge SourceSpan (1-indexed) to an LSP Range (0-indexed).
pub fn span_to_range(span: &SourceSpan) -> Range {
    Range {
        start: Position {
            line: span.start_line.saturating_sub(1),
            character: span.start_col.saturating_sub(1),
        },
        end: Position {
            line: span.end_line.saturating_sub(1),
            character: span.end_col.saturating_sub(1),
        },
    }
}

/// Convert a SpecForge SourceSpan to an LSP Location.
pub fn span_to_location(span: &SourceSpan) -> Option<tower_lsp::lsp_types::Location> {
    let uri = file_path_to_uri(&span.file)?;
    Some(tower_lsp::lsp_types::Location {
        uri,
        range: span_to_range(span),
    })
}

/// Convert a file path string to a Url.
pub fn file_path_to_uri(path: &str) -> Option<Url> {
    Url::from_file_path(path).ok()
}

/// Convert an LSP Url to a file path string.
pub fn uri_to_file_path(uri: &Url) -> Option<String> {
    uri.to_file_path().ok().map(|p| p.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_to_range_converts_1_indexed_to_0_indexed() {
        let span = SourceSpan::new("test.spec", 1, 1, 1, 10);
        let range = span_to_range(&span);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 9);
    }

    #[test]
    fn span_to_range_multiline() {
        let span = SourceSpan::new("test.spec", 5, 3, 10, 20);
        let range = span_to_range(&span);
        assert_eq!(range.start.line, 4);
        assert_eq!(range.start.character, 2);
        assert_eq!(range.end.line, 9);
        assert_eq!(range.end.character, 19);
    }

    #[test]
    fn span_to_range_saturates_at_zero() {
        let span = SourceSpan::new("test.spec", 0, 0, 0, 0);
        let range = span_to_range(&span);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
    }
}
