//! The LSP's adapter over `specforge_ops::navigate`: it builds the
//! navigator over the session (open buffers first, then disk) and converts
//! its answers, `SourceSpan`s, to LSP locations and UTF-16 ranges. What
//! the answers are is navigation's to say (ADR 0016); `use`-path
//! navigation is the LSP's own (MCP has no import tool).

use specforge_common::{SourceSpan, Sym};
use specforge_ops::navigate::Navigator;
use specforge_resolver::{ResolveConfig, resolve_import};
use std::path::Path;
use tower_lsp::lsp_types::{Location, Position, Range, Url};

use crate::LspState;
use crate::backend::file_path_to_uri;
use crate::document::utf16_col_to_byte_offset;

/// The navigator over the session: open buffers first, then disk.
pub fn navigator(state: &LspState) -> Navigator<'_, impl Fn(&str) -> Option<String> + '_> {
    Navigator::new(state.view(), move |file| file_content(state, file))
}

/// The text of a session file: its open buffer, else the file on disk.
pub(crate) fn file_content(state: &LspState, key: &str) -> Option<String> {
    let path = state.file_path(key);
    let uri = file_path_to_uri(&path.to_string_lossy());
    if let Some(doc) = state.document(uri.as_str()) {
        return Some(doc.content().to_string());
    }
    std::fs::read_to_string(path).ok()
}

/// The URI of a session file key.
pub(crate) fn uri_of(state: &LspState, key: &str) -> Url {
    file_path_to_uri(&state.file_path(key).to_string_lossy())
}

/// The location of a span of a session file.
pub(crate) fn location(state: &LspState, span: &SourceSpan) -> Location {
    Location {
        uri: uri_of(state, span.file.as_str()),
        range: range(state, span),
    }
}

/// The LSP range of a span: byte columns convert to UTF-16 against the
/// file's own text, so non-ASCII prefixes cannot shift editor ranges; a
/// file that cannot be read keeps its byte columns.
pub(crate) fn range(state: &LspState, span: &SourceSpan) -> Range {
    let lsp = match file_content(state, span.file.as_str()) {
        Some(content) => crate::source_span_to_lsp_range_with_text(span, &content),
        None => crate::source_span_to_lsp_range(span),
    };
    Range {
        start: Position {
            line: lsp.start_line,
            character: lsp.start_col,
        },
        end: Position {
            line: lsp.end_line,
            character: lsp.end_col,
        },
    }
}

/// An LSP position (0-based line, UTF-16 column) in `content` as
/// navigation's (1-based line, 1-based byte column); `None` past the end
/// of the document.
pub(crate) fn byte_position(content: &str, position: Position) -> Option<(usize, usize)> {
    let line = content.split('\n').nth(position.line as usize)?;
    if position.character as usize > line.chars().map(char::len_utf16).sum::<usize>() {
        return None;
    }
    let col = utf16_col_to_byte_offset(line, position.character as usize);
    Some((position.line as usize + 1, col + 1))
}

/// The file a `use` import path in `importing_file` (relative to
/// `spec_root`) names, resolved as the compile resolves it (relative,
/// `@alias`, bare, `index.spec`, never above the spec root): its first
/// line, keyed relative to the spec root. `None` when it names no file.
pub fn goto_import_definition(
    import_path: &str,
    importing_file: &str,
    spec_root: &Path,
    config: &ResolveConfig,
) -> Option<SourceSpan> {
    let target = resolve_import(spec_root, importing_file, import_path, config)?;
    Some(SourceSpan {
        file: Sym::new(&target),
        start_line: 0,
        start_col: 0,
        end_line: 0,
        end_col: 0,
    })
}
