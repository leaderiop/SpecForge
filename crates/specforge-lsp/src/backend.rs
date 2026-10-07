use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use tokio::sync::{Mutex, RwLock};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use specforge_project::{CheckMode, ProjectSession, SourceChange, UpdateKind};

use crate::document::LineIndex;
use crate::navigation::Compiled;
use crate::publish::{Publication, diagnostic_to_lsp};
use crate::uri::{file_path_to_uri, uri_to_file_path};
use crate::{ClientSupport, LspState, answers, server_capabilities, server_info};

use specforge_ops::format;

pub struct Backend {
    client: Client,
    state: Arc<RwLock<LspState>>,
    /// The project root: rootUri, else the first workspace folder.
    root_dir: Arc<Mutex<Option<String>>>,
    /// Latest-wins reparse requests (C4-03): keystrokes send here; one
    /// serialized worker coalesces and processes, so at most one
    /// whole-graph pass runs at a time and the state lock is never held
    /// across a keystroke storm.
    update_tx: mpsc::UnboundedSender<Url>,
    /// Held by every change to the project session (edits, files changed
    /// on disk, extension reloads, opening the project), so changes apply
    /// one at a time and none is lost to another.
    updates: Arc<Mutex<()>>,
    /// Whether the client declared `workspace.semanticTokens.refreshSupport`
    /// at initialize: only then is it sent `workspace/semanticTokens/refresh`.
    tokens_refresh_support: Arc<AtomicBool>,
    /// The file watchers the client was asked to register
    /// ([`crate::watchers`]), so a reload that changes them re-registers.
    watched: Arc<Mutex<Vec<FileSystemWatcher>>>,
    /// Whether the client declared
    /// `workspace.didChangeWatchedFiles.relativePatternSupport`.
    relative_patterns: Arc<AtomicBool>,
}

/// A change the project session is asked to apply.
enum Change {
    /// Open the project at this root, then apply every open buffer.
    Open(PathBuf),
    /// An open document's buffer changed: it is the truth for its file.
    Buffer(Url),
    /// Files changed, were created or deleted on disk (absolute paths). The
    /// session says what they are, once it is held for the update
    /// (`ProjectSession::changes`), and applies what they amount to: an
    /// environment reload (then every open buffer again), an update of the
    /// changed sources, or a re-check.
    Apply(Vec<PathBuf>),
}

/// What [`Backend::recompile`] did to the session.
struct Recompiled {
    /// The environment was loaded again (or the project opened).
    environment: bool,
    /// The session's inputs changed (`Update::inputs_changed`): what the
    /// client is asked to watch must follow them.
    inputs_changed: bool,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let state = Arc::new(RwLock::new(LspState::new()));
        let (update_tx, mut update_rx) = mpsc::unbounded_channel::<Url>();
        let updates = Arc::new(Mutex::new(()));
        let tokens_refresh_support = Arc::new(AtomicBool::new(false));
        let watched = Arc::new(Mutex::new(Vec::new()));
        let relative_patterns = Arc::new(AtomicBool::new(false));

        // Serialized latest-wins reparse worker (C4-03). Exits when the
        // Backend (and its sender) is dropped.
        let worker_state = Arc::clone(&state);
        let worker_client = client.clone();
        let worker_updates = Arc::clone(&updates);
        let worker_refresh_support = Arc::clone(&tokens_refresh_support);
        let worker_watched = Arc::clone(&watched);
        let worker_relative_patterns = Arc::clone(&relative_patterns);
        tokio::spawn(async move {
            while let Some(first) = update_rx.recv().await {
                // Coalesce everything already queued, then hold off until
                // the stream is quiet for DEBOUNCE_WINDOW.
                let mut pending = vec![first];
                while let Ok(Some(next)) =
                    tokio::time::timeout(crate::DEBOUNCE_WINDOW, update_rx.recv()).await
                {
                    pending.push(next);
                }
                pending.sort();
                pending.dedup();
                for uri in pending {
                    let recompiled = Self::recompile(
                        &worker_state,
                        &worker_client,
                        &worker_updates,
                        Change::Buffer(uri),
                    )
                    .await;
                    // An edit that names a file the checks read moves what
                    // the client must watch (ADR 0030).
                    if recompiled.is_some_and(|r| r.inputs_changed) {
                        Self::sync_watchers(
                            &worker_state,
                            &worker_client,
                            &worker_watched,
                            worker_relative_patterns.load(Ordering::Relaxed),
                        )
                        .await;
                    }
                }
                Self::refresh_semantic_tokens_if_stale(
                    &worker_state,
                    &worker_client,
                    &worker_refresh_support,
                )
                .await;
            }
        });

