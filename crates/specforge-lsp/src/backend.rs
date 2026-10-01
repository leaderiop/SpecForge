use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;
use tokio::sync::{Mutex, RwLock};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use specforge_project::{CheckMode, ProjectSession, SourceChange};
use specforge_registry::KindRegistry;

use crate::{
    LspState, classify_tokens, code_action_create_stub, code_actions_from_diagnostics,
    code_actions_missing_verify, complete_entity_ids, complete_entity_ids_filtered,
    complete_keywords, cursor_context, document_symbols, find_all_references, go_to_definition,
    goto_import_definition, hover_field_info, hover_info_with_registries, server_capabilities,
    server_info, source_span_to_lsp_range, source_span_to_lsp_range_with_text, workspace_symbols,
};

use crate::formatting::{EditorOptions, format_document, format_document_range};

use crate::document::utf16_col_to_byte_offset;
use crate::{byte_col_to_utf16, utf16_len};

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
}

/// A change the project session is asked to apply.
enum Change {
    /// Open the project at this root, then apply every open buffer.
    Open(PathBuf),
    /// An open document's buffer changed: it is the truth for its file.
    Buffer(Url),
    /// Files changed, were created or deleted on disk (absolute paths).
    Disk(Vec<String>),
    /// `specforge.json` or an extension changed: load the environment
    /// again, rebuild from disk, then apply every open buffer.
    Reload,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let state = Arc::new(RwLock::new(LspState::new()));
        let (update_tx, mut update_rx) = mpsc::unbounded_channel::<Url>();
        let updates = Arc::new(Mutex::new(()));
        let tokens_refresh_support = Arc::new(AtomicBool::new(false));

