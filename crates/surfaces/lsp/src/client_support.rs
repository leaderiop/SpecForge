//! What the client declared at initialize that shapes an answer or a reaction ([`ClientSupport`]).

use tower_lsp::lsp_types::ClientCapabilities;

/// What the client declared at initialize that shapes an answer or a
/// reaction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClientSupport {
    /// `textDocument.definition.linkSupport`: a definition is a
    /// `LocationLink` (the block, its name selected), else a `Location` at
    /// the name.
    pub definition_links: bool,
    /// `textDocument.documentSymbol.hierarchicalDocumentSymbolSupport`: the
    /// outline nests.
    pub hierarchical_symbols: bool,
    /// `textDocument.completion.completionItem.insertReplaceSupport`: a
    /// completion item's edit inserts over the word's start to the cursor
    /// and replaces the whole word, else it is a plain edit.
    pub insert_replace: bool,
    /// `workspace.semanticTokens.refreshSupport`: only then is the editor
    /// asked to refresh its semantic tokens.
    pub tokens_refresh: bool,
    /// `workspace.codeLens.refreshSupport`: only then is the editor asked to refresh its code
    /// lenses after a rebuild.
    pub lens_refresh: bool,
    /// `workspace.didChangeWatchedFiles.relativePatternSupport`: the
    /// watchers are patterns relative to each input's directory, else
    /// absolute globs.
    pub relative_patterns: bool,
    /// `textDocument.codeAction.resolveSupport` names `edit`: a stub's edits
    /// are made when the editor resolves the action, not when it lists them.
    pub code_action_resolve: bool,
    /// `experimental.specforgeGraphChanged`: only then is the editor sent `specforge/graphChanged`
    /// after a rebuild (ADR 0268 D2), a method no other client knows.
    pub graph_changed: bool,
    /// `workspace.workspaceEdit.documentChanges`: an edit names the version of the buffer it applies to
    /// (`documentChanges`), else it is a bare `changes` map (ADR 0289).
    pub document_changes: bool,
}

impl ClientSupport {
    /// What `capabilities` declare of these.
    pub fn of(capabilities: &ClientCapabilities) -> ClientSupport {
        let text_document = capabilities.text_document.as_ref();
        let workspace = capabilities.workspace.as_ref();
        ClientSupport {
            definition_links: text_document
                .and_then(|t| t.definition.as_ref())
                .and_then(|d| d.link_support)
                .unwrap_or(false),
            hierarchical_symbols: text_document
                .and_then(|t| t.document_symbol.as_ref())
                .and_then(|d| d.hierarchical_document_symbol_support)
                .unwrap_or(false),
            insert_replace: text_document
                .and_then(|t| t.completion.as_ref())
                .and_then(|c| c.completion_item.as_ref())
                .and_then(|i| i.insert_replace_support)
                .unwrap_or(false),
            tokens_refresh: workspace
                .and_then(|w| w.semantic_tokens.as_ref())
                .and_then(|t| t.refresh_support)
                .unwrap_or(false),
            lens_refresh: workspace
                .and_then(|w| w.code_lens.as_ref())
                .and_then(|l| l.refresh_support)
                .unwrap_or(false),
            relative_patterns: workspace
                .and_then(|w| w.did_change_watched_files.as_ref())
                .and_then(|w| w.relative_pattern_support)
                .unwrap_or(false),
            code_action_resolve: text_document
                .and_then(|t| t.code_action.as_ref())
                .and_then(|c| c.resolve_support.as_ref())
                .is_some_and(|r| r.properties.iter().any(|p| p == "edit")),
            document_changes: workspace
                .and_then(|w| w.workspace_edit.as_ref())
                .and_then(|e| e.document_changes)
                .unwrap_or(false),
            graph_changed: capabilities
                .experimental
                .as_ref()
                .and_then(|e| e.get(crate::views::GRAPH_CHANGED_CAPABILITY))
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        }
    }
}