        Self {
            client,
            state,
            root_dir: Arc::new(Mutex::new(None)),
            update_tx,
            updates,
            tokens_refresh_support,
            watched,
            relative_patterns,
        }
    }

    /// After a recompile: when the graph changed in anything semantic
    /// tokens depend on (entity IDs, kinds, titles, the kind registry's
    /// classification), ask a client that declared refreshSupport to
    /// re-request tokens. The LSP does not subscribe to watch deltas; this
    /// is how open editors learn their highlighting went stale. The request
    /// is sent from its own task so a slow client never stalls a recompile.
    async fn refresh_semantic_tokens_if_stale(
        state: &RwLock<LspState>,
        client: &Client,
        refresh_support: &AtomicBool,
    ) {
        let stale = state.write().await.record_token_signature();
        if stale && refresh_support.load(Ordering::Relaxed) {
            let client = client.clone();
            tokio::spawn(async move {
                let _ = client.semantic_tokens_refresh().await;
            });
        }
    }

    /// Apply `change` to the project session, the one `specforge watch`
    /// holds, and publish everything the project reports now: the
    /// diagnostics `specforge check` reports for the same sources and
    /// buffers. Returns `None` when there was nothing to apply, or it could
    /// not be applied.
    ///
    /// Changes apply one at a time (`updates`). The session does
    /// synchronous file reads and whole-graph checks, so it runs on the
    /// blocking pool: it is taken out of the state (a brief write lock),
    /// updated without any lock held, and put back. Meanwhile readers see
    /// its last complete graph and environment (C4-05).
    async fn recompile(
        state: &RwLock<LspState>,
        client: &Client,
        updates: &Mutex<()>,
        change: Change,
    ) -> Option<Recompiled> {
        let _one_at_a_time = updates.lock().await;
        let (session, buffers, edited, changes) = {
            let mut st = state.write().await;
            let session = st.take_session()?;
            // What changed files are, to the session as it is now (a
            // reload queued before this one may have changed the answer).
            let changes = match &change {
                Change::Apply(paths) => {
                    Some(session.inputs().changes(paths.iter().map(PathBuf::as_path)))
                }
                _ => None,
            };
            if changes
                .as_ref()
                .is_some_and(specforge_project::Changes::is_empty)
            {
                st.set_session(session);
                return None;
            }
            let reload = changes.as_ref().is_some_and(|c| c.environment);
            // Every open buffer, as (absolute path, text), for a change
            // that rebuilds from disk; the edited one for a buffer change.
            let buffer = |uri: &str| {
                let doc = st.document(uri)?;
                let url = Url::parse(uri).ok()?;
                Some((uri_to_file_path(&url), doc.text().to_string()))
            };
            let (buffers, edited): (Vec<(String, String)>, Option<Url>) = match &change {
                Change::Buffer(uri) => (
                    buffer(uri.as_str()).into_iter().collect(),
                    Some(uri.clone()),
                ),
                Change::Apply(_) if !reload => (Vec::new(), None),
                Change::Open(_) | Change::Apply(_) => (
                    st.open_uris().into_iter().filter_map(buffer).collect(),
                    None,
                ),
            };
            (session, buffers, edited, changes)
        };
        if matches!(change, Change::Buffer(_)) && buffers.is_empty() {
            // Closed before the worker got to it.
            state.write().await.set_session(session);
            return None;
        }

        // Opening a project loads its environment first and shows it to
        // readers before the sources are read: the kinds and fields
        // keyword completion offers need no `.spec` file, so they are
        // answered while indexing runs (CONTEXT: Environment).
        let mut opening = None;
        if let Change::Open(root) = &change {
            let root = root.clone();
            match tokio::task::spawn_blocking(move || ProjectSession::begin_open(&root)).await {
                Ok(loaded) => {
                    state
                        .write()
                        .await
                        .show_environment(Arc::clone(loaded.environment()));
                    opening = Some(loaded);
                }
                Err(e) => {
                    Self::lose_session(state, client, e).await;
                    return None;
                }
            }
        }

        let joined = tokio::task::spawn_blocking(move || {
            let mut session = session;
            let mut touched: Vec<String> = Vec::new();
            let mut environment = false;
            let mut inputs_changed = false;
            // Every session verifies its updates in a debug build (ADR
            // 0032): a rebuild that differs from a cold build is reported
            // below.
            let mut divergences: Vec<String> = Vec::new();
            match (&change, changes) {
                (Change::Open(_), _) => {
                    if let Some(loaded) = opening {
                        session = loaded.finish();
                    }
                    environment = true;
                    inputs_changed = true;
                }
                (Change::Apply(_), Some(changes)) => {
                    if let Some(update) = session.apply(&changes) {
                        divergences.extend(update.verification.and_then(std::result::Result::err));
                        environment = update.kind == UpdateKind::Environment;
                        inputs_changed |= update.inputs_changed;
                        touched.extend(update.rebuilt_files);
                    }
                    touched.extend(changes.sources);
                }
                _ => {}
            }
            let typing = matches!(change, Change::Buffer(_));
            for (path, text) in &buffers {
                let key = session.source_key(std::path::Path::new(path));
                let mode = if typing {
                    // The syntax-only fast path (C4-07): no checks while
                    // the edited file does not parse.
                    CheckMode::SyntaxOnlyIfParseErrorsIn(&key)
                } else {
                    CheckMode::Full
                };
                let buffer = SourceChange::Buffer {
                    path: &key,
                    text: Some(text),
                };
                let update = session.update_with(buffer, mode);
                divergences.extend(update.verification.and_then(std::result::Result::err));
                inputs_changed |= update.inputs_changed;
                touched.extend(update.rebuilt_files);
                touched.push(key);
            }
            (session, touched, environment, inputs_changed, divergences)
        })
        .await;

        match joined {
            Ok((session, touched, environment, inputs_changed, divergences)) => {
                for divergence in &divergences {
                    client
                        .log_message(
                            MessageType::ERROR,
                            format!(
                                "an incremental rebuild diverged from a cold build: {divergence}"
                            ),
                        )
                        .await;
                }
                debug_assert!(divergences.is_empty(), "{divergences:?}");
                let touched: Vec<Url> = {
                    let mut st = state.write().await;
                    st.set_session(session);
                    touched
                        .iter()
                        .map(|key| file_path_to_uri(&st.file_path(key).to_string_lossy()))
                        .collect()
                };
                Self::publish(state, client, edited, touched).await;
                Some(Recompiled {
                    environment,
                    inputs_changed,
                })
            }
            Err(e) => {
                Self::lose_session(state, client, e).await;
                None
            }
        }
    }

    /// An update panicked: the session is lost, so the state falls back to
    /// an empty one rather than a stale stand-in.
    async fn lose_session(
        state: &RwLock<LspState>,
        client: &Client,
        error: tokio::task::JoinError,
    ) {
        state.write().await.set_session(ProjectSession::detached());
        client
            .log_message(
                MessageType::ERROR,
                format!("specforge-lsp: recompile failed: {error}"),
            )
            .await;
    }

    /// Ask the client to watch every file the project is built from
    /// ([`crate::watchers::file_watchers`]), replacing the watchers it was
    /// asked for before when they differ: after the project opens, and
    /// after a reload that changed its inputs. A client that refuses the
    /// project's watchers keeps the static ones.
    async fn sync_watchers(
        state: &RwLock<LspState>,
        client: &Client,
        watched: &Mutex<Vec<FileSystemWatcher>>,
        relative_patterns: bool,
    ) {
        let wanted = {
            let st = state.read().await;
            match st.session() {
                Some(session) => {
                    crate::watchers::file_watchers(session.inputs(), relative_patterns)
                }
                None => return,
            }
        };
        let mut watched = watched.lock().await;
        if *watched == wanted {
            return;
        }
        let _ = client
            .unregister_capability(vec![Unregistration {
                id: crate::watchers::REGISTRATION_ID.into(),
                method: "workspace/didChangeWatchedFiles".into(),
            }])
            .await;
        *watched = match Self::register_watchers(client, wanted.clone()).await {
            Ok(()) => wanted,
            Err(_) => {
                let defaults = crate::watchers::default_watchers();
                let _ = Self::register_watchers(client, defaults.clone()).await;
                defaults
            }
        };
    }

    /// Register `watchers` for `workspace/didChangeWatchedFiles`.
    async fn register_watchers(client: &Client, watchers: Vec<FileSystemWatcher>) -> Result<()> {
        client
            .register_capability(vec![Registration {
                id: crate::watchers::REGISTRATION_ID.into(),
                method: "workspace/didChangeWatchedFiles".into(),
                register_options: Some(
                    serde_json::to_value(DidChangeWatchedFilesRegistrationOptions { watchers })
                        .expect("watcher options serialize"),
                ),
            }])
            .await
    }

    /// Publish what the project reports now ([`Publication::of`]): each
    /// diagnostic on the file its span names, a spanless one about entities
    /// at the first one's name, one about none on `edited` (else the anchor,
    /// else the first open document); files that had diagnostics and have
    /// none now, and every `touched` file, get an empty list. What is
    /// published is kept: code actions act on it.
    async fn publish(
        state: &RwLock<LspState>,
        client: &Client,
        edited: Option<Url>,
        touched: Vec<Url>,
    ) {
        let publication = Publication::of(&*state.read().await, edited.as_ref(), &touched);
        state.write().await.record(&publication);
        for (uri, file) in publication.files {
            client
                .publish_diagnostics(uri, file.diagnostics, file.version)
                .await;
        }
    }
}

