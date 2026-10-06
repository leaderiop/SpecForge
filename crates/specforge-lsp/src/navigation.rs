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
use crate::document::LineIndex;
use std::cell::RefCell;
use std::sync::Arc;

/// The navigator over the session: open buffers first, then disk.
pub fn navigator(state: &LspState) -> Navigator<'_, impl Fn(&str) -> Option<String> + '_> {
    Navigator::new(state.view(), move |file| file_content(state, file))
}

/// The text of a session file: its open buffer, else the file on disk.
pub(crate) fn file_content(state: &LspState, key: &str) -> Option<String> {
    let path = state.file_path(key);
    let uri = file_path_to_uri(&path.to_string_lossy());
    if let Some(doc) = state.document(uri.as_str()) {
        return Some(doc.text().to_string());
    }
    std::fs::read_to_string(path).ok()
}

/// The URI of a session file key.
pub(crate) fn uri_of(state: &LspState, key: &str) -> Url {
    file_path_to_uri(&state.file_path(key).to_string_lossy())
}

/// Spans of session files as LSP ranges and locations: each file's line
/// index built once per request, from its open document, else from disk.
/// A file that cannot be read keeps its byte columns (the only place a
/// span is not converted, and the reason is that there is no text).
pub(crate) struct Ranges<'s> {
    state: &'s LspState,
    indexes: RefCell<HashMap<Sym, Option<Arc<LineIndex>>>>,
}

impl<'s> Ranges<'s> {
    pub(crate) fn new(state: &'s LspState) -> Self {
        Ranges {
            state,
            indexes: RefCell::new(HashMap::new()),
        }
    }

    /// The line index of a session file: its open document's, else one
    /// built from the file on disk (once per request); `None` when it
    /// cannot be read.
    pub(crate) fn index_of(&self, file: &str) -> Option<Arc<LineIndex>> {
        let key = Sym::new(file);
        if let Some(index) = self.indexes.borrow().get(&key) {
            return index.clone();
        }
        let path = self.state.file_path(file);
        let uri = file_path_to_uri(&path.to_string_lossy());
        let index = match self.state.document(uri.as_str()) {
            Some(doc) => Some(Arc::clone(doc.index())),
            None => std::fs::read_to_string(path)
                .ok()
                .map(|text| Arc::new(LineIndex::new(&text))),
        };
        self.indexes.borrow_mut().insert(key, index.clone());
        index
    }

    /// The LSP range of a span: byte columns convert to UTF-16 against the
    /// file's own text, so non-ASCII prefixes cannot shift editor ranges;
    /// a file that cannot be read keeps its byte columns.
    pub(crate) fn range(&self, span: &SourceSpan) -> Range {
        match self.index_of(span.file.as_str()) {
            Some(index) => index.range(span),
            None => {
                let at = |line: usize, col: usize| Position {
                    line: line.saturating_sub(1) as u32,
                    character: col.saturating_sub(1) as u32,
                };
                Range {
                    start: at(span.start_line, span.start_col),
                    end: at(span.end_line, span.end_col),
                }
            }
        }
    }

    /// The location of a span of a session file.
    pub(crate) fn location(&self, span: &SourceSpan) -> Location {
        Location {
            uri: uri_of(self.state, span.file.as_str()),
            range: self.range(span),
        }
    }

    /// The state the spans are read against.
    pub(crate) fn state(&self) -> &'s LspState {
        self.state
    }
}

/// A fix as the code action that applies it.
pub(crate) fn fix_to_code_action(ranges: &Ranges, fix: Fix) -> CodeAction {
    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    for edit in &fix.edits {
        changes
            .entry(uri_of(ranges.state(), edit.span.file.as_str()))
            .or_default()
            .push(TextEdit {
                range: ranges.range(&edit.span),
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
    ranges: &Ranges,
    entries: Vec<OutlineEntry>,
    hierarchical: bool,
) -> DocumentSymbolResponse {
    let kinds = ranges.state().kind_registry();
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
                        range: ranges.range(&method.block),
                        selection_range: ranges.range(&method.name_span),
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
                    range: ranges.range(&entry.block),
                    selection_range: ranges.range(&entry.name),
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
            location: ranges.location(&entry.block),
            name: entry.id.to_string(),
            kind: symbol_kind_from_entity(entry.kind.as_str(), kinds),
            tags: None,
            deprecated: None,
            container_name: Some(entry.kind.to_string()),
        });
        for method in &entry.children {
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                location: ranges.location(&method.block),
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
