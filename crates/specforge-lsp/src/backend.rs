use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::sync::{mpsc, watch};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use specforge_project::RuntimeSource;
use specforge_watch::Debouncer;

use crate::changes::Change;
use crate::editor::ClientEditor;
use crate::reaction::Reaction;
use crate::{ClientSupport, LspState, answers, server_capabilities, server_info};

use specforge_ops::format;

/// The reaction, behind the queue every change waits in (ADR 0043 D4).
type Shared = Arc<Mutex<Reaction<ClientEditor>>>;

pub struct Backend {
    /// Formatting publishes and logs on the request path (ADR 0043 D8).
    client: Client,
    state: Arc<RwLock<LspState>>,
    /// The project root: rootUri, else the first workspace folder.
    root_dir: Arc<Mutex<Option<String>>>,
    /// Latest-wins reparse requests (C4-03): keystrokes send here; one
    /// serialized worker coalesces and processes, so at most one
    /// whole-graph pass runs at a time and the state lock is never held
    /// across a keystroke storm.
    update_tx: mpsc::UnboundedSender<Url>,
    /// What every change to the project session is reacted to by: applied,
    /// published, the editor's watchers followed, its highlighting
    /// refreshed (ADR 0035, ADR 0043).
    reaction: Shared,
    /// Dropped with the backend: the editor's calls give up once the server is gone, so a
    /// request the editor never answers cannot hold the reaction's thread (and the runtime's
    /// shutdown) for good.
    _gone: watch::Sender<()>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let state = Arc::new(RwLock::new(LspState::new()));
        let (gone, closed) = watch::channel(());
        let reaction: Shared = Arc::new(Mutex::new(Reaction::new(
            ClientEditor::new(client.clone(), closed),
            Arc::clone(&state),
            RuntimeSource::project(),
        )));
        let (update_tx, mut update_rx) = mpsc::unbounded_channel::<Url>();

        // Serialized latest-wins reparse worker (C4-03). Exits when the
        // Backend (and its sender) is dropped.
        let worker = Arc::clone(&reaction);
        tokio::spawn(async move {
            // The rule `specforge watch` batches file changes by: the burst
            // is quiet for the debounce window, each document once.
            let debouncer = Debouncer::new(specforge_watch::DEFAULT_DEBOUNCE_WINDOW);
            while let Some(pending) = debouncer.coalesce_async(&mut update_rx).await {
                // Everything the burst edited is one update (ADR 0023 D9).
                react(&worker, Change::Edited(pending)).await;
            }
        });

        Self {
            client,
            state,
            root_dir: Arc::new(Mutex::new(None)),
            update_tx,
            reaction,
            _gone: gone,
        }
    }
}

