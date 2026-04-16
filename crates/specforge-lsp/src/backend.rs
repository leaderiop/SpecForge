use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use specforge_common::Sym;
use specforge_graph::Node;
use specforge_parser::{EntityId, EntityKind};
use specforge_registry::{
    detect_mistyped_references, detect_unknown_entity_fields, detect_unknown_entity_kinds,
    populate_registries, EntityRefInfo,
};
use specforge_wasm::protocol::{
    load_protocol_extension, protocol_extension_to_manifest, ProtocolHost,
};

use crate::formatting::{format_document, format_document_range, EditorOptions};
use crate::{
    classify_tokens, code_actions_missing_verify, complete_entity_ids,
    complete_entity_ids_filtered, complete_keywords, compute_rename_edits, cursor_context,
    document_symbols, find_all_references, go_to_definition, goto_import_definition,
    hover_field_info, hover_info_with_registries, server_capabilities, server_info,
    source_span_to_lsp_range,
    workspace_symbols, LspState,
};

/// Default entity kinds used when no extension registry is available.
pub const DEFAULT_KINDS: &[&str] = &[
    "behavior",
    "type",
    "feature",
    "invariant",
    "event",
    "port",
    "journey",
    "deliverable",
    "milestone",
    "module",
    "term",
    "decision",
    "constraint",
    "failure_mode",
    "persona",
    "channel",
    "spec",
    "ref",
];

/// Testable kinds that support verify statements.
pub const TESTABLE_KINDS: &[&str] = &[
    "behavior",
    "type",
    "invariant",
    "event",
    "port",
    "constraint",
];

/// Debounce delay for `did_change` reparse (milliseconds).
const DEBOUNCE_MS: u64 = 150;

pub struct Backend {
    client: Client,
    state: Arc<RwLock<LspState>>,
    root_dir: Arc<Mutex<Option<String>>>,
    /// Resolved spec root directory (project root + spec_root from specforge.json).
    /// Falls back to project root if specforge.json is absent or has no spec_root.
    spec_root: Arc<Mutex<Option<String>>>,
    /// Per-URI debounce handles for `did_change` reparse.
    pending_updates: Arc<Mutex<HashMap<Url, JoinHandle<()>>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(RwLock::new(LspState::new())),
            root_dir: Arc::new(Mutex::new(None)),
            spec_root: Arc::new(Mutex::new(None)),
            pending_updates: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Directories to skip during workspace indexing (build artifacts, dependencies).
    const SKIP_DIRS: &[&str] = &["target", "node_modules", ".git", ".hg", "dist", "build"];

