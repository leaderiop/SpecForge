//! The LSP's adapter over `specforge_ops::navigate`: it builds the
//! navigator over the session (open buffers first, then disk) and converts
//! its answers, `SourceSpan`s, to LSP locations and UTF-16 ranges. What
//! the answers are is navigation's to say (ADR 0016); `use`-path
//! navigation is the LSP's own (MCP has no import tool).

use specforge_common::{SourceSpan, Sym};
use specforge_ops::navigate::{Fix, FixKind, Navigator, OutlineEntry};
use specforge_registry::KindRegistry;
use specforge_resolver::resolve_import;
use std::collections::HashMap;
use std::path::Path;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, DocumentSymbol, DocumentSymbolResponse, Location, Range,
    SymbolInformation, SymbolKind, TextEdit, Url, WorkspaceEdit,
};

use crate::LspState;
use crate::backend::file_path_to_uri;
use crate::document::LineIndex;
use std::cell::RefCell;
use std::sync::Arc;

/// The navigator over the session. It reads the text the project was
/// compiled from ([`file_content`]), the text every span it answers is a
/// position in.
pub fn navigator(state: &LspState) -> Navigator<'_, impl Fn(&str) -> Option<String> + '_> {
    Navigator::new(state.view(), move |file| file_content(state, file))
}

/// The text of a session file that navigation reads: the text the project
/// was compiled from, not the buffer now (an edit not yet compiled would
/// move the spans) nor the disk. `None` for a file the compile does not
/// hold.
pub(crate) fn file_content(state: &LspState, key: &str) -> Option<String> {
    state.compiled_text(key).map(|text| text.to_string())
}

/// The URI of a session file key.
pub(crate) fn uri_of(state: &LspState, key: &str) -> Url {
    file_path_to_uri(&state.file_path(key).to_string_lossy())
}

/// Spans of session files as LSP ranges and locations. A span is a
/// position in the text the project was compiled from, so it converts
/// (byte columns to UTF-16) through that text's [`LineIndex`], built once
/// per file per request, or the open document's own index when the
/// document is that text. A file the compile holds no text of cannot be
/// converted: its range is `None`, never byte columns passed off as UTF-16.
pub(crate) struct Ranges<'s> {
    state: &'s LspState,
    indexes: RefCell<HashMap<Sym, Option<Arc<LineIndex>>>>,
    stale: RefCell<HashMap<Sym, bool>>,
}

impl<'s> Ranges<'s> {
    pub(crate) fn new(state: &'s LspState) -> Self {
        Ranges {
            state,
            indexes: RefCell::new(HashMap::new()),
            stale: RefCell::new(HashMap::new()),
        }
    }

    /// The line index of the text session file `file` was compiled from
    /// (once per request); `None` when the compile holds no text of it.
    pub(crate) fn index_of(&self, file: &str) -> Option<Arc<LineIndex>> {
        let key = Sym::new(file);
        if let Some(index) = self.indexes.borrow().get(&key) {
            return index.clone();
        }
        let index = self.state.compiled_text(file).map(|text| {
            let uri = uri_of(self.state, file);
            match self.state.document(uri.as_str()) {
                Some(doc) if doc.text() == &*text => Arc::clone(doc.index()),
                _ => Arc::new(LineIndex::new(&text)),
            }
        });
        self.indexes.borrow_mut().insert(key, index.clone());
        index
    }

    /// The LSP range of a span of the compile, against the text it is a
    /// position in; `None` when that text is unknown.
    pub(crate) fn range(&self, span: &SourceSpan) -> Option<Range> {
        Some(self.index_of(span.file.as_str())?.range(span))
    }

    /// The location of a span of a session file; `None` when its text is
    /// unknown.
    pub(crate) fn location(&self, span: &SourceSpan) -> Option<Location> {
        Some(Location {
            uri: uri_of(self.state, span.file.as_str()),
            range: self.range(span)?,
        })
    }

    /// Whether the text the editor has for session file `file` is not the
    /// text the compile read ([`LspState::is_compiled`]): edits computed
    /// from the compile do not apply to it. Read once per file per request.
    pub(crate) fn is_stale(&self, file: &str) -> bool {
        let key = Sym::new(file);
        *self
            .stale
            .borrow_mut()
            .entry(key)
            .or_insert_with(|| !self.state.is_compiled(file))
    }

    /// The state the spans are read against.
    pub(crate) fn state(&self) -> &'s LspState {
        self.state
    }
}