/// Run `change` through the reaction on the blocking pool, after every change queued before it.
async fn react(reaction: &Shared, change: Change) {
    let mut reaction = Arc::clone(reaction).lock_owned().await;
    // The reaction reports an update's panic itself; a panic here (a debug build's divergence
    // assertion) has left the state whole, and the next change goes on.
    let _ = tokio::task::spawn_blocking(move || {
        reaction.react(change);
    })
    .await;
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        self.state
            .write()
            .await
            .set_client(ClientSupport::of(&params.capabilities));
        let root = params
            .root_uri
            .as_ref()
            .and_then(|u| u.to_file_path().ok())
            .map(|p| p.to_string_lossy().to_string())
            .or_else(|| {
                params
                    .workspace_folders
                    .as_ref()
                    .and_then(|folders| folders.first())
                    .and_then(|f| f.uri.to_file_path().ok())
                    .map(|p| p.to_string_lossy().to_string())
            });
        // The project is opened at this root once initialized: its
        // specforge.json names the spec root and the extensions, read the
        // way `specforge check` reads them.
        *self.root_dir.lock().await = root;
        let state = self.state.read().await;
        let kind_keywords: Vec<String> = state.kind_registry().keywords().cloned().collect();
        let kind_refs: Vec<&str> = kind_keywords.iter().map(|s| s.as_str()).collect();

        drop(state);
        let caps = server_capabilities(&kind_refs);
        let token_types: Vec<SemanticTokenType> = crate::TOKEN_TYPES
            .iter()
            .map(|t| SemanticTokenType::new(t))
            .collect();

        let info = server_info();
        Ok(InitializeResult {
            server_info: Some(tower_lsp::lsp_types::ServerInfo {
                name: info.name,
                version: Some(info.version),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(caps.supports_hover)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(caps.completion_trigger_characters.clone()),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(caps.supports_go_to_definition)),
                references_provider: Some(OneOf::Left(caps.supports_find_references)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                code_action_provider: Some(CodeActionProviderCapability::Simple(
                    caps.supports_code_actions,
                )),
                document_symbol_provider: Some(OneOf::Left(caps.supports_document_symbols)),
                workspace_symbol_provider: Some(OneOf::Left(caps.supports_workspace_symbols)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types,
                                token_modifiers: crate::TOKEN_MODIFIERS
                                    .iter()
                                    .map(|m| SemanticTokenModifier::new(m))
                                    .collect(),
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            ..Default::default()
                        },
                    ),
                ),
                document_formatting_provider: Some(OneOf::Left(caps.supports_document_formatting)),
                document_range_formatting_provider: Some(OneOf::Left(
                    caps.supports_document_range_formatting,
                )),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        let root = self.root_dir.lock().await.clone().map(PathBuf::from);
        // Taken before returning, so every change the client reports next waits for the
        // project to open; the open itself runs in the background with workDone progress
        // (C4-04): `initialized` returns immediately so the session stays responsive.
        let mut reaction = Arc::clone(&self.reaction).lock_owned().await;
        tokio::task::spawn_blocking(move || reaction.open(root));
    }

    async fn shutdown(&self) -> Result<()> {
        self.state.write().await.shutdown();
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;

        {
            let mut state = self.state.write().await;
            state.open_document(uri.as_str(), &text);
            if let Some(doc) = state.document_mut(uri.as_str()) {
                doc.set_version(version);
            }
        }

        react(&self.reaction, Change::Edited(vec![uri])).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        // Apply text changes immediately (keeps buffer current for completions/hover)
        {
            let mut state = self.state.write().await;
            for change in &params.content_changes {
                state.apply_change(uri.as_str(), change.range, &change.text);
            }
            if let Some(doc) = state.document_mut(uri.as_str()) {
                doc.set_version(params.text_document.version);
            }
        }

        // Hand off to the serialized latest-wins worker (C4-03): the burst
        // is coalesced and one whole-graph pass runs at a time.
        let _ = self.update_tx.send(uri);
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.state.write().await.close_document(uri.as_str());
        // The editor keeps a closed document's squiggles until told
        // otherwise: publish an empty set to clear them.
        self.client
            .publish_diagnostics(uri.clone(), Vec::new(), None)
            .await;
        // The buffer is no longer the truth for its file (ADR 0023 D9).
        react(&self.reaction, Change::Closed(uri)).await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        // One reaction for the whole batch: an extension reload (new kind
        // classifications), a deletion or an on-disk edit may all have
        // changed what open editors highlight.
        react(&self.reaction, Change::Watched(params.changes)).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let at = params.text_document_position_params;
        Ok(answers::hover(
            &*self.state.read().await,
            &at.text_document.uri,
            at.position,
        ))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let at = params.text_document_position;
        Ok(answers::completion(
            &*self.state.read().await,
            &at.text_document.uri,
            at.position,
        ))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let at = params.text_document_position_params;
        Ok(answers::definition(
            &*self.state.read().await,
            &at.text_document.uri,
            at.position,
        ))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let at = params.text_document_position;
        Ok(answers::references(
            &*self.state.read().await,
            &at.text_document.uri,
            at.position,
            params.context.include_declaration,
        ))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        answers::prepare_rename(
            &*self.state.read().await,
            &params.text_document.uri,
            params.position,
        )
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let at = params.text_document_position;
        answers::rename(
            &*self.state.read().await,
            &at.text_document.uri,
            at.position,
            &params.new_name,
        )
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        Ok(answers::code_actions(
            &*self.state.read().await,
            &params.text_document.uri,
            params.range,
        ))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        Ok(answers::document_symbols(
            &*self.state.read().await,
            &params.text_document.uri,
        ))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        Ok(answers::workspace_symbols(
            &*self.state.read().await,
            &params.query,
        ))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        Ok(answers::semantic_tokens(
            &*self.state.read().await,
            &params.text_document.uri,
        ))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        self.format(&params.text_document.uri, &params.options, None)
            .await
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let lines = format::Lines {
            first: params.range.start.line as usize,
            last: params.range.end.line as usize,
        };
        self.format(&params.text_document.uri, &params.options, Some(lines))
            .await
    }
}

impl Backend {
    /// Format an open document as `specforge format` formats its file
    /// (ADR 0021): what [`answers::formatting`] answers, published beside
    /// the compile's diagnostics, and the editor told once per
    /// configuration that the project's wins over `options`.
    async fn format(
        &self,
        uri: &Url,
        options: &FormattingOptions,
        lines: Option<format::Lines>,
    ) -> Result<Option<Vec<TextEdit>>> {
        let formatted = answers::formatting(&*self.state.read().await, uri, options, lines);
        let Some(formatted) = formatted else {
            return Ok(None);
        };
        if let Some((diagnostics, version)) = formatted.publish {
            self.client
                .publish_diagnostics(uri.clone(), diagnostics, version)
                .await;
        }
        if let Some((configuration, message)) = formatted.notice
            && self.state.write().await.first_format_notice(&configuration)
        {
            self.client.log_message(MessageType::INFO, message).await;
        }
        Ok(Some(formatted.edits))
    }
}
