//! The LSP's adapter over `specforge_ops::navigate`: it builds the
//! navigator over the session (open buffers first, then disk) and converts
//! its answers, `SourceSpan`s, to LSP locations and UTF-16 ranges. What
//! the answers are is navigation's to say (ADR 0016); `use`-path
//! navigation is the LSP's own (MCP has no import tool).

use specforge_common::{SourceSpan, Sym};
use specforge_ops::navigate::{Fix, FixKind, Navigator, OutlineEntry};
use specforge_registry::KindRegistry;
use specforge_resolver::{ResolveConfig, resolve_import};
use std::collections::HashMap;
use std::path::Path;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, DocumentSymbol, DocumentSymbolResponse, Location, Position, Range,
    SymbolInformation, SymbolKind, TextEdit, Url, WorkspaceEdit,
};

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

/// An LSP range of `content` (the document `file`) as a span, its ends
/// clamped to the text: what a code action request's range covers.
pub(crate) fn span_of_range(content: &str, file: &str, range: Range) -> SourceSpan {
    let clamp = |position: Position| {
        let lines: Vec<&str> = content.split('\n').collect();
        let line = (position.line as usize).min(lines.len().saturating_sub(1));
        let text = lines.get(line).copied().unwrap_or("");
        let width: usize = text.chars().map(char::len_utf16).sum();
        let character = if (position.line as usize) < lines.len() {
            (position.character as usize).min(width)
        } else {
            width
        };
        (line + 1, utf16_col_to_byte_offset(text, character) + 1)
    };
    let (start_line, start_col) = clamp(range.start);
    let (end_line, end_col) = clamp(range.end);
    SourceSpan {
        file: Sym::new(file),
        start_line,
        start_col,
        end_line,
        end_col,
    }
}

/// A fix as the code action that applies it.
pub(crate) fn fix_to_code_action(state: &LspState, fix: Fix) -> CodeAction {
    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    for edit in &fix.edits {
        changes
            .entry(uri_of(state, edit.span.file.as_str()))
            .or_default()
            .push(TextEdit {
                range: range(state, &edit.span),
                new_text: edit.new_text.clone(),
            });
    }
    CodeAction {
        title: fix.title,
        kind: Some(match fix.kind {
            FixKind::QuickFix => CodeActionKind::QUICKFIX,
            FixKind::Refactor => CodeActionKind::REFACTOR,
        }),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// An outline as the LSP's document symbols: nested (methods as children,
/// each selecting its name) when `hierarchical`, else flat (a method's
/// container is its entity).
pub(crate) fn outline_to_document_symbols(
    state: &LspState,
    entries: Vec<OutlineEntry>,
    hierarchical: bool,
) -> DocumentSymbolResponse {
    let kinds = state.kind_registry();
    if hierarchical {
        #[allow(deprecated)]
        let symbols = entries
            .into_iter()
            .map(|entry| {
                let kind = entry.kind.as_str();
                let children: Vec<DocumentSymbol> = entry
                    .children
                    .iter()
                    .map(|method| DocumentSymbol {
                        name: method.name.clone(),
                        detail: Some(format!("method {}", method.signature)),
                        kind: SymbolKind::METHOD,
                        tags: None,
                        deprecated: None,
                        range: range(state, &method.block),
                        selection_range: range(state, &method.name_span),
                        children: None,
                    })
                    .collect();
                DocumentSymbol {
                    name: entry.id.to_string(),
                    detail: Some(match &entry.title {
                        Some(title) => format!("{kind} — {title}"),
                        None => kind.to_string(),
                    }),
                    kind: symbol_kind_from_entity(kind, kinds),
                    tags: None,
                    deprecated: None,
                    range: range(state, &entry.block),
                    selection_range: range(state, &entry.name),
                    children: (!children.is_empty()).then_some(children),
                }
            })
            .collect();
        return DocumentSymbolResponse::Nested(symbols);
    }
    let mut symbols = Vec::new();
    for entry in entries {
        #[allow(deprecated)]
        symbols.push(SymbolInformation {
            location: location(state, &entry.block),
            name: entry.id.to_string(),
            kind: symbol_kind_from_entity(entry.kind.as_str(), kinds),
            tags: None,
            deprecated: None,
            container_name: Some(entry.kind.to_string()),
        });
        for method in &entry.children {
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                location: location(state, &method.block),
                name: method.name.clone(),
                kind: SymbolKind::METHOD,
                tags: None,
                deprecated: None,
                container_name: Some(entry.id.to_string()),
            });
        }
    }
    DocumentSymbolResponse::Flat(symbols)
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

/// The symbol kind of an entity kind: its extension-declared LSP icon
/// (`spec` is a namespace).
pub(crate) fn symbol_kind_from_entity(kind: &str, kind_registry: &KindRegistry) -> SymbolKind {
    if kind == "spec" {
        return SymbolKind::NAMESPACE;
    }
    if let Some(entry) = kind_registry.get(kind)
        && let Some(ref icon) = entry.declared.lsp_icon
    {
        return lsp_icon_to_symbol_kind(icon);
    }
    SymbolKind::VARIABLE
}

fn lsp_icon_to_symbol_kind(icon: &str) -> SymbolKind {
    match icon {
        "Method" => SymbolKind::METHOD,
        "Struct" => SymbolKind::STRUCT,
        "Class" => SymbolKind::CLASS,
        "Module" => SymbolKind::MODULE,
        "Constant" => SymbolKind::CONSTANT,
        "Event" => SymbolKind::EVENT,
        "Interface" => SymbolKind::INTERFACE,
        "Property" => SymbolKind::PROPERTY,
        "Variable" => SymbolKind::VARIABLE,
        "Text" => SymbolKind::STRING,
        "Package" => SymbolKind::PACKAGE,
        "Folder" => SymbolKind::NAMESPACE,
        _ => SymbolKind::VARIABLE,
    }
}