        // Serialized latest-wins reparse worker (C4-03). Exits when the
        // Backend (and its sender) is dropped.
        let worker_state = Arc::clone(&state);
        let worker_client = client.clone();
        let worker_updates = Arc::clone(&updates);
        let worker_refresh_support = Arc::clone(&tokens_refresh_support);
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
                    Self::recompile(
                        &worker_state,
                        &worker_client,
                        &worker_updates,
                        Change::Buffer(uri),
                    )
                    .await;
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
    /// buffers. Returns false when the change could not be applied.
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
    ) -> bool {
        let _one_at_a_time = updates.lock().await;
        let (session, buffers, edited) = {
            let mut st = state.write().await;
            let Some(session) = st.take_session() else {
                return false;
            };
            // Every open buffer, as (absolute path, text), for a change
            // that rebuilds from disk; the edited one for a buffer change.
            let buffer = |uri: &str| {
                let doc = st.document(uri)?;
                let url = Url::parse(uri).ok()?;
                Some((uri_to_file_path(&url), doc.content().to_string()))
            };
            let (buffers, edited): (Vec<(String, String)>, Option<Url>) = match &change {
                Change::Buffer(uri) => (
                    buffer(uri.as_str()).into_iter().collect(),
                    Some(uri.clone()),
                ),
                Change::Disk(_) => (Vec::new(), None),
                Change::Open(_) | Change::Reload => (
                    st.open_uris().into_iter().filter_map(buffer).collect(),
                    None,
                ),
            };
            (session, buffers, edited)
        };
        if matches!(change, Change::Buffer(_)) && buffers.is_empty() {
            // Closed before the worker got to it.
            state.write().await.set_session(session);
            return false;
        }

        let joined = tokio::task::spawn_blocking(move || {
            let mut session = session;
            let mut touched: Vec<String> = Vec::new();
            match &change {
                Change::Open(root) => session = ProjectSession::open(root),
                Change::Reload => touched.extend(session.reload_environment().rebuilt_files),
                Change::Disk(paths) => {
                    let spec_root = session.environment().spec_root.clone();
                    let keys: Vec<String> = paths
                        .iter()
                        .map(|p| crate::state::file_key(&spec_root, p))
                        .collect();
                    touched.extend(session.update(SourceChange::Disk(&keys)).rebuilt_files);
                    touched.extend(keys);
                }
                Change::Buffer(_) => {}
            }
            let spec_root = session.environment().spec_root.clone();
            let typing = matches!(change, Change::Buffer(_));
            for (path, text) in &buffers {
                let key = crate::state::file_key(&spec_root, path);
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
                touched.extend(session.update_with(buffer, mode).rebuilt_files);
                touched.push(key);
            }
            (session, touched)
        })
        .await;

        match joined {
            Ok((session, touched)) => {
                let touched: Vec<Url> = {
                    let mut st = state.write().await;
                    st.set_session(session);
                    touched
                        .iter()
                        .map(|key| file_path_to_uri(&st.file_path(key).to_string_lossy()))
                        .collect()
                };
                Self::publish(state, client, edited, touched).await;
                true
            }
            Err(e) => {
                // The update panicked: the session is lost, so the state
                // falls back to an empty one rather than a stale stand-in.
                state.write().await.set_session(ProjectSession::detached());
                client
                    .log_message(
                        MessageType::ERROR,
                        format!("specforge-lsp: recompile failed: {e}"),
                    )
                    .await;
                false
            }
        }
    }

    /// Publish what the project reports now, each diagnostic on the file
    /// its span names. One without a span goes on `edited`, else on the
    /// document the last one went on while it is open, else on the first
    /// open document. Files that had diagnostics and have none now, and
    /// every `touched` file, are published too (an empty list clears
    /// them).
    async fn publish(
        state: &RwLock<LspState>,
        client: &Client,
        edited: Option<Url>,
        touched: Vec<Url>,
    ) {
        let mut published: HashMap<Url, Vec<Diagnostic>> = HashMap::new();
        let mut core: HashMap<Url, Vec<specforge_common::Diagnostic>> = HashMap::new();
        let mut targets: BTreeSet<Url> = touched.into_iter().collect();
        let versions: HashMap<Url, Option<i32>>;
        {
            let st = state.read().await;
            let anchor = edited
                .clone()
                .or_else(|| st.anchor().and_then(|uri| Url::parse(uri).ok()))
                .or_else(|| st.open_uris().first().and_then(|uri| Url::parse(uri).ok()));
            let diagnostics = st.session().map(|s| s.diagnostics()).unwrap_or_default();
            for diagnostic in &diagnostics {
                let uri = match &diagnostic.span {
                    Some(span) => uri_of(&st, span.file.as_str()),
                    None => match &anchor {
                        Some(anchor) => anchor.clone(),
                        None => continue,
                    },
                };
                let text = diagnostic
                    .span
                    .as_ref()
                    .and_then(|s| file_content(&st, s.file.as_str()));
                published
                    .entry(uri.clone())
                    .or_default()
                    .push(diagnostic_to_lsp(diagnostic, text.as_deref()));
                core.entry(uri).or_default().push(diagnostic.clone());
            }
            targets.extend(published.keys().cloned());
            targets.extend(
                st.published_uris()
                    .iter()
                    .filter_map(|uri| Url::parse(uri).ok()),
            );
            targets.extend(edited.clone());
            versions = targets
                .iter()
                .map(|uri| {
                    let version = st.document(uri.as_str()).and_then(|d| d.version());
                    (uri.clone(), version)
                })
                .collect();
        }
        {
            // Keep what is published: code actions act on it.
            let mut st = state.write().await;
            if let Some(edited) = &edited {
                st.set_anchor(Some(edited.to_string()));
            }
            for uri in &targets {
                match core.remove(uri) {
                    Some(diagnostics) => st.set_diagnostics(uri.as_str(), diagnostics),
                    None => st.clear_diagnostics(uri.as_str()),
                }
            }
        }
        for uri in targets {
            let diagnostics = published.remove(&uri).unwrap_or_default();
            let version = versions.get(&uri).copied().flatten();
            client.publish_diagnostics(uri, diagnostics, version).await;
        }
    }
}

/// The URI of a session file key.
fn uri_of(state: &LspState, key: &str) -> Url {
    file_path_to_uri(&state.file_path(key).to_string_lossy())
}

/// The session file key of a document.
fn key_of(state: &LspState, uri: &Url) -> String {
    state.file_key(&uri_to_file_path(uri))
}

/// The location of a span of a session file.
fn location_of(state: &LspState, span: &specforge_common::SourceSpan) -> Location {
    Location {
        uri: uri_of(state, span.file.as_str()),
        range: span_range_for(state, span),
    }
}

pub fn source_span_to_location(span: &specforge_common::SourceSpan) -> Location {
    Location {
        uri: file_path_to_uri(span.file.as_str()),
        range: source_span_to_range(span),
    }
}