    /// Walk the workspace root for all `.spec` files and parse them into the graph.
    /// Returns the number of files indexed.
    async fn index_workspace(&self, root: &str) -> usize {
        let mut count = 0;
        for entry in walkdir::WalkDir::new(root)
            .into_iter()
            .filter_entry(|e| {
                // Skip known build/dependency directories to avoid slow traversals
                if e.file_type().is_dir()
                    && let Some(name) = e.file_name().to_str()
                {
                    return !Self::SKIP_DIRS.contains(&name);
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "spec")
                && let Ok(content) = std::fs::read_to_string(path)
            {
                let file_path = path.to_string_lossy().to_string();
                let spec_file = specforge_parser::parse(&content, &file_path);
                let mut state = self.state.write().await;
                let graph = state.graph_mut();
                for entity in &spec_file.entities {
                    graph.add_node(Node {
                        id: EntityId {
                            raw: entity.id.raw,
                        },
                        kind: EntityKind {
                            raw: entity.kind.raw,
                        },
                        title: entity.title.clone(),
                        fields: entity.fields.clone(),
                        source_span: entity.span.clone(),
                    });
                }
                count += 1;
            }
        }

        // Build edges across all indexed files so cross-file references
        // work immediately (before any file is opened in the editor).
        let mut state = self.state.write().await;
        let graph = state.graph_mut();
        // Use the shared resolve_references (same as CLI) — this clears
        // edges and rebuilds them from reference lists, discarding the
        // diagnostics since we haven't opened any documents yet.
        let _ = graph.resolve_references();

        count
    }

    /// Load extensions via the protocol pipeline and populate registries.
    /// Returns the number of extensions loaded.
    async fn load_registries(&self, project_root: &str) -> usize {
        let config_path = std::path::Path::new(project_root).join("specforge.json");
        let extensions: Vec<String> = match std::fs::read_to_string(&config_path) {
            Ok(content) => {
                let json: serde_json::Value = match serde_json::from_str(&content) {
                    Ok(v) => v,
                    Err(_) => return 0,
                };
                json.get("extensions")
                    .and_then(|e| e.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default()
            }
            Err(_) => return 0,
        };

        if extensions.is_empty() {
            return 0;
        }

        let runtime = specforge_emitter::builtins::default_runtime();
        let host = ProtocolHost::new(&runtime);
        let mut manifests = Vec::new();

        for ext_spec in &extensions {
            // Normalize path-style specifiers to canonical @specforge/ names
            let ext_name = if ext_spec.starts_with('@') {
                ext_spec.clone()
            } else {
                let last = std::path::Path::new(ext_spec)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(ext_spec);
                format!("@specforge/{}", last)
            };
            if let Ok(proto_ext) = load_protocol_extension(&host, &ext_name) {
                manifests.push(protocol_extension_to_manifest(&proto_ext));
            }
        }

        let count = manifests.len();
        if !manifests.is_empty() {
            let (kind_reg, field_reg, edge_reg, _diags) = populate_registries(&manifests);
            let mut state = self.state.write().await;
            state.set_registries(kind_reg, field_reg, edge_reg);
        }

        count
    }

    /// Parse a document, update the graph, and return diagnostics grouped by file URI.
    /// Diagnostics are keyed by URI so callers can publish each file's diagnostics
    /// under the correct URI (not all under the triggering file).
    async fn parse_and_update(
        &self,
        uri: &Url,
        content: &str,
    ) -> std::collections::HashMap<Url, Vec<Diagnostic>> {
        let file_path = uri_to_file_path(uri);

        // Get the old tree for incremental parsing
        let old_tree = {
            let state = self.state.read().await;
            state
                .document(uri.as_str())
                .and_then(|d| d.previous_tree().cloned())
        };

        let (spec_file, new_tree) =
            specforge_parser::parse_incremental(content, &file_path, old_tree.as_ref());

        let mut state = self.state.write().await;

        // Store the new tree for future incremental parses
        if let Some(doc) = state.document_mut(uri.as_str()) {
            doc.set_previous_tree(new_tree);
        }

        let graph = state.graph_mut();

        // Remove old nodes from this file
        let old_ids: Vec<Sym> = graph
            .nodes_in_file(&file_path)
            .iter()
            .map(|n| n.id.raw)
            .collect();
        for id in old_ids {
            graph.remove_node(id.as_str());
        }

        // Add new nodes from parse result
        for entity in &spec_file.entities {
            graph.add_node(Node {
                id: EntityId {
                    raw: entity.id.raw,
                },
                kind: EntityKind {
                    raw: entity.kind.raw,
                },
                title: entity.title.clone(),
                fields: entity.fields.clone(),
                source_span: entity.span.clone(),
            });
        }

        // Resolve references → edges using the shared function (same as CLI).
        // This is the single source of truth for E001 unresolved-reference
        // diagnostics, ensuring LSP and CLI report identical errors.
        let ref_diags = graph.resolve_references();

        // Snapshot node data for registry-based diagnostics below.
        let all_nodes: Vec<(Sym, specforge_parser::FieldMap, specforge_common::SourceSpan)> =
            graph
                .nodes()
                .iter()
                .map(|n| (n.id.raw, n.fields.clone(), n.source_span.clone()))
                .collect();

        // Collect all diagnostics grouped by file URI
        let mut diags_by_file: std::collections::HashMap<Url, Vec<Diagnostic>> =
            std::collections::HashMap::new();

        // Parse errors belong to the triggering file
        for e in &spec_file.errors {
            diags_by_file
                .entry(uri.clone())
                .or_default()
                .push(Diagnostic {
                    range: source_span_to_range(&e.span),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("E001".into())),
                    source: Some("specforge".into()),
                    message: e.message.clone(),
                    ..Default::default()
                });
        }

        // Resolver diagnostics: unresolved references, grouped by file
        for rd in &ref_diags {
            let diag_uri = rd
                .span
                .as_ref()
                .map(|s| file_path_to_uri(s.file.as_str()))
                .unwrap_or_else(|| uri.clone());
            diags_by_file
                .entry(diag_uri)
                .or_default()
                .push(diagnostic_to_lsp(rd));
        }

        // Release the mutable graph borrow so we can access state immutably below
        let _ = graph;

        // Validator diagnostics, grouped by each diagnostic's own file
        let validator_diags = specforge_validator::validate(state.graph_mut());
        for vd in &validator_diags {
            let diag_uri = vd
                .span
                .as_ref()
                .map(|s| file_path_to_uri(s.file.as_str()))
                .unwrap_or_else(|| uri.clone());
            diags_by_file
                .entry(diag_uri)
                .or_default()
                .push(diagnostic_to_lsp(vd));
        }

        // E022: Mistyped reference diagnostics (wrong-kind targets)
        let field_reg = state.field_registry();
        let kind_reg = state.kind_registry();
        if !field_reg.is_empty() && !kind_reg.is_empty() {
            let graph = state.graph();
            let node_kind_index: std::collections::HashMap<String, String> = graph
                .nodes()
                .iter()
                .map(|n| (n.id.raw.to_string(), n.kind.raw.to_string()))
                .collect();

            let entity_refs: Vec<EntityRefInfo> = all_nodes
                .iter()
                .map(|(id, fields, span)| {
                    let ref_fields: Vec<(String, Vec<String>)> = fields
                        .entries()
                        .iter()
                        .filter_map(|entry| {
                            if let specforge_parser::FieldValue::ReferenceList(refs) = &entry.value {
                                Some((entry.key.to_string(), refs.clone()))
                            } else {
                                None
                            }
                        })
                        .collect();
                    let entity_kind = graph
                        .node(id.as_str())
                        .map(|n| n.kind.raw.to_string())
                        .unwrap_or_default();
                    (entity_kind, id.to_string(), ref_fields, span.clone())
                })
                .collect();

            let w022_diags = detect_mistyped_references(
                &entity_refs,
                field_reg,
                kind_reg,
                &node_kind_index,
            );
            for d in &w022_diags {
                let diag_uri = d
                    .span
                    .as_ref()
                    .map(|s| file_path_to_uri(s.file.as_str()))
                    .unwrap_or_else(|| uri.clone());
                diags_by_file
                    .entry(diag_uri)
                    .or_default()
                    .push(diagnostic_to_lsp(d));
            }
        }

        // E024: Unknown entity kinds + W020: Unknown entity fields
        // Only fire when registries are populated (not in structural-only mode).
        if !kind_reg.is_empty() {
            let graph = state.graph();

            // E024: entity kinds not registered by any extension
            let entity_kinds: Vec<(String, String, specforge_common::SourceSpan)> = graph
                .nodes()
                .iter()
                .map(|n| (n.kind.raw.to_string(), n.id.raw.to_string(), n.source_span.clone()))
                .collect();
            let e024_diags = detect_unknown_entity_kinds(&entity_kinds, kind_reg, None);
            for d in &e024_diags {
                let diag_uri = d
                    .span
                    .as_ref()
                    .map(|s| file_path_to_uri(s.file.as_str()))
                    .unwrap_or_else(|| uri.clone());
                diags_by_file
                    .entry(diag_uri)
                    .or_default()
                    .push(diagnostic_to_lsp(d));
            }

            // W020: fields not registered for their entity kind
            if !field_reg.is_empty() {
                let entity_fields: Vec<(String, String, Vec<String>, specforge_common::SourceSpan)> = graph
                    .nodes()
                    .iter()
                    .map(|n| {
                        let field_names: Vec<String> = n.fields.entries().iter()
                            .map(|e| e.key.to_string())
                            .collect();
                        (n.kind.raw.to_string(), n.id.raw.to_string(), field_names, n.source_span.clone())
                    })
                    .collect();
                let w020_diags = detect_unknown_entity_fields(&entity_fields, kind_reg, field_reg);
                for d in &w020_diags {
                    let diag_uri = d
                        .span
                        .as_ref()
                        .map(|s| file_path_to_uri(s.file.as_str()))
                        .unwrap_or_else(|| uri.clone());
                    diags_by_file
                        .entry(diag_uri)
                        .or_default()
                        .push(diagnostic_to_lsp(d));
                }
            }
        }

        // Ensure the triggering file always has an entry (even if empty)
        // so its diagnostics get cleared when there are no errors.
        diags_by_file.entry(uri.clone()).or_default();

        // Collect all files that had diagnostics before — they need to be
        // cleared if they no longer have any.
        let known_files: Vec<Sym> = state
            .graph()
            .nodes()
            .iter()
            .map(|n| n.source_span.file)
            .collect();
        for file in &known_files {
            let file_uri = file_path_to_uri(file.as_str());
            diags_by_file.entry(file_uri).or_default();
        }

        diags_by_file
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

fn diagnostic_to_lsp(diag: &specforge_common::Diagnostic) -> Diagnostic {
    Diagnostic {
        range: diag
            .span
            .as_ref()
            .map(source_span_to_range)
            .unwrap_or_default(),
        severity: Some(match diag.severity {
            specforge_common::Severity::Error => DiagnosticSeverity::ERROR,
            specforge_common::Severity::Warning => DiagnosticSeverity::WARNING,
            specforge_common::Severity::Info => DiagnosticSeverity::INFORMATION,
        }),
        code: Some(NumberOrString::String(diag.code.clone())),
        source: Some("specforge".into()),
        message: diag.message.clone(),
        ..Default::default()
    }
}

fn symbol_kind_from_entity(kind: &str) -> SymbolKind {
    match kind {
        "behavior" => SymbolKind::METHOD,
        "type" => SymbolKind::STRUCT,
        "feature" => SymbolKind::MODULE,
        "invariant" => SymbolKind::CONSTANT,
        "event" => SymbolKind::EVENT,
        "port" => SymbolKind::INTERFACE,
        "spec" => SymbolKind::NAMESPACE,
        _ => SymbolKind::VARIABLE,
    }
}

/// Extract the word at a given cursor position from document content.
pub fn word_at_position(content: &str, line: usize, col: usize) -> Option<String> {
    let target_line = content.lines().nth(line)?;
    if col > target_line.len() {
        return None;
    }
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
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

fn publish_format_diags(diags: &[specforge_common::Diagnostic]) -> Vec<Diagnostic> {
    diags.iter().map(diagnostic_to_lsp).collect()
}

fn formatter_edits_to_lsp(edits: Vec<specforge_formatter::TextEdit>) -> Vec<TextEdit> {
    edits
        .into_iter()
        .map(|e| TextEdit {
            range: Range {
                start: Position {
                    line: e.start_line as u32,
                    character: e.start_col as u32,
                },
                end: Position {
                    line: e.end_line as u32,
                    character: e.end_col as u32,
                },
            },
            new_text: e.new_text,
        })
        .collect()
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
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
        // Resolve spec_root from specforge.json (falls back to project root)
        let resolved_spec_root = root.as_deref().and_then(|r| {
            let config_path = std::path::Path::new(r).join("specforge.json");
            let content = std::fs::read_to_string(&config_path).ok()?;
            let json: serde_json::Value = serde_json::from_str(&content).ok()?;
            let spec_root_field = json.get("spec_root")?.as_str()?;
            let resolved = std::path::Path::new(r).join(spec_root_field);
            if resolved.is_dir() {
                Some(resolved.to_string_lossy().to_string())
            } else {
                None
            }
        }).or_else(|| root.clone());
        *self.spec_root.lock().await = resolved_spec_root;
        *self.root_dir.lock().await = root;
        let caps = server_capabilities(DEFAULT_KINDS);
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
                document_formatting_provider: Some(OneOf::Left(
                    caps.supports_document_formatting,
                )),
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
                        watchers: vec![FileSystemWatcher {
                            glob_pattern: GlobPattern::String("**/*.spec".into()),
                            kind: Some(WatchKind::all()),
                        }],
                    })
                    .unwrap(),
                ),
            }])
            .await;

        // Load extension registries from specforge.json before indexing
        let root_dir = self.root_dir.lock().await.clone();
        if let Some(ref root) = root_dir {
            let ext_count = self.load_registries(root).await;
            if ext_count > 0 {
                self.client
                    .log_message(
                        MessageType::INFO,
                        format!("specforge-lsp: loaded {ext_count} extension(s)"),
                    )
                    .await;
            }
        }

        // Use spec_root (from specforge.json) for indexing — it's much narrower than
        // root_dir and avoids traversing huge directories like target/ or node_modules/.
        let spec_root = self.spec_root.lock().await.clone();
        let index_root = spec_root.or(root_dir);
        if let Some(root_path) = index_root {
            let count = self.index_workspace(&root_path).await;
            self.client
                .log_message(
                    MessageType::INFO,
                    format!("specforge-lsp: indexed {count} .spec files from {root_path}"),
                )
                .await;

            // Re-diagnose all already-open documents now that the full graph
            // is populated.  Without this, didOpen diagnostics that raced
            // against indexing would show stale E001 errors for cross-file
            // references that hadn't been indexed yet.
            let open_uris: Vec<(Url, String)> = {
                let state = self.state.read().await;
                state
                    .open_uris()
                    .into_iter()
                    .filter_map(|uri_str| {
                        let uri = Url::parse(uri_str).ok()?;
                        let content = state.document(uri_str)?.content().to_string();
                        Some((uri, content))
                    })
                    .collect()
            };
            for (uri, content) in open_uris {
                let diags_by_file = self.parse_and_update(&uri, &content).await;
                for (file_uri, diags) in diags_by_file {
                    self.client
                        .publish_diagnostics(file_uri, diags, None)
                        .await;
                }
            }
        } else {
            self.client
                .log_message(MessageType::INFO, "specforge-lsp initialized (no root_uri)")
                .await;
        }
    }

    async fn shutdown(&self) -> Result<()> {
        self.state.write().await.shutdown();
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;

        self.state
            .write()
            .await
            .open_document(uri.as_str(), &text);

        let diags_by_file = self.parse_and_update(&uri, &text).await;
        for (file_uri, diags) in diags_by_file {
            self.client.publish_diagnostics(file_uri, diags, None).await;
        }
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
        }

        // Cancel any pending debounced reparse for this URI
        {
            let mut pending = self.pending_updates.lock().await;
            if let Some(handle) = pending.remove(&uri) {
                handle.abort();
            }
        }

        // Spawn a debounced reparse task
        let state = self.state.clone();
        let client = self.client.clone();
        let pending = self.pending_updates.clone();
        let uri_clone = uri.clone();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(DEBOUNCE_MS)).await;

            // Remove ourselves from pending map
            {
                let mut p = pending.lock().await;
                p.remove(&uri_clone);
            }

            let content = {
                let s = state.read().await;
                s.document(uri_clone.as_str())
                    .map(|d| d.content().to_string())
            };
            if let Some(content) = content {
                let file_path = uri_to_file_path(&uri_clone);
                let old_tree = {
                    let s = state.read().await;
                    s.document(uri_clone.as_str())
                        .and_then(|d| d.previous_tree().cloned())
                };
                let (spec_file, new_tree) =
                    specforge_parser::parse_incremental(&content, &file_path, old_tree.as_ref());

                let mut s = state.write().await;

                // Store the new tree
                if let Some(doc) = s.document_mut(uri_clone.as_str()) {
                    doc.set_previous_tree(new_tree);
                }

                let graph = s.graph_mut();

                // Remove old nodes from this file
                let old_ids: Vec<Sym> = graph
                    .nodes_in_file(&file_path)
                    .iter()
                    .map(|n| n.id.raw)
                    .collect();
                for id in old_ids {
                    graph.remove_node(id.as_str());
                }

                // Add new nodes
                for entity in &spec_file.entities {
                    graph.add_node(Node {
                        id: EntityId { raw: entity.id.raw },
                        kind: EntityKind { raw: entity.kind.raw },
                        title: entity.title.clone(),
                        fields: entity.fields.clone(),
                        source_span: entity.span.clone(),
                    });
                }

                // Resolve references → edges using the shared function (same as CLI)
                let ref_diags = graph.resolve_references();

                let all_nodes: Vec<(Sym, specforge_parser::FieldMap, specforge_common::SourceSpan)> =
                    graph
                        .nodes()
                        .iter()
                        .map(|n| (n.id.raw, n.fields.clone(), n.source_span.clone()))
                        .collect();

                let mut diags_by_file: std::collections::HashMap<Url, Vec<Diagnostic>> =
                    std::collections::HashMap::new();

                for e in &spec_file.errors {
                    diags_by_file
                        .entry(uri_clone.clone())
                        .or_default()
                        .push(Diagnostic {
                            range: source_span_to_range(&e.span),
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: Some(NumberOrString::String("E001".into())),
                            source: Some("specforge".into()),
                            message: e.message.clone(),
                            ..Default::default()
                        });
                }

                // Resolver diagnostics from shared function
                for rd in &ref_diags {
                    let diag_uri = rd.span.as_ref()
                        .map(|sp| file_path_to_uri(sp.file.as_str()))
                        .unwrap_or_else(|| uri_clone.clone());
                    diags_by_file.entry(diag_uri).or_default().push(diagnostic_to_lsp(rd));
                }

                // Validator diagnostics
                let validator_diags = specforge_validator::validate(s.graph_mut());
                for vd in &validator_diags {
                    let diag_uri = vd
                        .span
                        .as_ref()
                        .map(|sp| file_path_to_uri(sp.file.as_str()))
                        .unwrap_or_else(|| uri_clone.clone());
                    diags_by_file
                        .entry(diag_uri)
                        .or_default()
                        .push(diagnostic_to_lsp(vd));
                }

                // E024/W020/E022: Registry-based diagnostics
                let kind_reg = s.kind_registry();
                let field_reg = s.field_registry();
                if !kind_reg.is_empty() {
                    let graph = s.graph();

                    // E024: unknown entity kinds
                    let entity_kinds: Vec<(String, String, specforge_common::SourceSpan)> = graph
                        .nodes().iter()
                        .map(|n| (n.kind.raw.to_string(), n.id.raw.to_string(), n.source_span.clone()))
                        .collect();
                    for d in &detect_unknown_entity_kinds(&entity_kinds, kind_reg, None) {
                        let diag_uri = d.span.as_ref()
                            .map(|sp| file_path_to_uri(sp.file.as_str()))
                            .unwrap_or_else(|| uri_clone.clone());
                        diags_by_file.entry(diag_uri).or_default().push(diagnostic_to_lsp(d));
                    }

                    // W020: unknown entity fields
                    if !field_reg.is_empty() {
                        let entity_fields: Vec<(String, String, Vec<String>, specforge_common::SourceSpan)> = graph
                            .nodes().iter()
                            .map(|n| {
                                let fnames: Vec<String> = n.fields.entries().iter().map(|e| e.key.to_string()).collect();
                                (n.kind.raw.to_string(), n.id.raw.to_string(), fnames, n.source_span.clone())
                            })
                            .collect();
                        for d in &detect_unknown_entity_fields(&entity_fields, kind_reg, field_reg) {
                            let diag_uri = d.span.as_ref()
                                .map(|sp| file_path_to_uri(sp.file.as_str()))
                                .unwrap_or_else(|| uri_clone.clone());
                            diags_by_file.entry(diag_uri).or_default().push(diagnostic_to_lsp(d));
                        }

                        // E022: mistyped references
                        let node_kind_index: std::collections::HashMap<String, String> = graph
                            .nodes().iter()
                            .map(|n| (n.id.raw.to_string(), n.kind.raw.to_string()))
                            .collect();
                        let entity_refs: Vec<EntityRefInfo> = all_nodes.iter()
                            .map(|(id, fields, span)| {
                                let ref_fields: Vec<(String, Vec<String>)> = fields.entries().iter()
                                    .filter_map(|entry| {
                                        if let specforge_parser::FieldValue::ReferenceList(refs) = &entry.value {
                                            Some((entry.key.to_string(), refs.clone()))
                                        } else { None }
                                    })
                                    .collect();
                                let entity_kind = graph.node(id.as_str())
                                    .map(|n| n.kind.raw.to_string()).unwrap_or_default();
                                (entity_kind, id.to_string(), ref_fields, span.clone())
                            })
                            .collect();
                        for d in &detect_mistyped_references(&entity_refs, field_reg, kind_reg, &node_kind_index) {
                            let diag_uri = d.span.as_ref()
                                .map(|sp| file_path_to_uri(sp.file.as_str()))
                                .unwrap_or_else(|| uri_clone.clone());
                            diags_by_file.entry(diag_uri).or_default().push(diagnostic_to_lsp(d));
                        }
                    }
                }

                diags_by_file.entry(uri_clone.clone()).or_default();

                let known_files: Vec<Sym> = s
                    .graph()
                    .nodes()
                    .iter()
                    .map(|n| n.source_span.file)
                    .collect();
                for file in &known_files {
                    let file_uri = file_path_to_uri(file.as_str());
                    diags_by_file.entry(file_uri).or_default();
                }

                // Drop write lock before publishing
                drop(s);

                for (file_uri, diags) in diags_by_file {
                    client.publish_diagnostics(file_uri, diags, None).await;
                }
            }
        });

        // Store the handle for cancellation
        let mut pending = self.pending_updates.lock().await;
        pending.insert(uri, handle);
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.state
            .write()
            .await
            .close_document(params.text_document.uri.as_str());
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        for change in &params.changes {
            let uri = &change.uri;
            let file_path = uri_to_file_path(uri);

            // Only handle .spec files
            if !file_path.ends_with(".spec") {
                continue;
            }

            match change.typ {
                FileChangeType::DELETED => {
                    // Remove all entities from this file and rebuild edges
                    let mut state = self.state.write().await;
                    let graph = state.graph_mut();
                    let old_ids: Vec<Sym> = graph
                        .nodes_in_file(&file_path)
                        .iter()
                        .map(|n| n.id.raw)
                        .collect();
                    for id in &old_ids {
                        graph.remove_node(id.as_str());
                    }
                    // Resolve references using the shared function (same as CLI)
                    let ref_diags = graph.resolve_references();

                    // Run validator and publish updated diagnostics for all affected files
                    let validator_diags = specforge_validator::validate(graph);
                    let known_files: Vec<Sym> = graph
                        .nodes()
                        .iter()
                        .map(|n| n.source_span.file)
                        .collect();

                    // Publish empty diagnostics for the deleted file (clear stale squiggles)
                    self.client
                        .publish_diagnostics(uri.clone(), vec![], None)
                        .await;

                    // Publish updated diagnostics for remaining files
                    let mut diags_by_file: std::collections::HashMap<Url, Vec<Diagnostic>> =
                        std::collections::HashMap::new();
                    for rd in &ref_diags {
                        let diag_uri = rd
                            .span
                            .as_ref()
                            .map(|s| file_path_to_uri(s.file.as_str()))
                            .unwrap_or_else(|| uri.clone());
                        diags_by_file
                            .entry(diag_uri)
                            .or_default()
                            .push(diagnostic_to_lsp(rd));
                    }
                    for vd in &validator_diags {
                        let diag_uri = vd
                            .span
                            .as_ref()
                            .map(|s| file_path_to_uri(s.file.as_str()))
                            .unwrap_or_else(|| uri.clone());
                        diags_by_file
                            .entry(diag_uri)
                            .or_default()
                            .push(diagnostic_to_lsp(vd));
                    }
                    // Ensure all known files get an entry (clears stale diagnostics)
                    for file in &known_files {
                        diags_by_file.entry(file_path_to_uri(file.as_str())).or_default();
                    }
                    drop(state);
                    for (file_uri, diags) in diags_by_file {
                        self.client
                            .publish_diagnostics(file_uri, diags, None)
                            .await;
                    }
                }
                _ => {
                    // Created or Changed — re-read from disk and update graph
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        let diags_by_file = self.parse_and_update(uri, &content).await;
                        for (file_uri, diags) in diags_by_file {
                            self.client
                                .publish_diagnostics(file_uri, diags, None)
                                .await;
                        }
                    }
                }
            }
        }
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
        let kr = if kind_reg.is_empty() { None } else { Some(kind_reg) };
        let fr = if field_reg.is_empty() { None } else { Some(field_reg) };
        let info = hover_info_with_registries(state.graph(), &word, kr, fr).or_else(|| {
            // Fallback: try field hover if word is not an entity ID
            if !field_reg.is_empty() {
                let entity_kind = crate::completion::enclosing_entity_kind(
                    &content,
                    pos.line as usize,
                )?;
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

        let entity_items = if let Some(ref tk) = target_kind {
            complete_entity_ids_filtered(state.graph(), &prefix, Some(tk))
        } else {
            complete_entity_ids(state.graph(), &prefix)
        };

        for item in entity_items {
            let detail = item
                .title
                .as_ref()
                .map(|t| format!("{} — {}", item.kind, t))
                .unwrap_or_else(|| item.kind.clone());
            items.push(CompletionItem {
                label: item.id.clone(),
                kind: Some(CompletionItemKind::REFERENCE),
                detail: Some(detail),
                ..Default::default()
            });
        }

        if pos.character < 2 {
            // Use registered kinds if available, fall back to hardcoded defaults
            let kind_reg = state.kind_registry();
            let dynamic_kinds: Vec<String> = kind_reg.keywords().cloned().collect();
            if !dynamic_kinds.is_empty() {
                let kind_refs: Vec<&str> = dynamic_kinds.iter().map(|s| s.as_str()).collect();
                for kw in complete_keywords(&kind_refs) {
                    items.push(CompletionItem {
                        label: kw,
                        kind: Some(CompletionItemKind::KEYWORD),
                        ..Default::default()
                    });
                }
            } else {
                for kw in complete_keywords(DEFAULT_KINDS) {
                    items.push(CompletionItem {
                        label: kw,
                        kind: Some(CompletionItemKind::KEYWORD),
                        ..Default::default()
                    });
                }
            }
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
            let resolved = self.spec_root.lock().await;
            if let Some(spec_root) = resolved.as_deref() {
                let span = goto_import_definition(import_path, spec_root);
                return Ok(
                    span.map(|s| GotoDefinitionResponse::Scalar(source_span_to_location(&s)))
                );
            }
        }

        let word = match word_at_position(&content, pos.line as usize, pos.character as usize) {
            Some(w) => w,
            None => return Ok(None),
        };

        let span = go_to_definition(state.graph(), &word);
        Ok(span.map(|s| GotoDefinitionResponse::Scalar(source_span_to_location(&s))))
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
        Ok(Some(refs.iter().map(source_span_to_location).collect()))
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
        Ok(span.map(|s| PrepareRenameResponse::Range(source_span_to_range(&s))))
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

        let edits = match compute_rename_edits(state.graph(), &word, &new_name) {
            Some(e) => e,
            None => return Ok(None),
        };

        let mut changes: std::collections::HashMap<Url, Vec<TextEdit>> =
            std::collections::HashMap::new();
        for edit in edits {
            let file_uri = file_path_to_uri(&edit.file);
            changes.entry(file_uri).or_default().push(TextEdit {
                range: Range {
                    start: Position {
                        line: edit.line.saturating_sub(1) as u32,
                        character: edit.start_col.saturating_sub(1) as u32,
                    },
                    end: Position {
                        line: edit.line.saturating_sub(1) as u32,
                        character: edit.end_col.saturating_sub(1) as u32,
                    },
                },
                new_text: edit.new_text,
            });
        }

        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let file_path = uri_to_file_path(&uri);

        let state = self.state.read().await;
        let actions = code_actions_missing_verify(state.graph(), &file_path, TESTABLE_KINDS);

        if actions.is_empty() {
            return Ok(None);
        }

        let lsp_actions: Vec<CodeActionOrCommand> = actions
            .into_iter()
            .map(|a| {
                let file_uri = file_path_to_uri(&a.file);
                let mut changes = std::collections::HashMap::new();
                changes.entry(file_uri).or_insert_with(Vec::new).push(
                    TextEdit {
                        range: Range {
                            start: Position {
                                line: a.insert_line as u32,
                                character: 0,
                            },
                            end: Position {
                                line: a.insert_line as u32,
                                character: 0,
                            },
                        },
                        new_text: format!("{}\n", a.edit_text),
                    },
                );
                CodeActionOrCommand::CodeAction(tower_lsp::lsp_types::CodeAction {
                    title: a.title,
                    kind: Some(CodeActionKind::QUICKFIX),
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
        let file_path = uri_to_file_path(&uri);

        let state = self.state.read().await;
        let symbols = document_symbols(state.graph(), &file_path);

        if symbols.is_empty() {
            return Ok(None);
        }

        #[allow(deprecated)]
        let lsp_symbols: Vec<SymbolInformation> = symbols
            .into_iter()
            .map(|s| SymbolInformation {
                name: s.id,
                kind: symbol_kind_from_entity(&s.kind),
                tags: None,
                deprecated: None,
                location: source_span_to_location(&s.span),
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

        #[allow(deprecated)]
        let lsp_symbols: Vec<SymbolInformation> = symbols
            .into_iter()
            .map(|s| SymbolInformation {
                name: s.id,
                kind: symbol_kind_from_entity(&s.kind),
                tags: None,
                deprecated: None,
                location: source_span_to_location(&s.span),
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

        let caps = server_capabilities(DEFAULT_KINDS);
        let token_type_index: std::collections::HashMap<&str, u32> = caps
            .semantic_token_types
            .iter()
            .enumerate()
            .map(|(i, t)| (t.as_str(), i as u32))
            .collect();

        let tokens = classify_tokens(&content, DEFAULT_KINDS);

        let mut data = Vec::new();
        let mut prev_line: u32 = 0;
        let mut prev_col: u32 = 0;

        for tok in &tokens {
            let line = tok.line as u32;
            let col = tok.col as u32;
            let delta_line = line - prev_line;
            let delta_start = if delta_line == 0 {
                col - prev_col
            } else {
                col
            };
            let length = tok.text.len() as u32;
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

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
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

        let lsp_diags = publish_format_diags(&diags);
        if !lsp_diags.is_empty() {
            self.client
                .publish_diagnostics(uri.clone(), lsp_diags, None)
                .await;
        }

        Ok(Some(formatter_edits_to_lsp(edits)))
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

        let lsp_diags = publish_format_diags(&diags);
        if !lsp_diags.is_empty() {
            self.client
                .publish_diagnostics(uri.clone(), lsp_diags, None)
                .await;
        }

        Ok(Some(formatter_edits_to_lsp(edits)))
    }
}