/// A fix as the code action that applies it; `None` when one of its edits
/// cannot be placed (its file's text is unknown, or the open buffer is no
/// longer the text the fix was computed on): a fix applies whole or not at
/// all.
pub(crate) fn fix_to_code_action(ranges: &Ranges, fix: Fix) -> Option<CodeAction> {
    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    for edit in &fix.edits {
        let file = edit.span.file.as_str();
        if ranges.is_stale(file) {
            return None;
        }
        changes
            .entry(uri_of(ranges.state(), file))
            .or_default()
            .push(TextEdit {
                range: ranges.range(&edit.span)?,
                new_text: edit.new_text.clone(),
            });
    }
    Some(CodeAction {
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
    })
}

/// An outline as the LSP's document symbols: nested (methods as children,
/// each selecting its name) when `hierarchical`, else flat (a method's
/// container is its entity). An entry or method whose file's text is
/// unknown is left out (no range of it is honest).
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
            .filter_map(|entry| {
                let kind = entry.kind.as_str();
                let children: Vec<DocumentSymbol> = entry
                    .children
                    .iter()
                    .filter_map(|method| {
                        Some(DocumentSymbol {
                            name: method.name.clone(),
                            detail: Some(format!("method {}", method.signature)),
                            kind: SymbolKind::METHOD,
                            tags: None,
                            deprecated: None,
                            range: ranges.range(&method.block)?,
                            selection_range: ranges.range(&method.name_span)?,
                            children: None,
                        })
                    })
                    .collect();
                Some(DocumentSymbol {
                    name: entry.id.to_string(),
                    detail: Some(match &entry.title {
                        Some(title) => format!("{kind} — {title}"),
                        None => kind.to_string(),
                    }),
                    kind: symbol_kind_from_entity(kind, kinds),
                    tags: None,
                    deprecated: None,
                    range: ranges.range(&entry.block)?,
                    selection_range: ranges.range(&entry.name)?,
                    children: (!children.is_empty()).then_some(children),
                })
            })
            .collect();
        return DocumentSymbolResponse::Nested(symbols);
    }
    let mut symbols = Vec::new();
    for entry in entries {
        let Some(location) = ranges.location(&entry.block) else {
            continue;
        };
        #[allow(deprecated)]
        symbols.push(SymbolInformation {
            location,
            name: entry.id.to_string(),
            kind: symbol_kind_from_entity(entry.kind.as_str(), kinds),
            tags: None,
            deprecated: None,
            container_name: Some(entry.kind.to_string()),
        });
        for method in &entry.children {
            let Some(location) = ranges.location(&method.block) else {
                continue;
            };
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                location,
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
/// bare, `index.spec`, never above the spec root): its first
/// line, keyed relative to the spec root. `None` when it names no file.
pub fn goto_import_definition(
    import_path: &str,
    importing_file: &str,
    spec_root: &Path,
) -> Option<SourceSpan> {
    let target = resolve_import(spec_root, importing_file, import_path)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_project::ProjectSession;
    use specforge_test_macros::test as spec;

    /// A title with non-ASCII letters: `{` is at byte column 25 and UTF-16
    /// column 21 of the first line.
    const COMPILED: &str = "behavior alpha \"Ééé\" {\n}\n";
    /// The same file as the editor has it after the title was edited: not
    /// yet compiled.
    const TYPED: &str = "behavior alpha \"E\" {\n}\n";

    fn brace() -> SourceSpan {
        SourceSpan {
            file: Sym::new("a.spec"),
            start_line: 1,
            start_col: 25,
            end_line: 1,
            end_col: 26,
        }
    }

    /// A state serving the project at `dir` (one file, `a.spec`, holding
    /// `COMPILED`), and the URI of that file.
    fn serving(dir: &tempfile::TempDir) -> (LspState, String) {
        std::fs::write(
            dir.path().join("specforge.json"),
            r#"{"name":"t","version":"0.1.0","extensions":[]}"#,
        )
        .unwrap();
        std::fs::write(dir.path().join("a.spec"), COMPILED).unwrap();
        let mut state = LspState::new();
        state.set_session(ProjectSession::open(dir.path()));
        let uri = uri_of(&state, "a.spec").to_string();
        (state, uri)
    }

    #[spec(
        invariant = "lsp_utf16_positions",
        verify = "a span converts against the text the project was compiled from, not the buffer typed since"
    )]
    fn a_span_converts_against_the_compiled_text_not_the_stale_buffer() {
        let dir = tempfile::tempdir().unwrap();
        let (mut state, uri) = serving(&dir);

        // The buffer is the compiled text: the span lands on its UTF-16
        // column, through the document's own index.
        state.open_document(&uri, COMPILED);
        let ranges = Ranges::new(&state);
        assert!(!ranges.is_stale("a.spec"));
        let range = ranges.range(&brace()).expect("the compile holds a.spec");
        assert_eq!((range.start.line, range.start.character), (0, 21));
        assert_eq!((range.end.line, range.end.character), (0, 22));

        // The editor typed since, and the compile has not run: the span is
        // still a position in the compiled text, so it keeps its column
        // (against the buffer, byte column 25 is past the line's end).
        state.apply_change(&uri, None, TYPED);
        let ranges = Ranges::new(&state);
        assert!(ranges.is_stale("a.spec"));
        assert_eq!(ranges.range(&brace()), Some(range));
        assert_eq!(
            ranges.location(&brace()).map(|l| l.uri.to_string()),
            Some(uri.clone())
        );
        // What navigation reads is that text too.
        assert_eq!(file_content(&state, "a.spec").as_deref(), Some(COMPILED));
    }

    #[spec(
        invariant = "lsp_utf16_positions",
        verify = "a span of a file the compile holds no text of has no range, never byte columns"
    )]
    fn a_span_of_a_file_without_compiled_text_has_no_range() {
        let dir = tempfile::tempdir().unwrap();
        let (mut state, _) = serving(&dir);

        // On disk and readable, but not part of the compile; and a name
        // the project has never held.
        std::fs::write(dir.path().join("ghost.spec"), COMPILED).unwrap();
        for file in ["ghost.spec", "nope.spec"] {
            let span = SourceSpan {
                file: Sym::new(file),
                ..brace()
            };
            let ranges = Ranges::new(&state);
            assert_eq!(ranges.range(&span), None, "{file}");
            assert_eq!(ranges.location(&span), None, "{file}");
            assert!(ranges.index_of(file).is_none(), "{file}");
        }

        // A file that cannot be read from disk now still has the text it
        // was compiled from: the disk is not asked.
        std::fs::remove_file(dir.path().join("a.spec")).unwrap();
        let ranges = Ranges::new(&state);
        let range = ranges.range(&brace()).expect("compiled text is kept");
        assert_eq!(range.start.character, 21);

        // While the session is out for an update, readers keep its texts.
        let session = state.take_session().expect("held");
        assert_eq!(
            state.compiled_text("a.spec").as_deref(),
            Some(COMPILED),
            "the stand-in holds the compiled text"
        );
        state.set_session(session);
    }

    #[spec(
        invariant = "lsp_utf16_positions",
        verify = "a fix is offered whole or not at all: never over a buffer typed since, nor a file with no compiled text"
    )]
    fn a_fix_is_offered_whole_or_not_at_all() {
        let dir = tempfile::tempdir().unwrap();
        let (mut state, uri) = serving(&dir);
        let fix = |files: &[&str]| Fix {
            title: "fix".into(),
            kind: FixKind::QuickFix,
            source: specforge_ops::navigate::FixSource::ReplaceUnresolved,
            diagnostic_code: None,
            subject: None,
            edits: files
                .iter()
                .map(|file| specforge_ops::navigate::TextEdit {
                    span: SourceSpan {
                        file: Sym::new(file),
                        ..brace()
                    },
                    new_text: "x".into(),
                })
                .collect(),
            anchor: None,
        };

        state.open_document(&uri, COMPILED);
        let ranges = Ranges::new(&state);
        let action = fix_to_code_action(&ranges, fix(&["a.spec"])).expect("placeable");
        let changes = action.edit.unwrap().changes.unwrap();
        assert_eq!(
            changes[&Url::parse(&uri).unwrap()][0].range.start.character,
            21
        );
        // One edit with no text refuses the whole fix.
        assert!(fix_to_code_action(&ranges, fix(&["a.spec", "ghost.spec"])).is_none());

        // The editor typed since: the edits are positions in a text it no
        // longer has.
        state.apply_change(&uri, None, TYPED);
        let ranges = Ranges::new(&state);
        assert!(fix_to_code_action(&ranges, fix(&["a.spec"])).is_none());
    }
}