pub fn source_span_to_range(span: &specforge_common::SourceSpan) -> Range {
    let lsp = source_span_to_lsp_range(span);
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

/// Resolve the text of a session file: open buffer first, then disk.
fn file_content(state: &LspState, key: &str) -> Option<String> {
    let path = state.file_path(key);
    let uri = file_path_to_uri(&path.to_string_lossy());
    if let Some(doc) = state.document(uri.as_str()) {
        return Some(doc.content().to_string());
    }
    std::fs::read_to_string(path).ok()
}

/// C3-09: text-aware range — byte columns convert to UTF-16 using the
/// file's own text, so non-ASCII prefixes cannot shift editor ranges.
fn span_range_for(state: &LspState, span: &specforge_common::SourceSpan) -> Range {
    match file_content(state, span.file.as_str()) {
        Some(content) => {
            let lsp = source_span_to_lsp_range_with_text(span, &content);
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
        None => source_span_to_range(span),
    }
}

pub fn file_path_to_uri(path: &str) -> Url {
    Url::from_file_path(path).unwrap_or_else(|_| {
        Url::parse(&format!("file://{path}")).unwrap_or_else(|_| Url::parse("file:///").unwrap())
    })
}

pub fn uri_to_file_path(uri: &Url) -> String {
    uri.to_file_path()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| uri.to_string())
}

/// docs/diagnostics.md as built into this binary: the anchors a docs link
/// may point at.
const DIAGNOSTICS_DOC: &str = include_str!("../../../docs/diagnostics.md");

/// The page on the canonical repository (the workspace's Cargo
/// `repository`, ADR 0004 D6-b) that documents every catalogued code.
const DIAGNOSTICS_DOC_URL: &str = concat!(
    env!("CARGO_PKG_REPOSITORY"),
    "/blob/main/docs/diagnostics.md"
);

/// The docs link for `code`, or `None` when docs/diagnostics.md has no
/// section for it (a third-party code, or anything outside the catalog).
fn docs_href(code: &str) -> Option<Url> {
    let heading = format!("## {code}");
    DIAGNOSTICS_DOC
        .lines()
        .any(|line| line == heading)
        .then(|| Url::parse(&format!("{DIAGNOSTICS_DOC_URL}#{}", code.to_lowercase())).ok())
        .flatten()
}

fn diagnostic_to_lsp(diag: &specforge_common::Diagnostic, content: Option<&str>) -> Diagnostic {
    let range = diag
        .span
        .as_ref()
        .map(|span| match content {
            Some(text) => {
                let lsp = source_span_to_lsp_range_with_text(span, text);
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
            None => source_span_to_range(span),
        })
        .unwrap_or_default();
    Diagnostic {
        range,
        code: Some(NumberOrString::String(diag.code.clone())),
        // C4-10: editors can render this as a "view docs" link; the target
        // page is generated from the `specforge explain` catalog.
        code_description: docs_href(&diag.code).map(|href| CodeDescription { href }),
        severity: Some(match diag.severity {
            specforge_common::Severity::Error => DiagnosticSeverity::ERROR,
            specforge_common::Severity::Warning => DiagnosticSeverity::WARNING,
            specforge_common::Severity::Info => DiagnosticSeverity::INFORMATION,
        }),
        source: Some("specforge".into()),
        // C4-08: the suggestion is the actionable half of the diagnostic
        // ("did you mean X / do Y") — surface it in the editor instead of
        // dropping it at the LSP boundary.
        message: match &diag.suggestion {
            Some(suggestion) => format!("{}\n\nsuggestion: {suggestion}", diag.message),
            None => diag.message.clone(),
        },
        ..Default::default()
    }
}

fn symbol_kind_from_entity(kind: &str, kind_registry: &KindRegistry) -> SymbolKind {
    if kind == "spec" {
        return SymbolKind::NAMESPACE;
    }
    if let Some(entry) = kind_registry.get(kind)
        && let Some(ref icon) = entry.lsp_icon
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

/// Extract the word at a given cursor position from document content.
pub fn word_at_position(content: &str, line: usize, col: usize) -> Option<String> {
    let target_line = content.lines().nth(line)?;
    // `col` arrives as UTF-16 code units (LSP `character`); convert it to a
    // byte offset within the line before scanning.
    if col > target_line.chars().map(char::len_utf16).sum::<usize>() {
        return None;
    }
    let col = utf16_col_to_byte_offset(target_line, col);
    let bytes = target_line.as_bytes();
    let is_id_char = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut start = col;
    while start > 0 && is_id_char(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < bytes.len() && is_id_char(bytes[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    Some(target_line[start..end].to_string())
}

/// If the line is a `use` import statement, returns the import path portion.
/// Handles all three forms:
///   use "path"
///   use { ... } from "path"
///   use * as x from "path"
/// Also handles `pub use` variants.
pub fn import_path_on_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    // Strip pub prefix if present
    let rest = trimmed
        .strip_prefix("pub use ")
        .or_else(|| trimmed.strip_prefix("use "))?;
    // Extract the quoted path — it's always the last "..." on the line
    let last_quote_end = rest.rfind('"')?;
    let before_last = &rest[..last_quote_end];
    let last_quote_start = before_last.rfind('"')?;
    let path = &rest[last_quote_start + 1..last_quote_end];
    if path.is_empty() { None } else { Some(path) }
}

fn formatter_edits_to_lsp(
    edits: Vec<specforge_formatter::TextEdit>,
    source: &str,
) -> Vec<TextEdit> {
    // Formatter edit columns are byte offsets into `source`; LSP expects
    // UTF-16 code units. Convert per line using the formatted document text.
    let line_texts: Vec<&str> = source.lines().collect();
    let utf16 = |line: usize, byte_col: usize| -> u32 {
        line_texts
            .get(line)
            .map(|l| byte_col_to_utf16(l, byte_col) as u32)
            .unwrap_or(0)
    };
    edits
        .into_iter()
        .map(|e| TextEdit {
            range: Range {
                start: Position {
                    line: e.start_line as u32,
                    character: utf16(e.start_line, e.start_col),
                },
                end: Position {
                    line: e.end_line as u32,
                    character: utf16(e.end_line, e.end_col),
                },
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
        // Register file watchers for *.spec files so external changes are detected
        let _ = self
            .client
            .register_capability(vec![Registration {
                id: "specforge-file-watcher".into(),
                method: "workspace/didChangeWatchedFiles".into(),
                register_options: Some(
                    serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                        watchers: vec![
                            FileSystemWatcher {
                                glob_pattern: GlobPattern::String("**/*.spec".into()),
                                kind: Some(WatchKind::all()),
                            },
                            FileSystemWatcher {
                                glob_pattern: GlobPattern::String("**/specforge.json".into()),
                                kind: Some(WatchKind::all()),
                            },
                            FileSystemWatcher {
                                glob_pattern: GlobPattern::String("**/*.wasm".into()),
                                kind: Some(WatchKind::all()),
                            },
                        ],
                    })
                    .unwrap(),
                ),
            }])
            .await;

        // Opening the project (extensions, then every .spec file under the
        // spec root) runs in a background task with workDone progress
        // (C4-04): `initialized` returns immediately so the session stays
        // responsive. Edits that arrive meanwhile queue behind it.
        let root = self.root_dir.lock().await.clone();
        let client = self.client.clone();
        let state = Arc::clone(&self.state);
        let updates = Arc::clone(&self.updates);
        let refresh_support = Arc::clone(&self.tokens_refresh_support);
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
            .await;
            let (ext_count, kind_count, file_count, spec_root) = {
                let st = state.read().await;
                (
                    st.registries().manifests.len(),
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

        Self::recompile(
            &self.state,
            &self.client,
            &self.updates,
            Change::Buffer(uri),
        )
        .await;
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
                if let Some(range) = change.range {
                    state.apply_change(
                        uri.as_str(),
                        range.start.line as usize,
                        range.start.character as usize,
                        range.end.line as usize,
                        range.end.character as usize,
                        &change.text,
                    );
                } else {
                    state.close_document(uri.as_str());
                    state.open_document(uri.as_str(), &change.text);
                }
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
        // Extension configuration or plugin artifact changed: the
        // environment is loaded again, the spec root re-indexed, and
        // everything republished (hardening-plan H4 / R-5).
        let reload = params.changes.iter().any(|change| {
            let path = uri_to_file_path(&change.uri);
            path.ends_with("specforge.json") || path.ends_with(".wasm")
        });
        if reload {
            Self::recompile(&self.state, &self.client, &self.updates, Change::Reload).await;
            let ext_count = self.state.read().await.registries().manifests.len();
            self.client
                .log_message(
                    MessageType::INFO,
                    format!(
                        "specforge-lsp: extension environment changed, reloaded {ext_count} extension(s)"
                    ),
                )
                .await;
        } else {
            // .spec files changed on disk. An open document's buffer is
            // the truth for its file, so only its deletion counts.
            let paths: Vec<String> = {
                let state = self.state.read().await;
                params
                    .changes
                    .iter()
                    .filter(|change| uri_to_file_path(&change.uri).ends_with(".spec"))
                    .filter(|change| {
                        change.typ == FileChangeType::DELETED || !state.is_open(change.uri.as_str())
                    })
                    .map(|change| uri_to_file_path(&change.uri))
                    .collect()
            };
            if !paths.is_empty() {
                Self::recompile(
                    &self.state,
                    &self.client,
                    &self.updates,
                    Change::Disk(paths),
                )
                .await;
            }
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
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let state = self.state.read().await;
        let content = match state.document(uri.as_str()) {
            Some(doc) => doc.content().to_string(),
            None => return Ok(None),
        };

        let word = match word_at_position(&content, pos.line as usize, pos.character as usize) {
            Some(w) => w,
            None => return Ok(None),
        };

        let kind_reg = state.kind_registry();
        let field_reg = state.field_registry();
        let kr = if kind_reg.is_empty() {
            None
        } else {
            Some(kind_reg)
        };
        let fr = if field_reg.is_empty() {
            None
        } else {
            Some(field_reg)
        };
        let info = hover_info_with_registries(state.graph(), &word, kr, fr).or_else(|| {
            // Fallback: try field hover if word is not an entity ID
            if !field_reg.is_empty() {
                let entity_kind =
                    crate::completion::enclosing_entity_kind(&content, pos.line as usize)?;
                hover_field_info(&word, &entity_kind, field_reg)
            } else {
                None
            }
        });
        Ok(info.map(|md| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: md,
            }),
            range: None,
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let state = self.state.read().await;
        let content = match state.document(uri.as_str()) {
            Some(doc) => doc.content().to_string(),
            None => return Ok(None),
        };

        let prefix = word_at_position(&content, pos.line as usize, pos.character as usize)
            .unwrap_or_default();

        let mut items: Vec<CompletionItem> = Vec::new();

        // Detect cursor context: if inside a reference list, filter by target_kind
        let ctx = cursor_context(&content, pos.line as usize, pos.character as usize);
        let target_kind: Option<String> = ctx.as_ref().and_then(|c| {
            let field_reg = state.field_registry();
            field_reg
                .get(&c.entity_kind, &c.field_name)
                .and_then(|entry| entry.target_kind.clone())
        });

        // Outside a reference list the enclosing block decides: its own
        // body takes field names, the top level takes keywords.
        let block = if ctx.is_some() {
            None
        } else {
            crate::completion::enclosing_block(&content, pos.line as usize, pos.character as usize)
        };
        let lower_prefix = prefix.to_lowercase();
        if let Some((kind, 1)) = &block {
            let mut fields = state.field_registry().fields_for_kind(kind);
            fields.sort_by(|a, b| a.field_name.cmp(&b.field_name));
            for field in fields {
                if !field.field_name.to_lowercase().starts_with(&lower_prefix) {
                    continue;
                }
                items.push(CompletionItem {
                    label: field.field_name.clone(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: field.description.clone(),
                    insert_text: Some(crate::completion::field_snippet(field, 1)),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
                });
            }
            return Ok(Some(CompletionResponse::Array(items)));
        }

        if block.is_some() || ctx.is_some() {
            let entity_items = if let Some(ref tk) = target_kind {
                complete_entity_ids_filtered(state.graph(), &prefix, Some(tk))
            } else {
                complete_entity_ids(state.graph(), &prefix)
            };
            for (rank, item) in entity_items.into_iter().enumerate() {
                let detail = item
                    .title
                    .as_ref()
                    .map(|t| format!("{} — {}", item.kind, t))
                    .unwrap_or_else(|| item.kind.clone());
                items.push(CompletionItem {
                    label: item.id.clone(),
                    kind: Some(CompletionItemKind::REFERENCE),
                    detail: Some(detail),
                    // C4-06: preserve the server's fuzzy ranking in the editor.
                    sort_text: Some(format!("{rank:04}")),
                    ..Default::default()
                });
            }
            return Ok(Some(CompletionResponse::Array(items)));
        }

        // Top level: structural keywords and every registered kind, each
        // kind scaffolding its required fields.
        let kind_reg = state.kind_registry();
        let dynamic_kinds: Vec<String> = kind_reg.keywords().cloned().collect();
        let kind_refs: Vec<&str> = dynamic_kinds.iter().map(|s| s.as_str()).collect();
        for kw in complete_keywords(&kind_refs) {
            if !(prefix.is_empty() || kw.to_lowercase().starts_with(&lower_prefix)) {
                continue;
            }
            let (detail, snippet) = match kind_reg.get(&kw) {
                Some(entry) => (
                    Some(entry.source_extension.clone()),
                    Some(crate::completion::keyword_snippet(
                        &kw,
                        state.field_registry(),
                    )),
                ),
                None => (None, None),
            };
            items.push(CompletionItem {
                label: kw,
                kind: Some(CompletionItemKind::KEYWORD),
                detail,
                insert_text_format: snippet.as_ref().map(|_| InsertTextFormat::SNIPPET),
                insert_text: snippet,
                ..Default::default()
            });
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let state = self.state.read().await;
        let content = match state.document(uri.as_str()) {
            Some(doc) => doc.content().to_string(),
            None => return Ok(None),
        };

        if let Some(import_path) = content
            .lines()
            .nth(pos.line as usize)
            .and_then(import_path_on_line)
        {
            let spec_root = state.spec_root().to_string_lossy().into_owned();
            if !spec_root.is_empty() {
                let span = goto_import_definition(import_path, &spec_root);
                return Ok(span.map(|s| GotoDefinitionResponse::Scalar(location_of(&state, &s))));
            }
        }

        let word = match word_at_position(&content, pos.line as usize, pos.character as usize) {
            Some(w) => w,
            None => return Ok(None),
        };

        let span = go_to_definition(state.graph(), &word);
        Ok(span.map(|s| GotoDefinitionResponse::Scalar(location_of(&state, &s))))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;

        let state = self.state.read().await;
        let content = match state.document(uri.as_str()) {
            Some(doc) => doc.content().to_string(),
            None => return Ok(None),
        };
        let word = match word_at_position(&content, pos.line as usize, pos.character as usize) {
            Some(w) => w,
            None => return Ok(None),
        };

        let refs = find_all_references(state.graph(), &word);
        if refs.is_empty() {
            return Ok(None);
        }
        Ok(Some(refs.iter().map(|s| location_of(&state, s)).collect()))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let pos = params.position;

        let state = self.state.read().await;
        let content = match state.document(uri.as_str()) {
            Some(doc) => doc.content().to_string(),
            None => return Ok(None),
        };

        let word = match word_at_position(&content, pos.line as usize, pos.character as usize) {
            Some(w) => w,
            None => return Ok(None),
        };

        let span = crate::prepare_rename(state.graph(), &word);
        Ok(span.map(|s| {
            let lsp = source_span_to_lsp_range_with_text(&s, &content);
            PrepareRenameResponse::Range(Range {
                start: Position {
                    line: lsp.start_line,
                    character: lsp.start_col,
                },
                end: Position {
                    line: lsp.end_line,
                    character: lsp.end_col,
                },
            })
        }))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let new_name = params.new_name;

        let state = self.state.read().await;
        let content = match state.document(uri.as_str()) {
            Some(doc) => doc.content().to_string(),
            None => return Ok(None),
        };

        let word = match word_at_position(&content, pos.line as usize, pos.character as usize) {
            Some(w) => w,
            None => return Ok(None),
        };

        // Each whole-word occurrence inside the declaration and the
        // entities that reference it, read from the open buffer, else disk.
        // A site whose file cannot be read would be left behind: the
        // rename is all or nothing, so it is refused instead.
        let unreadable = std::cell::Cell::new(false);
        let edits = match specforge_graph::rename::identifier_edits(
            state.graph(),
            &word,
            &new_name,
            |file| {
                let text = file_content(&state, file);
                unreadable.set(unreadable.get() || text.is_none());
                text
            },
        ) {
            Some(e) if !unreadable.get() => e,
            _ => return Ok(None),
        };

        let mut changes: std::collections::HashMap<Url, Vec<TextEdit>> =
            std::collections::HashMap::new();
        for edit in edits {
            let file_uri = uri_of(&state, &edit.file);
            let line_idx = edit.line.saturating_sub(1); // 1-indexed -> 0-indexed
            let line_text = file_content(&state, &edit.file)
                .and_then(|text| text.lines().nth(line_idx).map(str::to_string))
                .unwrap_or_default();
            let start = byte_col_to_utf16(&line_text, edit.start_col) as u32;
            let end = byte_col_to_utf16(&line_text, edit.end_col) as u32;
            changes.entry(file_uri).or_default().push(TextEdit {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: start,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: end,
                    },
                },
                new_text: new_name.clone(),
            });
        }

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let state = self.state.read().await;
        let file_path = key_of(&state, &uri);
        let content = file_content(&state, &file_path);

        let mut actions =
            code_actions_missing_verify(state.graph(), &file_path, state.kind_registry());

        // C4-09: E003/E025 diagnostics with a did-you-mean suggestion
        // become one-tap rename quickfixes.
        let file_diags = state.diagnostics(uri.as_str()).to_vec();
        if let Some(text) = &content {
            actions.extend(code_actions_from_diagnostics(&file_diags, text));
        }

        // An E003 for an id that exists nowhere: offer a stub of the kind
        // the enclosing field targets (FieldRegistry target_kind).
        let mut stubbed = std::collections::HashSet::new();
        for diag in file_diags.iter().filter(|d| d.code == "E003") {
            // "unresolved reference '<target>' in entity '<source>'"
            let mut quoted = diag.message.split('\'');
            let (Some(target), Some(source)) = (quoted.nth(1), quoted.nth(1)) else {
                continue;
            };
            let graph = state.graph();
            if graph.node(target).is_some() || !stubbed.insert(target.to_string()) {
                continue;
            }
            let Some(node) = graph.node(source) else {
                continue;
            };
            let field = node.fields.entries().iter().find(|entry| {
                matches!(&entry.value, specforge_parser::FieldValue::ReferenceList(refs)
                    if refs.iter().any(|r| r.id == target))
            });
            let target_kind = field
                .and_then(|entry| {
                    state
                        .field_registry()
                        .get(node.kind.raw.as_str(), entry.key.as_str())
                })
                .and_then(|entry| entry.target_kind.as_deref());
            if let Some(action) = code_action_create_stub(target, target_kind, &file_path) {
                actions.push(action);
            }
        }

        if actions.is_empty() {
            return Ok(None);
        }

        let lsp_actions: Vec<CodeActionOrCommand> = actions
            .into_iter()
            .map(|a| {
                let file_uri = uri_of(&state, &a.file);
                // usize::MAX appends after the file's last line.
                let appended = a.insert_line == usize::MAX;
                let line_idx = if appended {
                    content.as_deref().map_or(0, |c| c.lines().count())
                } else {
                    a.insert_line.saturating_sub(1)
                };
                let (start_char, end_char) = match a.replace_cols {
                    Some((s, e)) => {
                        let line_text = content
                            .as_deref()
                            .and_then(|c| c.lines().nth(line_idx))
                            .unwrap_or("");
                        (
                            byte_col_to_utf16(line_text, s) as u32,
                            byte_col_to_utf16(line_text, e) as u32,
                        )
                    }
                    None => (0, 0),
                };
                let mut changes = std::collections::HashMap::new();
                changes
                    .entry(file_uri)
                    .or_insert_with(Vec::new)
                    .push(TextEdit {
                        range: Range {
                            start: Position {
                                line: line_idx as u32,
                                character: start_char,
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: end_char,
                            },
                        },
                        new_text: if a.replace_cols.is_some() {
                            a.edit_text
                        } else if appended {
                            format!("\n{}\n", a.edit_text)
                        } else {
                            format!("{}\n", a.edit_text)
                        },
                    });
                CodeActionOrCommand::CodeAction(tower_lsp::lsp_types::CodeAction {
                    title: a.title,
                    kind: Some(if a.action_kind == "refactor" {
                        CodeActionKind::REFACTOR
                    } else {
                        CodeActionKind::QUICKFIX
                    }),
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
            })
            .collect();

        Ok(Some(lsp_actions))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        let state = self.state.read().await;
        let symbols = document_symbols(state.graph(), &key_of(&state, &uri));

        if symbols.is_empty() {
            return Ok(None);
        }

        // C3-09: ranges convert span byte columns to UTF-16 against the
        // file's own text, so they survive non-ASCII prefixes.
        let kind_reg = state.kind_registry();
        #[allow(deprecated)]
        let lsp_symbols: Vec<SymbolInformation> = symbols
            .into_iter()
            .map(|s| SymbolInformation {
                location: location_of(&state, &s.span),
                name: s.id,
                kind: symbol_kind_from_entity(&s.kind, kind_reg),
                tags: None,
                deprecated: None,
                container_name: Some(s.kind),
            })
            .collect();

        Ok(Some(DocumentSymbolResponse::Flat(lsp_symbols)))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let state = self.state.read().await;
        let symbols = workspace_symbols(state.graph(), &params.query);

        if symbols.is_empty() {
            return Ok(None);
        }

        let kind_reg = state.kind_registry();
        #[allow(deprecated)]
        let lsp_symbols: Vec<SymbolInformation> = symbols
            .into_iter()
            .map(|s| SymbolInformation {
                // Graph byte columns convert to UTF-16 against the file
                // text when the file is readable; byte passthrough otherwise.
                location: location_of(&state, &s.span),
                name: s.id,
                kind: symbol_kind_from_entity(&s.kind, kind_reg),
                tags: None,
                deprecated: None,
                container_name: Some(s.kind),
            })
            .collect();

        Ok(Some(lsp_symbols))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;

        let state = self.state.read().await;
        let content = match state.document(uri.as_str()) {
            Some(doc) => doc.content().to_string(),
            None => return Ok(None),
        };

        let kind_keywords: Vec<String> = state.kind_registry().keywords().cloned().collect();
        let kind_refs: Vec<&str> = kind_keywords.iter().map(|s| s.as_str()).collect();
        let caps = server_capabilities(&kind_refs);
        let token_type_index: std::collections::HashMap<&str, u32> = caps
            .semantic_token_types
            .iter()
            .enumerate()
            .map(|(i, t)| (t.as_str(), i as u32))
            .collect();

        let tokens = classify_tokens(&content, state.kind_registry());

        // Classification works in byte columns; LSP semantic tokens are
        // UTF-16. Convert per token against its own line, then delta-encode.
        let line_texts: Vec<&str> = content.lines().collect();
        let utf16 = |tok: &crate::SemanticToken| -> (u32, u32) {
            let line_text = line_texts.get(tok.line).copied().unwrap_or("");
            let start = byte_col_to_utf16(line_text, tok.col) as u32;
            (start, utf16_len(&tok.text) as u32)
        };

        let mut data = Vec::new();
        let mut prev_line: u32 = 0;
        let mut prev_col: u32 = 0;

        for tok in &tokens {
            let line = tok.line as u32;
            let (col, length) = utf16(tok);
            let delta_line = line - prev_line;
            let delta_start = if delta_line == 0 { col - prev_col } else { col };
            let token_type = token_type_index
                .get(tok.token_type.as_str())
                .copied()
                .unwrap_or(0);

            data.push(tower_lsp::lsp_types::SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type,
                token_modifiers_bitset: tok.modifiers,
            });

            prev_line = line;
            prev_col = col;
        }

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;

        let state = self.state.read().await;
        let content = match state.document(uri.as_str()) {
            Some(doc) => doc.content().to_string(),
            None => return Ok(None),
        };

        let editor_opts = EditorOptions {
            tab_size: params.options.tab_size as usize,
            insert_spaces: params.options.insert_spaces,
        };

        let (edits, diags) = format_document(&content, None, None, Some(&editor_opts));

        if !diags.is_empty() {
            // A publish replaces the document's list: the formatter's
            // diagnostics go alongside the compile ones, not in their place.
            let lsp_diags: Vec<Diagnostic> = state
                .diagnostics(uri.as_str())
                .iter()
                .chain(&diags)
                .map(|d| diagnostic_to_lsp(d, Some(&content)))
                .collect();
            let version = state.document(uri.as_str()).and_then(|d| d.version());
            self.client
                .publish_diagnostics(uri.clone(), lsp_diags, version)
                .await;
        }

        Ok(Some(formatter_edits_to_lsp(edits, &content)))
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let range = params.range;

        let state = self.state.read().await;
        let content = match state.document(uri.as_str()) {
            Some(doc) => doc.content().to_string(),
            None => return Ok(None),
        };

        let editor_opts = EditorOptions {
            tab_size: params.options.tab_size as usize,
            insert_spaces: params.options.insert_spaces,
        };

        let (edits, diags) = format_document_range(
            &content,
            range.start.line as usize,
            range.end.line as usize,
            None,
            None,
            Some(&editor_opts),
        );

        if !diags.is_empty() {
            // A publish replaces the document's list: the formatter's
            // diagnostics go alongside the compile ones, not in their place.
            let lsp_diags: Vec<Diagnostic> = state
                .diagnostics(uri.as_str())
                .iter()
                .chain(&diags)
                .map(|d| diagnostic_to_lsp(d, Some(&content)))
                .collect();
            let version = state.document(uri.as_str()).and_then(|d| d.version());
            self.client
                .publish_diagnostics(uri.clone(), lsp_diags, version)
                .await;
        }

        Ok(Some(formatter_edits_to_lsp(edits, &content)))
    }
}

#[cfg(test)]
mod docs_link_tests {
    use super::*;

    fn href(code: &str) -> Option<String> {
        let diag = specforge_common::Diagnostic::error(code, "message");
        diagnostic_to_lsp(&diag, None)
            .code_description
            .map(|d| d.href.to_string())
    }

    /// C6: the "view docs" link points at the code's anchor in
    /// docs/diagnostics.md on the canonical repository (ADR 0004 D6-b), and
    /// only for codes that have an anchor there.
    #[test]
    fn docs_links_only_codes_with_an_anchor_on_the_canonical_repository() {
        assert_eq!(
            href("E001").as_deref(),
            Some("https://github.com/leaderiop/SpecForge/blob/main/docs/diagnostics.md#e001")
        );
        assert!(
            href("R-RES-005").is_some_and(|h| h.ends_with("#r-res-005")),
            "catalogued registry codes are linked"
        );
        assert_eq!(href("E901"), None, "third-party codes have no anchor");
        assert_eq!(
            href("F011"),
            None,
            "codes outside the catalog have no anchor"
        );
        assert_eq!(
            href("E047"),
            None,
            "retired codes have no anchor of their own"
        );
    }
}
