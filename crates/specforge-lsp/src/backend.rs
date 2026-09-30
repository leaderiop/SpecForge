use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, RwLock};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use specforge_common::Sym;
use specforge_graph::{GraphConfig, build_graph_with_config};
use specforge_registry::{KindRegistry, populate_registries};
use specforge_wasm::protocol::{
    ProtocolHost, load_protocol_extension, protocol_extension_to_manifest,
};
use specforge_watch::{ImportDag, IncrementalPipeline};

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
    root_dir: Arc<Mutex<Option<String>>>,
    /// All workspace roots to index (C4-04): rootUri plus every
    /// workspace folder, not just the first.
    workspace_roots: Arc<Mutex<Vec<String>>>,
    /// Resolved spec root directory (project root + spec_root from specforge.json).
    /// Falls back to project root if specforge.json is absent or has no spec_root.
    spec_root: Arc<Mutex<Option<String>>>,
    /// Latest-wins reparse requests (C4-03): keystrokes send here; one
    /// serialized worker coalesces and processes, so at most one
    /// whole-graph pass runs at a time and the state lock is never held
    /// across a keystroke storm.
    update_tx: mpsc::UnboundedSender<Url>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let state = Arc::new(RwLock::new(LspState::new()));
        let (update_tx, mut update_rx) = mpsc::unbounded_channel::<Url>();

        // Serialized latest-wins reparse worker (C4-03). Exits when the
        // Backend (and its sender) is dropped.
        let worker_state = Arc::clone(&state);
        let worker_client = client.clone();
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
                    let (version, content) = {
                        let s = worker_state.read().await;
                        match s.document(uri.as_str()) {
                            Some(doc) => (doc.version(), Some(doc.content().to_string())),
                            None => (None, None),
                        }
                    };
                    let Some(content) = content else {
                        continue;
                    };
                    // Single shared recompute path (same as did_open): drives
                    // the incremental pipeline and assembles every layer.
                    let diags_by_file = Self::parse_and_update(&worker_state, &uri, &content).await;
                    for (file_uri, diags) in diags_by_file {
                        worker_client
                            .publish_diagnostics(file_uri, diags, version)
                            .await;
                    }
                }
            }
        });

        Self {
            client,
            state,
            root_dir: Arc::new(Mutex::new(None)),
            workspace_roots: Arc::new(Mutex::new(Vec::new())),
            spec_root: Arc::new(Mutex::new(None)),
            update_tx,
        }
    }

    /// Walk the workspace roots for all `.spec` files and parse them into
    /// the graph. Uses the shared discovery policy (C14-16) with the
    /// project's `exclude` patterns from specforge.json (C4-04). Static
    /// over the shared state so the background indexing task can call it
    /// without borrowing the backend. Returns the number of files indexed.
    async fn index_roots_static(state: &RwLock<LspState>, roots: &[String]) -> usize {
        let roots_owned: Vec<String> = roots.to_vec();
        let parsed: Vec<(String, specforge_parser::SpecFile)> =
            tokio::task::spawn_blocking(move || {
                let mut files = Vec::new();
                for root in &roots_owned {
                    let root_path = std::path::Path::new(root);
                    let exclude = specforge_common::load_project_config(root_path).exclude;
                    for path in specforge_common::discover_spec_files(root_path, &exclude) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let file_path = path.to_string_lossy().to_string();
                            files.push((
                                file_path,
                                specforge_parser::parse(&content, &path.to_string_lossy()),
                            ));
                        }
                    }
                }
                files
            })
            .await
            .unwrap_or_default();

        let count = parsed.len();

        // Build the import DAG so later edits invalidate importers.
        let mut dag = ImportDag::new();
        for (path, spec_file) in &parsed {
            let imports: Vec<String> = spec_file
                .imports
                .iter()
                .map(|i| i.path.to_string())
                .collect();
            dag.set_imports_resolved(path, imports);
        }

        // Build the graph through the same build_graph_with_config the CLI
        // uses, seeded from the loaded extension registries — so LSP
        // diagnostics (duplicates, unresolved references, cycles) match.
        let mut state = state.write().await;
        let single_reference_fields: std::collections::HashSet<(String, String)> = state
            .field_registry()
            .iter()
            .filter(|(_, _, entry)| {
                entry.field_type == specforge_registry::ManifestFieldType::Reference
            })
            .map(|(kind, field, _)| (kind.to_string(), field.to_string()))
            .collect();
        // Body-parser kinds own syntax the core grammar does not parse;
        // their E001s are suppressed exactly as the CLI suppresses them.
        let body_parser_kinds: std::collections::HashSet<String> = state
            .kind_registry()
            .iter()
            .filter(|(_, e)| e.has_body_parser)
            .map(|(k, _)| k.clone())
            .collect();
        let suppressed_parse_error_ranges: Vec<(String, usize, usize)> = parsed
            .iter()
            .flat_map(|(path, sf)| {
                sf.entities
                    .iter()
                    .filter(|e| body_parser_kinds.contains(e.kind.raw.as_str()))
                    .map(move |e| (path.clone(), e.span.start_line, e.span.end_line))
            })
            .collect();
        let graph_config = GraphConfig {
            installed_keywords: state.kind_registry().keywords().cloned().collect(),
            known_provider_schemes: std::collections::HashSet::new(),
            known_extension_keywords: state.known_extension_keywords().clone(),
            bidirectional_pairs: state.field_registry().bidirectional_pairs(),
            suppressed_parse_error_ranges,
            single_reference_fields,
            absent_reference_targets: state
                .field_registry()
                .absent_reference_targets(state.kind_registry()),
        };
        let spec_files: Vec<specforge_parser::SpecFile> =
            parsed.iter().map(|(_, sf)| sf.clone()).collect();
        let (graph, build_diagnostics) = build_graph_with_config(&spec_files, &graph_config);
        *state.pipeline_mut() = IncrementalPipeline::from_cold_build(
            parsed,
            graph,
            dag,
            build_diagnostics,
            graph_config,
        );

        count
    }

    /// Load extensions via the protocol pipeline and populate registries.
    /// Static over the shared state so the background indexing task (C4-04)
    /// can call it without borrowing the backend.
    /// Returns the number of extensions loaded.
    async fn load_registries_static(state: &RwLock<LspState>, project_root: &str) -> usize {
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

        // One Wasm runtime per session — the same constructor the CLI and
        // MCP use (WASM-only migration, Phase 4).
        let runtime = specforge_component::project_runtime(std::path::Path::new(project_root));
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

            // Parse extension-declared validation rules
            let rule_inputs: Vec<(String, Vec<_>)> = manifests
                .iter()
                .map(|m| (m.name.clone(), m.validation_rules.clone()))
                .collect();
            let (mut patterns, _rule_diags) =
                specforge_registry::validation_engine::parse_all_rule_patterns(&rule_inputs);
            specforge_registry::validation_engine::resolve_edge_rules(
                &mut patterns,
                &edge_reg,
                &kind_reg,
            );

            // Auto-generate E006 rules for required fields (originless,
            // host-generated, declarative)
            let required_rules = specforge_registry::generate_required_field_rules(&field_reg);
            patterns.extend(required_rules.into_iter().map(|p| (p, String::new())));

            // Keyword -> extension index for I004 hints (same derivation as
            // the CLI pipeline: emitter compile.rs known_extension_keywords).
            let known_extension_keywords: HashMap<String, String> = manifests
                .iter()
                .flat_map(|m| {
                    m.entity_kinds
                        .iter()
                        .map(move |k| (k.keyword.clone(), m.name.clone()))
                })
                .collect();
            let mut state = state.write().await;
            state.set_registries(kind_reg, field_reg, edge_reg);
            state.set_validation_patterns(patterns);
            state.set_known_extension_keywords(known_extension_keywords);
            state.set_runtime(runtime);
        }

        count
    }

    /// Parse a document, update the graph, and return diagnostics grouped by file URI.
    /// Diagnostics are keyed by URI so callers can publish each file's diagnostics
    /// under the correct URI (not all under the triggering file). Callers stamp the
    /// triggering document's editor version onto every publish so clients can drop
    /// stale deliveries (C4-05).
    async fn parse_and_update(
        state: &RwLock<LspState>,
        uri: &Url,
        content: &str,
    ) -> std::collections::HashMap<Url, Vec<Diagnostic>> {
        let file_path = uri_to_file_path(uri);

        // Drive the shared incremental pipeline (the same core `specforge
        // watch` uses): the open buffer is authoritative for this file, while
        // transitively invalidated files (importers) are re-read from disk.
        // The pipeline retains tree-sitter trees and re-parses incrementally.
        //
        // The update does synchronous fs reads and a full re-parse + rebuild,
        // so it runs on the blocking pool: the pipeline is taken out of the
        // state (brief write lock), computed without any lock held, and put
        // back (brief write lock). Async workers are never blocked.
        let content_owned = content.to_string();
        let file_path_owned = file_path.clone();
        let joined = {
            let mut pipeline = {
                let mut st = state.write().await;
                st.take_pipeline()
            };
            tokio::task::spawn_blocking(move || {
                let result =
                    pipeline.update_open_file(&file_path_owned, Some(&content_owned), |f: &str| {
                        std::fs::read_to_string(f).ok()
                    });
                (pipeline, result)
            })
            .await
        };
        let (pipeline, result) = match joined {
            Ok(pair) => pair,
            Err(e) => {
                // Blocking task panicked: report as an E001 on the file.
                let mut m = std::collections::HashMap::new();
                let diag = Diagnostic {
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("E001".into())),
                    source: Some("specforge".into()),
                    message: format!("internal error during reparse: {e}"),
                    ..Default::default()
                };
                m.insert(uri.clone(), vec![diag]);
                return m;
            }
        };
        {
            let mut st = state.write().await;
            st.set_pipeline(pipeline);
        }

        // Everything below only reads the graph, registries, and cached
        // diagnostics — the graph validator takes `&Graph`. Hold the read
        // lock, not the write lock, so concurrent reads never queue behind
        // a long analysis pass (C4-05); writers are serialized by the
        // reparse worker (C4-03).
        let lock = state;
        let state = lock.read().await;

        // Collect all diagnostics grouped by file URI, as published and as
        // the core diagnostics code actions work from.
        let mut diags_by_file: std::collections::HashMap<Url, Vec<Diagnostic>> =
            std::collections::HashMap::new();
        let mut core_by_file: std::collections::HashMap<Url, Vec<specforge_common::Diagnostic>> =
            std::collections::HashMap::new();

        // Pipeline diagnostics — parse errors (E001), duplicate detection,
        // unresolved references (E003), and W061 reference cycles — grouped
        // by each diagnostic's own file. This is the same build_graph output
        // the CLI reports, so LSP and CLI agree byte for byte.
        for pd in &result.diagnostics {
            record(&state, uri, pd, &mut diags_by_file, &mut core_by_file);
        }

        // F1 syntax-only fast path (C4-07): when the edited file has parse
        // errors, the graph is broken — validator, registry checks, and Wasm
        // rule dispatch would evaluate garbage on every keystroke. Publish
        // the E001 layer only; full passes resume once it parses cleanly.
        let edited_has_parse_errors = result.diagnostics.iter().any(|pd| {
            pd.code == "E001"
                && pd
                    .span
                    .as_ref()
                    .map(|s| s.file.as_str() == file_path)
                    .unwrap_or(false)
        });

        if !edited_has_parse_errors {
            // The checks `specforge check` and watch run on a built graph:
            // core validation, registry checks and extension rules.
            let checks = specforge_emitter::compile::check_graph(
                state.graph(),
                &specforge_emitter::compile::GraphChecks {
                    spec_root: state.spec_root(),
                    kind_registry: state.kind_registry(),
                    field_registry: state.field_registry(),
                    rules: state.validation_patterns(),
                    runtime: state
                        .runtime()
                        .map(|r| r.as_ref() as &dyn specforge_wasm::WasmRuntime),
                },
            );
            for d in &checks {
                record(&state, uri, d, &mut diags_by_file, &mut core_by_file);
            }
        } // end syntax-only fast path gate

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

        // Keep what is published: code actions act on it.
        drop(state);
        let mut state = lock.write().await;
        for file_uri in diags_by_file.keys() {
            let core = core_by_file.remove(file_uri).unwrap_or_default();
            state.set_diagnostics(file_uri.as_str(), core);
        }

        diags_by_file
    }
}