/// Formatter edits (0-based lines, byte columns of the formatted
/// document) as LSP edits.
fn formatter_edits_to_lsp(
    edits: Vec<specforge_formatter::TextEdit>,
    index: &LineIndex,
) -> Vec<TextEdit> {
    edits
        .into_iter()
        .map(|e| TextEdit {
            range: Range {
                start: index.position_at(e.start_line, e.start_col),
                end: index.position_at(e.end_line, e.end_col),
            },
            new_text: e.new_text,
        })
        .collect()
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let refresh_support = params
            .capabilities
            .workspace
            .as_ref()
            .and_then(|w| w.semantic_tokens.as_ref())
            .and_then(|t| t.refresh_support)
            .unwrap_or(false);
        self.tokens_refresh_support
            .store(refresh_support, Ordering::Relaxed);
        let relative_patterns = params
            .capabilities
            .workspace
            .as_ref()
            .and_then(|w| w.did_change_watched_files.as_ref())
            .and_then(|w| w.relative_pattern_support)
            .unwrap_or(false);
        self.relative_patterns
            .store(relative_patterns, Ordering::Relaxed);
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
        // Until the project is open, watch every .spec, config and lock
        // file; once it is, the watchers cover exactly what it is built
        // from (`sync_watchers`).
        let defaults = crate::watchers::default_watchers();
        if Self::register_watchers(&self.client, defaults.clone())
            .await
            .is_ok()
        {
            *self.watched.lock().await = defaults;
        }

        // Opening the project (extensions, then every .spec file under the
        // spec root) runs in a background task with workDone progress
        // (C4-04): `initialized` returns immediately so the session stays
        // responsive. Edits that arrive meanwhile queue behind it.
        let root = self.root_dir.lock().await.clone();
        let client = self.client.clone();
        let state = Arc::clone(&self.state);
        let updates = Arc::clone(&self.updates);
        let refresh_support = Arc::clone(&self.tokens_refresh_support);
        let watched = Arc::clone(&self.watched);
        let relative_patterns = self.relative_patterns.load(Ordering::Relaxed);
        tokio::spawn(async move {
            let token = NumberOrString::String("specforge-index".into());
            let _ = client
                .send_request::<tower_lsp::lsp_types::request::WorkDoneProgressCreate>(
                    WorkDoneProgressCreateParams {
                        token: token.clone(),
                    },
                )
                .await;
            client
                .send_notification::<tower_lsp::lsp_types::notification::Progress>(ProgressParams {
                    token: token.clone(),
                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(
                        WorkDoneProgressBegin {
                            title: "specforge: indexing workspace".into(),
                            cancellable: None,
                            message: None,
                            percentage: None,
                        },
                    )),
                })
                .await;

            let end = |message: Option<String>| ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                    message,
                })),
            };
            let Some(root) = root else {
                client
                    .log_message(MessageType::INFO, "specforge-lsp initialized (no root_uri)")
                    .await;
                client
                    .send_notification::<tower_lsp::lsp_types::notification::Progress>(end(None))
                    .await;
                return;
            };

            let opened = Self::recompile(
                &state,
                &client,
                &updates,
                Change::Open(PathBuf::from(&root)),
            )
            .await
            .is_some();
            let (ext_count, kind_count, file_count, spec_root) = {
                let st = state.read().await;
                (
                    st.registries().declarations().len(),
                    st.kind_registry().len(),
                    st.session().map_or(0, ProjectSession::file_count),
                    st.spec_root().to_string_lossy().into_owned(),
                )
            };
            if opened && ext_count > 0 {
                // The lsp_initialized announcement: its payload is the
                // extension and entity kind counts.
                client
                    .log_message(
                        MessageType::INFO,
                        format!(
                            "specforge-lsp: loaded {ext_count} extension(s), \
                             {kind_count} entity kind(s)"
                        ),
                    )
                    .await;
            }
            client
                .log_message(
                    MessageType::INFO,
                    format!("specforge-lsp: indexed {file_count} .spec files from {spec_root}"),
                )
                .await;
            Self::refresh_semantic_tokens_if_stale(&state, &client, &refresh_support).await;

            client
                .send_notification::<tower_lsp::lsp_types::notification::Progress>(end(Some(
                    format!("{file_count} files"),
                )))
                .await;
            // The client now watches what the project is built from.
            Self::sync_watchers(&state, &client, &watched, relative_patterns).await;
        });
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

        let recompiled = Self::recompile(
            &self.state,
            &self.client,
            &self.updates,
            Change::Buffer(uri),
        )
        .await;
        if recompiled.is_some_and(|r| r.inputs_changed) {
            Self::sync_watchers(
                &self.state,
                &self.client,
                &self.watched,
                self.relative_patterns.load(Ordering::Relaxed),
            )
            .await;
        }
        Self::refresh_semantic_tokens_if_stale(
            &self.state,
            &self.client,
            &self.tokens_refresh_support,
        )
        .await;
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
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        // An open document's buffer is the truth for its file, so of its
        // changes on disk only its deletion counts. What the others are (a
        // source, an environment or check input, nothing) is the session's
        // to say (classify_project_changes).
        let paths: Vec<PathBuf> = {
            let state = self.state.read().await;
            params
                .changes
                .iter()
                .filter(|change| {
                    change.typ == FileChangeType::DELETED || !state.is_open(change.uri.as_str())
                })
                .map(|change| PathBuf::from(uri_to_file_path(&change.uri)))
                .collect()
        };
        let recompiled = if paths.is_empty() {
            None
        } else {
            Self::recompile(
                &self.state,
                &self.client,
                &self.updates,
                Change::Apply(paths),
            )
            .await
        };
        let (environment, inputs_changed) =
            recompiled.map_or((false, false), |r| (r.environment, r.inputs_changed));
        if environment {
            // The environment loaded again (hardening-plan H4 / R-5): the
            // spec root re-indexed, everything republished, and the
            // watchers follow what the project is now built from.
            let ext_count = self.state.read().await.registries().declarations().len();
            self.client
                .log_message(
                    MessageType::INFO,
                    format!(
                        "specforge-lsp: extension environment changed, reloaded {ext_count} extension(s)"
                    ),
                )
                .await;
        }
        // The watchers follow what the project is now built from: after a
        // reload, and after any update that changed its inputs.
        if environment || inputs_changed {
            Self::sync_watchers(
                &self.state,
                &self.client,
                &self.watched,
                self.relative_patterns.load(Ordering::Relaxed),
            )
            .await;
        }
        // One check for the whole batch: an extension reload (new kind
        // classifications), a deletion or an on-disk edit may all have
        // changed what open editors highlight.
        Self::refresh_semantic_tokens_if_stale(
            &self.state,
            &self.client,
            &self.tokens_refresh_support,
        )
        .await;
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
    /// (ADR 0021), and publish what formatting reported alongside the
    /// compile's diagnostics. Inside a project the project's configuration
    /// wins over `options`; the editor is told so once per configuration.
    async fn format(
        &self,
        uri: &Url,
        options: &FormattingOptions,
        lines: Option<format::Lines>,
    ) -> Result<Option<Vec<TextEdit>>> {
        let editor = format::EditorOptions {
            tab_size: options.tab_size as usize,
            insert_spaces: options.insert_spaces,
        };
        let (edits, notice) =
            {
                let state = self.state.read().await;
                let Some(doc) = state.document(uri.as_str()) else {
                    return Ok(None);
                };
                let file = uri.to_file_path().ok();
                let place = file
                    .as_deref()
                    .map_or(format::Place::Detached, format::Place::File);
                let formatted = format::document(place, doc.text(), lines, Some(editor));
                if !formatted.diagnostics.is_empty() {
                    // A publish replaces the document's list: the formatter's
                    // diagnostics go alongside the compile ones, not in their
                    // place.
                    // The compile's are positions in the text it compiled, the
                    // formatter's in the document it formatted.
                    let ranges = Compiled::new(&state);
                    let lsp_diags: Vec<Diagnostic> =
                        state
                            .diagnostics(uri.as_str())
                            .iter()
                            .map(|d| diagnostic_to_lsp(d, |span| ranges.range(span)))
                            .chain(formatted.diagnostics.iter().map(|d| {
                                diagnostic_to_lsp(d, |span| Some(doc.index().range(span)))
                            }))
                            .collect();
                    self.client
                        .publish_diagnostics(uri.clone(), lsp_diags, doc.version())
                        .await;
                }
                let notice = overridden_editor_options(&formatted, editor);
                (
                    formatter_edits_to_lsp(formatted.edits(), doc.index()),
                    notice,
                )
            };
        if let Some((configuration, message)) = notice
            && self.state.write().await.first_format_notice(&configuration)
        {
            self.client.log_message(MessageType::INFO, message).await;
        }
        Ok(Some(edits))
    }
}

/// When a project's configuration formatted `formatted` and the editor's
/// `editor` settings differ from it: the configuration (its key for the
/// once-per-session notice) and the message telling the editor so.
fn overridden_editor_options(
    formatted: &format::FormattedDocument,
    editor: format::EditorOptions,
) -> Option<(String, String)> {
    let configuration = match &formatted.config_source {
        format::ConfigSource::File(path) => path.display().to_string(),
        format::ConfigSource::Defaults => "the defaults".to_string(),
        format::ConfigSource::Editor => return None,
    };
    let config = &formatted.config;
    let same = editor.insert_spaces != config.use_tabs
        && (config.use_tabs || editor.tab_size == config.indent_width);
    if same {
        return None;
    }
    let indent = if config.use_tabs { "tabs" } else { "spaces" };
    let message = format!(
        "formatting with {configuration} (indent {}, {indent}); the editor's tabSize {} / insertSpaces {} apply only outside a project",
        config.indent_width, editor.tab_size, editor.insert_spaces
    );
    Some((configuration, message))
}