/// Add `diagnostic` to the file its span names (`fallback` without a
/// span), both converted for publishing and as is.
fn record(
    state: &LspState,
    fallback: &Url,
    diagnostic: &specforge_common::Diagnostic,
    published: &mut std::collections::HashMap<Url, Vec<Diagnostic>>,
    core: &mut std::collections::HashMap<Url, Vec<specforge_common::Diagnostic>>,
) {
    let file_uri = diagnostic
        .span
        .as_ref()
        .map(|s| file_path_to_uri(s.file.as_str()))
        .unwrap_or_else(|| fallback.clone());
    let file_text = diagnostic
        .span
        .as_ref()
        .and_then(|s| file_content(state, s.file.as_str()));
    published
        .entry(file_uri.clone())
        .or_default()
        .push(diagnostic_to_lsp(diagnostic, file_text.as_deref()));
    core.entry(file_uri).or_default().push(diagnostic.clone());
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

/// Resolve the text of a file: open buffer first, then disk.
fn file_content(state: &LspState, file: &str) -> Option<String> {
    let uri = file_path_to_uri(file);
    if let Some(doc) = state.document(uri.as_str()) {
        return Some(doc.content().to_string());
    }
    std::fs::read_to_string(uri_to_file_path(&uri)).ok()
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
        // C4-10: editors can render this as a "view docs" link; the target
        // page is generated from the `specforge explain` catalog.
        code: Some(NumberOrString::String(diag.code.clone())),
        // C4-10: editors can render this as a "view docs" link; the target
        // page is generated from the `specforge explain` catalog.
        code_description: Url::parse(&format!(
            "https://github.com/specforge/specforge/blob/main/docs/diagnostics.md#{}",
            diag.code.to_lowercase()
        ))
        .ok()
        .map(|href| CodeDescription { href }),
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
        let resolved_spec_root = root
            .as_deref()
            .and_then(|r| {
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
            })
            .or_else(|| root.clone());
        // Every workspace folder is indexed (C4-04), not just the first.
        let mut roots: Vec<String> = params
            .workspace_folders
            .as_ref()
            .map(|folders| {
                folders
                    .iter()
                    .filter_map(|f| f.uri.to_file_path().ok())
                    .map(|p| p.to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();
        if let Some(root) = &root
            && !roots.contains(root)
        {
            roots.push(root.clone());
        }
        if let Some(spec_root) = &resolved_spec_root {
            self.state
                .write()
                .await
                .set_spec_root(std::path::PathBuf::from(spec_root));
        }
        *self.spec_root.lock().await = resolved_spec_root;
        *self.workspace_roots.lock().await = roots;
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

        // Indexing, registry loading, and the open-document re-diagnose all
        // move to a background task with workDone progress (C4-04):
        // `initialized` returns immediately so the session stays responsive.
        let roots = self.workspace_roots.lock().await.clone();
        let spec_root = self.spec_root.lock().await.clone();
        let client = self.client.clone();
        let state = Arc::clone(&self.state);
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

            // Load extension registries from specforge.json before indexing
            if let Some(root) = roots.first() {
                let ext_count = Self::load_registries_static(&state, root).await;
                if ext_count > 0 {
                    client
                        .log_message(
                            MessageType::INFO,
                            format!("specforge-lsp: loaded {ext_count} extension(s)"),
                        )
                        .await;
                }
            }

            // spec_root (from specforge.json) narrows the walk; otherwise
            // index every workspace folder (C4-04).
            let index_roots: Vec<String> = match spec_root {
                Some(spec_root) => vec![spec_root],
                None => roots,
            };
            if index_roots.is_empty() {
                client
                    .log_message(MessageType::INFO, "specforge-lsp initialized (no root_uri)")
                    .await;
                client
                    .send_notification::<tower_lsp::lsp_types::notification::Progress>(
                        ProgressParams {
                            token,
                            value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                                WorkDoneProgressEnd { message: None },
                            )),
                        },
                    )
                    .await;
                return;
            }
            let count = Self::index_roots_static(&state, &index_roots).await;
            client
                .log_message(
                    MessageType::INFO,
                    format!(
                        "specforge-lsp: indexed {count} .spec files from {}",
                        index_roots.join(", ")
                    ),
                )
                .await;

            // Re-diagnose all already-open documents now that the full graph
            // is populated.  Without this, didOpen diagnostics that raced
            // against indexing would show stale E001 errors for cross-file
            // references that hadn't been indexed yet.
            let open_docs: Vec<(Url, String, Option<i32>)> = {
                let state = state.read().await;
                state
                    .open_uris()
                    .into_iter()
                    .filter_map(|uri_str| {
                        let uri = Url::parse(uri_str).ok()?;
                        let doc = state.document(uri_str)?;
                        Some((uri, doc.content().to_string(), doc.version()))
                    })
                    .collect()
            };
            for (uri, content, version) in open_docs {
                let diags_by_file = Self::parse_and_update(&state, &uri, &content).await;
                for (file_uri, diags) in diags_by_file {
                    client.publish_diagnostics(file_uri, diags, version).await;
                }
            }

            client
                .send_notification::<tower_lsp::lsp_types::notification::Progress>(ProgressParams {
                    token,
                    value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(
                        WorkDoneProgressEnd {
                            message: Some(format!("{count} files")),
                        },
                    )),
                })
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

        let diags_by_file = Self::parse_and_update(&self.state, &uri, &text).await;
        for (file_uri, diags) in diags_by_file {
            self.client
                .publish_diagnostics(file_uri, diags, Some(version))
                .await;
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
            if let Some(doc) = state.document_mut(uri.as_str()) {
                doc.set_version(params.text_document.version);
            }
        }

        // Hand off to the serialized latest-wins worker (C4-03): the burst
        // is coalesced and one whole-graph pass runs at a time.
        let _ = self.update_tx.send(uri);
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

            // Extension configuration or plugin artifact changed: reload the
            // runtime + registries, reindex, and republish (hardening-plan
            // H4 / R-5).
            if file_path.ends_with("specforge.json") || file_path.ends_with(".wasm") {
                let root_dir = self.root_dir.lock().await.clone();
                if let Some(root) = root_dir {
                    let ext_count = Self::load_registries_static(&self.state, &root).await;
                    self.client
                        .log_message(
                            MessageType::INFO,
                            format!(
                                "specforge-lsp: extension environment changed, reloaded {ext_count} extension(s)"
                            ),
                        )
                        .await;
                    let files = Self::index_roots_static(&self.state, &[root]).await;
                    let _ = files;
                }
                continue;
            }

            // Only handle .spec files
            if !file_path.ends_with(".spec") {
                continue;
            }

            match change.typ {
                FileChangeType::DELETED => {
                    // Remove the file through the shared pipeline (nodes,
                    // edges, cached parse, and import-DAG entries), then
                    // republish diagnostics for everything affected.
                    // The pipeline update does synchronous fs reads (the
                    // invalidation set is re-read from disk), so it runs on
                    // the blocking pool: take the pipeline out (brief write
                    // lock), compute without any lock held, put it back
                    // (C4-03). Async workers are never blocked on std::fs.
                    let mut pipeline = {
                        let mut st = self.state.write().await;
                        st.take_pipeline()
                    };
                    let joined = tokio::task::spawn_blocking(move || {
                        let result = pipeline.update_open_file(&file_path, None, |f: &str| {
                            std::fs::read_to_string(f).ok()
                        });
                        (pipeline, result)
                    })
                    .await;
                    let (pipeline, result) = match joined {
                        Ok(pair) => pair,
                        Err(e) => {
                            self.client
                                .log_message(
                                    MessageType::ERROR,
                                    format!("specforge-lsp: deletion reparse failed: {e}"),
                                )
                                .await;
                            continue;
                        }
                    };
                    {
                        let mut st = self.state.write().await;
                        st.set_pipeline(pipeline);
                    }

                    let state = self.state.read().await;

                    let mut diags_by_file: std::collections::HashMap<Url, Vec<Diagnostic>> =
                        std::collections::HashMap::new();
                    let mut core_by_file: std::collections::HashMap<
                        Url,
                        Vec<specforge_common::Diagnostic>,
                    > = std::collections::HashMap::new();

                    // Publish empty diagnostics for the deleted file (clears stale squiggles)
                    diags_by_file.insert(uri.clone(), vec![]);

                    // Pipeline diagnostics for surviving files
                    for file in &result.changed_diagnostic_files {
                        let file_uri = file_path_to_uri(file);
                        diags_by_file.entry(file_uri).or_default();
                        for d in state.pipeline().file_diagnostics(file) {
                            record(&state, uri, d, &mut diags_by_file, &mut core_by_file);
                        }
                    }

                    // Validator diagnostics over the post-deletion graph
                    let validator_diags = specforge_validator::validate(state.graph());
                    for vd in &validator_diags {
                        record(&state, uri, vd, &mut diags_by_file, &mut core_by_file);
                    }

                    // Ensure all known files get an entry (clears stale diagnostics)
                    let known_files: Vec<Sym> = state
                        .graph()
                        .nodes()
                        .iter()
                        .map(|n| n.source_span.file)
                        .collect();
                    for file in &known_files {
                        diags_by_file
                            .entry(file_path_to_uri(file.as_str()))
                            .or_default();
                    }
                    drop(state);
                    {
                        let mut state = self.state.write().await;
                        for file_uri in diags_by_file.keys() {
                            let core = core_by_file.remove(file_uri).unwrap_or_default();
                            state.set_diagnostics(file_uri.as_str(), core);
                        }
                    }
                    for (file_uri, diags) in diags_by_file {
                        self.client.publish_diagnostics(file_uri, diags, None).await;
                    }
                }
                _ => {
                    // Created or Changed — re-read from disk (blocking pool)
                    // and update graph through the shared recompute path.
                    let content = {
                        let file_path = file_path.clone();
                        tokio::task::spawn_blocking(move || {
                            std::fs::read_to_string(&file_path).ok()
                        })
                        .await
                        .unwrap_or(None)
                    };
                    if let Some(content) = content {
                        let version = {
                            let state = self.state.read().await;
                            state.document(uri.as_str()).and_then(|d| d.version())
                        };
                        let diags_by_file =
                            Self::parse_and_update(&self.state, uri, &content).await;
                        for (file_uri, diags) in diags_by_file {
                            self.client
                                .publish_diagnostics(file_uri, diags, version)
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
            let resolved = self.spec_root.lock().await;
            if let Some(spec_root) = resolved.as_deref() {
                let span = goto_import_definition(import_path, spec_root);
                return Ok(span.map(|s| {
                    let location = source_span_to_location(&s);
                    GotoDefinitionResponse::Scalar(Location {
                        uri: location.uri,
                        range: span_range_for(&state, &s),
                    })
                }));
            }
        }

        let word = match word_at_position(&content, pos.line as usize, pos.character as usize) {
            Some(w) => w,
            None => return Ok(None),
        };

        let span = go_to_definition(state.graph(), &word);
        Ok(span.map(|s| {
            let location = source_span_to_location(&s);
            GotoDefinitionResponse::Scalar(Location {
                uri: location.uri,
                range: span_range_for(&state, &s),
            })
        }))
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
        Ok(Some(
            refs.iter()
                .map(|s| {
                    let location = source_span_to_location(s);
                    Location {
                        uri: location.uri,
                        range: span_range_for(&state, s),
                    }
                })
                .collect(),
        ))
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
        let edits = match specforge_graph::rename::identifier_edits(
            state.graph(),
            &word,
            &new_name,
            |file| {
                state
                    .document(file_path_to_uri(file).as_str())
                    .map(|doc| doc.content().to_string())
                    .or_else(|| std::fs::read_to_string(file).ok())
            },
        ) {
            Some(e) => e,
            None => return Ok(None),
        };

        let mut changes: std::collections::HashMap<Url, Vec<TextEdit>> =
            std::collections::HashMap::new();
        for edit in edits {
            let file_uri = file_path_to_uri(&edit.file);
            let line_idx = edit.line.saturating_sub(1); // 1-indexed -> 0-indexed
            let line_text = state
                .document(file_uri.as_str())
                .map(|doc| doc.content().to_string())
                .or_else(|| std::fs::read_to_string(&edit.file).ok())
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
        let file_path = uri_to_file_path(&uri);
        let state = self.state.read().await;
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
                let file_uri = file_path_to_uri(&a.file);
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
        let file_path = uri_to_file_path(&uri);

        let state = self.state.read().await;
        let symbols = document_symbols(state.graph(), &file_path);

        if symbols.is_empty() {
            return Ok(None);
        }

        // C3-09: the file's own text is available — convert span byte
        // columns to UTF-16 so symbol ranges survive non-ASCII prefixes.
        let file_text = file_content(&state, &file_path);

        let kind_reg = state.kind_registry();
        #[allow(deprecated)]
        let lsp_symbols: Vec<SymbolInformation> = symbols
            .into_iter()
            .map(|s| SymbolInformation {
                name: s.id,
                kind: symbol_kind_from_entity(&s.kind, kind_reg),
                tags: None,
                deprecated: None,
                location: match &file_text {
                    Some(content) => {
                        let lsp = source_span_to_lsp_range_with_text(&s.span, content);
                        Location {
                            uri: file_path_to_uri(&file_path),
                            range: Range {
                                start: Position {
                                    line: lsp.start_line,
                                    character: lsp.start_col,
                                },
                                end: Position {
                                    line: lsp.end_line,
                                    character: lsp.end_col,
                                },
                            },
                        }
                    }
                    None => source_span_to_location(&s.span),
                },
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
            .map(|s| {
                // Convert graph byte columns to UTF-16 against the file text
                // when the file is readable; byte passthrough otherwise.
                let file_uri = file_path_to_uri(s.span.file.as_str());
                let text = state
                    .document(file_uri.as_str())
                    .map(|doc| doc.content().to_string())
                    .or_else(|| std::fs::read_to_string(s.span.file.as_str()).ok());
                let location = match &text {
                    Some(content) => {
                        let lsp = source_span_to_lsp_range_with_text(&s.span, content);
                        Location {
                            uri: file_uri,
                            range: Range {
                                start: Position {
                                    line: lsp.start_line,
                                    character: lsp.start_col,
                                },
                                end: Position {
                                    line: lsp.end_line,
                                    character: lsp.end_col,
                                },
                            },
                        }
                    }
                    None => source_span_to_location(&s.span),
                };
                SymbolInformation {
                    name: s.id,
                    kind: symbol_kind_from_entity(&s.kind, kind_reg),
                    tags: None,
                    deprecated: None,
                    location,
                    container_name: Some(s.kind),
                }
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

        let lsp_diags: Vec<Diagnostic> = diags
            .iter()
            .map(|d| diagnostic_to_lsp(d, Some(&content)))
            .collect();
        if !lsp_diags.is_empty() {
            self.client
                .publish_diagnostics(uri.clone(), lsp_diags, None)
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

        let lsp_diags: Vec<Diagnostic> = diags
            .iter()
            .map(|d| diagnostic_to_lsp(d, Some(&content)))
            .collect();
        if !lsp_diags.is_empty() {
            self.client
                .publish_diagnostics(uri.clone(), lsp_diags, None)
                .await;
        }

        Ok(Some(formatter_edits_to_lsp(edits, &content)))
    }
}
