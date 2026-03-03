use std::collections::HashMap;

use regex::Regex;

use crate::file_graph::FileGraph;
use crate::symbol_table::{Declaration, SymbolTable};
use specforge_common::{
    CompilerConfig, Diagnostic, DiagnosticBag, EdgeType, EntityId, EntityKind, FieldValue,
    FormatVersion, GenConfig, Module, NamingStyle, ResultStyle, SourceSpan, ValidationCode,
};
use specforge_parser::{AstEntity, SpecFile};

/// Maximum allowed length for a scheme ref identifier part.
const MAX_IDENTIFIER_LENGTH: usize = 256;

/// Pre-compiled regex patterns for provider identifier validation.
struct CompiledPatterns {
    patterns: HashMap<String, HashMap<String, Regex>>,
}

impl CompiledPatterns {
    /// Compile patterns from config. Invalid regex strings are silently skipped.
    fn from_config(config: &CompilerConfig) -> Self {
        let mut patterns = HashMap::new();
        for (scheme, kind_patterns) in &config.provider_id_patterns {
            let mut compiled = HashMap::new();
            for (kind, pat) in kind_patterns {
                if let Ok(re) = Regex::new(pat) {
                    compiled.insert(kind.clone(), re);
                }
            }
            if !compiled.is_empty() {
                patterns.insert(scheme.clone(), compiled);
            }
        }
        CompiledPatterns { patterns }
    }

    /// Get the compiled regex for a scheme/kind pair, if any.
    fn get(&self, scheme: &str, kind: &str) -> Option<&Regex> {
        self.patterns.get(scheme).and_then(|m| m.get(kind))
    }
}

/// Check if a scheme or kind part is a valid lowercase identifier: `[a-z][a-z0-9_]*`.
fn is_valid_scheme_part(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// The result of resolving a project.
#[derive(Debug)]
pub struct ResolvedProject {
    pub files: Vec<SpecFile>,
    pub symbols: SymbolTable,
    pub file_graph: FileGraph,
    pub diagnostics: DiagnosticBag,
    pub config: CompilerConfig,
}

/// Resolve a set of parsed spec files into a `ResolvedProject`.
///
/// Steps:
/// 1. Build file graph from imports
/// 2. Detect import cycles (E003)
/// 3. Build symbol table, detect duplicates (E002)
/// 4. Extract compiler config from spec root (or use `external_config` if provided)
/// 5. Resolve references (E001), with soft-ref logic for I004/I005
///
/// When `external_config` is `Some`, it is used directly and the spec root block
/// extraction is skipped. This is the path taken when `specforge.json` provides
/// the project configuration.
pub fn resolve(files: Vec<SpecFile>, spec_root: &str) -> ResolvedProject {
    resolve_with_config(files, spec_root, None)
}

/// Like [`resolve`], but accepts an optional pre-built `CompilerConfig`.
pub fn resolve_with_config(
    files: Vec<SpecFile>,
    spec_root: &str,
    external_config: Option<CompilerConfig>,
) -> ResolvedProject {
    let mut diagnostics = DiagnosticBag::new();
    let mut file_graph = FileGraph::new();
    let mut symbols = SymbolTable::new();

    // Step 1: Build file graph
    for file in &files {
        file_graph.add_file(&file.path);
        for import in &file.imports {
            let target_path = resolve_import_path(spec_root, &import.path);
            file_graph.add_import(&file.path, &target_path);
        }
    }

    // Step 2: Detect import cycles
    for cycle in file_graph.find_cycles() {
        let cycle_str = cycle.join(" -> ");
        let span = SourceSpan::file_start(cycle.first().map(|s| s.as_str()).unwrap_or("unknown"));
        diagnostics.push(
            Diagnostic::new(
                ValidationCode::E003,
                span,
                format!("circular import: {cycle_str}"),
            )
            .with_help("break the cycle by moving shared declarations to a separate file"),
        );
    }

    // Step 3: Build symbol table
    for file in &files {
        for entity in &file.entities {
            let decl = Declaration {
                id: entity.id.clone(),
                kind: entity.kind.clone(),
                file: file.path.clone(),
                span: entity.span.clone(),
            };
            if let Some(existing) = symbols.insert(decl) {
                diagnostics.push(
                    Diagnostic::new(
                        ValidationCode::E002,
                        entity.span.clone(),
                        format!("duplicate entity name `{}`", entity.id.raw()),
                    )
                    .with_label(
                        existing.span.clone(),
                        "first declared here".to_string(),
                    ),
                );
            }
        }
    }

    // Step 4: Extract config from spec root (or use external config)
    let mut config = match external_config {
        Some(cfg) => cfg,
        None => extract_config(&files, &symbols),
    };

    // Step 4b: Collect custom entity definitions
    for file in &files {
        for def in &file.custom_defs {
            if config.custom_entities.contains_key(&def.name) {
                diagnostics.push(
                    Diagnostic::new(
                        ValidationCode::E002,
                        SourceSpan::file_start(&file.path),
                        format!("duplicate custom entity definition `{}`", def.name),
                    )
                    .with_help("each `define` name must be unique across all files"),
                );
            } else {
                config.custom_entities.insert(def.name.clone(), def.clone());
            }
        }
    }

    // Step 5: Resolve references
    let compiled = CompiledPatterns::from_config(&config);
    for file in &files {
        for entity in &file.entities {
            resolve_entity_refs(entity, &symbols, &config, &compiled, &mut diagnostics);
        }
    }

    ResolvedProject {
        files,
        symbols,
        file_graph,
        diagnostics,
        config,
    }
}

fn resolve_import_path(spec_root: &str, import_path: &str) -> String {
    format!("{spec_root}/{import_path}.spec")
}

fn extract_config(files: &[SpecFile], symbols: &SymbolTable) -> CompilerConfig {
    // Find the spec root entity
    for file in files {
        for entity in &file.entities {
            if entity.kind == EntityKind::Spec {
                return config_from_spec_entity(entity);
            }
        }
    }
    // No spec root found — use defaults
    let _ = symbols;
    CompilerConfig::default()
}

fn config_from_spec_entity(entity: &AstEntity) -> CompilerConfig {
    let name = entity
        .title
        .clone()
        .unwrap_or_else(|| "unnamed".to_string());

    let version = match entity.fields.get("version") {
        Some(FieldValue::String(s)) => s.parse::<FormatVersion>().unwrap_or(FormatVersion::CURRENT),
        _ => FormatVersion::CURRENT,
    };

    let namespace = match entity.fields.get("namespace") {
        Some(FieldValue::String(s)) => Some(s.clone()),
        _ => None,
    };

    let display_prefix = match entity.fields.get("display_prefix") {
        Some(FieldValue::String(s)) => Some(s.clone()),
        _ => None,
    };

    let mut plugins = Vec::new();
    if let Some(FieldValue::StringList(list)) = entity.fields.get("plugins") {
        for pkg in list {
            if let Some(module) = Module::from_package_name(pkg) {
                plugins.push(module);
            }
        }
    }

    let mut personas = Vec::new();
    let mut surfaces = Vec::new();
    let mut provider_schemes = Vec::new();
    let mut provider_kinds = std::collections::HashMap::new();
    let mut provider_id_patterns: std::collections::HashMap<String, std::collections::HashMap<String, String>> = std::collections::HashMap::new();
    let mut gen_configs = Vec::new();

    for (key, value) in entity.fields.iter() {
        if let Some(persona_id) = key.strip_prefix("persona:") {
            if let FieldValue::Block(fields) = value {
                let display = match fields.get("display_name") {
                    Some(FieldValue::String(s)) => s.clone(),
                    _ => persona_id.to_string(),
                };
                personas.push((persona_id.to_string(), display));
            }
        } else if let Some(surface_id) = key.strip_prefix("surface:") {
            if let FieldValue::Block(fields) = value {
                let display = match fields.get("display_name") {
                    Some(FieldValue::String(s)) => s.clone(),
                    _ => surface_id.to_string(),
                };
                surfaces.push((surface_id.to_string(), display));
            }
        } else if let Some(gen_name) = key.strip_prefix("gen:") {
            if let FieldValue::Block(fields) = value {
                let out = match fields.get("out") {
                    Some(FieldValue::String(s)) => s.clone(),
                    _ => format!("generated/{gen_name}"),
                };
                let result_style = match fields.get("result") {
                    Some(FieldValue::String(s)) | Some(FieldValue::Enum(s)) => {
                        ResultStyle::from_str_opt(s).unwrap_or_default()
                    }
                    _ => ResultStyle::default(),
                };
                let readonly = match fields.get("readonly") {
                    Some(FieldValue::Bool(b)) => *b,
                    _ => false,
                };
                let naming = match fields.get("naming") {
                    Some(FieldValue::String(s)) | Some(FieldValue::Enum(s)) => {
                        NamingStyle::from_str_opt(s).unwrap_or_default()
                    }
                    _ => NamingStyle::default(),
                };
                let tests = match fields.get("tests") {
                    Some(FieldValue::String(s)) => Some(s.clone()),
                    _ => None,
                };
                // Collect extra fields for external generators
                let mut extra = std::collections::HashMap::new();
                for (fk, fv) in fields.iter() {
                    if !matches!(fk, "out" | "result" | "readonly" | "naming" | "tests") {
                        if let FieldValue::String(s) = fv {
                            extra.insert(fk.to_string(), s.clone());
                        }
                    }
                }
                gen_configs.push(GenConfig {
                    name: gen_name.to_string(),
                    out,
                    result_style,
                    readonly,
                    naming,
                    tests,
                    extra,
                });
            }
        }
        if key == "providers" {
            if let FieldValue::Block(providers) = value {
                for (scheme, provider_value) in providers.iter() {
                    provider_schemes.push(scheme.to_string());
                    // Extract kinds and id_patterns from provider block if present
                    if let FieldValue::Block(inner) = provider_value {
                        if let Some(FieldValue::ReferenceList(refs)) = inner.get("kinds") {
                            let kinds: Vec<String> =
                                refs.iter().map(|r| r.raw().to_string()).collect();
                            provider_kinds.insert(scheme.to_string(), kinds);
                        }
                        if let Some(FieldValue::Block(patterns)) = inner.get("id_patterns") {
                            let mut kind_patterns = std::collections::HashMap::new();
                            for (kind_name, pattern_value) in patterns.iter() {
                                if let FieldValue::String(pat) = pattern_value {
                                    kind_patterns.insert(kind_name.to_string(), pat.clone());
                                }
                            }
                            if !kind_patterns.is_empty() {
                                provider_id_patterns.insert(scheme.to_string(), kind_patterns);
                            }
                        }
                    }
                }
            }
        }
    }

    CompilerConfig {
        name,
        version,
        namespace,
        display_prefix,
        plugins,
        provider_schemes,
        provider_kinds,
        provider_id_patterns,
        personas,
        surfaces,
        strict: false,
        gen_configs,
        coverage: specforge_common::CoverageConfig::default(),
        custom_entities: std::collections::HashMap::new(),
    }
}

fn resolve_entity_refs(
    entity: &AstEntity,
    symbols: &SymbolTable,
    config: &CompilerConfig,
    compiled: &CompiledPatterns,
    diagnostics: &mut DiagnosticBag,
) {
    // Check entity ID itself if it's a scheme ref (e.g., ref gh.issue:42)
    if entity.id.is_scheme_ref() {
        check_reference(&entity.id, &entity.span, "", symbols, config, compiled, diagnostics);
    }

    for (field_name, value) in entity.fields.iter() {
        // Only resolve references for fields that create edges in the graph.
        // Fields without an edge type mapping (e.g., contract, status)
        // contain data values, not entity references.
        if EdgeType::from_field_name(field_name).is_none() {
            // Still check scheme refs (gh.issue:42) in any field
            if let FieldValue::Reference(ref_id) = value {
                if ref_id.scheme().is_some() {
                    check_reference(ref_id, &entity.span, field_name, symbols, config, compiled, diagnostics);
                }
            }
            continue;
        }

        match value {
            FieldValue::Reference(ref_id) => {
                check_reference(ref_id, &entity.span, field_name, symbols, config, compiled, diagnostics);
            }
            FieldValue::ReferenceList(refs) => {
                for ref_id in refs {
                    check_reference(ref_id, &entity.span, field_name, symbols, config, compiled, diagnostics);
                }
            }
            _ => {}
        }
    }
}

fn check_reference(
    ref_id: &EntityId,
    span: &SourceSpan,
    field_name: &str,
    symbols: &SymbolTable,
    config: &CompilerConfig,
    compiled: &CompiledPatterns,
    diagnostics: &mut DiagnosticBag,
) {
    let raw = ref_id.raw();

    // Scheme refs are validated by providers, not symbol table (I005, E012, E011)
    if let Some(scheme) = ref_id.scheme() {
        if let EntityId::SchemeRef {
            kind, identifier, ..
        } = ref_id
        {
            // Format validation: scheme/kind must be lowercase identifiers
            if !is_valid_scheme_part(scheme) {
                diagnostics.push(
                    Diagnostic::new(
                        ValidationCode::E011,
                        span.clone(),
                        format!("invalid scheme format `{scheme}` — must match [a-z][a-z0-9_]*"),
                    )
                    .with_help("scheme names must be lowercase (e.g., `gh`, `jira`)"),
                );
                return;
            }
            if !is_valid_scheme_part(kind) {
                diagnostics.push(
                    Diagnostic::new(
                        ValidationCode::E011,
                        span.clone(),
                        format!(
                            "invalid kind format `{kind}` — must match [a-z][a-z0-9_]*"
                        ),
                    )
                    .with_help("kind names must be lowercase (e.g., `issue`, `pr`)"),
                );
                return;
            }
            // Identifier length cap
            if identifier.len() > MAX_IDENTIFIER_LENGTH {
                diagnostics.push(
                    Diagnostic::new(
                        ValidationCode::E011,
                        span.clone(),
                        format!(
                            "ref identifier too long ({} chars, max {MAX_IDENTIFIER_LENGTH})",
                            identifier.len()
                        ),
                    )
                    .with_help("shorten the identifier or use a shorter alias"),
                );
                return;
            }
        }

        if !config.has_provider_scheme(scheme) {
            diagnostics.push(
                Diagnostic::new(
                    ValidationCode::I005,
                    span.clone(),
                    format!("unknown provider scheme `{scheme}`"),
                )
                .with_help(format!(
                    "install a provider that registers the `{scheme}` scheme"
                )),
            );
        } else if let EntityId::SchemeRef {
            kind, identifier, ..
        } = ref_id
        {
            // Scheme is known — check if the kind is valid (E012)
            if !config.has_provider_kind(scheme, kind) {
                let valid_kinds = config
                    .provider_kinds
                    .get(scheme)
                    .map(|ks| ks.join(", "))
                    .unwrap_or_default();
                diagnostics.push(
                    Diagnostic::new(
                        ValidationCode::E012,
                        span.clone(),
                        format!("unknown provider kind `{kind}` for scheme `{scheme}`"),
                    )
                    .with_help(format!("valid kinds for `{scheme}`: {valid_kinds}")),
                );
            } else if let Some(re) = compiled.get(scheme, kind) {
                // Kind is valid — check identifier against pattern (E011)
                if !re.is_match(identifier) {
                    diagnostics.push(
                        Diagnostic::new(
                            ValidationCode::E011,
                            span.clone(),
                            format!(
                                "identifier `{identifier}` does not match pattern for `{scheme}.{kind}`"
                            ),
                        )
                        .with_help(format!("expected pattern: {}", re.as_str())),
                    );
                }
            }
        }
        return;
    }

    // Already in symbol table -> valid
    if symbols.contains(raw) {
        return;
    }

    // Field-name-based soft reference check (I004)
    // Use the field name to determine what entity kind is expected,
    // and check if that kind's module is installed.
    if let Some(edge_type) = EdgeType::from_field_name(field_name) {
        if let Some(target_kind) = edge_type.target_kind() {
            let target_module = target_kind.module();
            if target_module != Module::Core && !config.has_plugin(target_module) {
                // The target kind belongs to a module that isn't installed -> I004
                let plugin_name = match target_module {
                    Module::Product => "@specforge/product",
                    Module::Governance => "@specforge/governance",
                    Module::Core => unreachable!(),
                };
                diagnostics.push(
                    Diagnostic::new(
                        ValidationCode::I004,
                        span.clone(),
                        format!(
                            "unknown entity `{raw}` in field `{field_name}`"
                        ),
                    )
                    .with_help(format!(
                        "install {plugin_name} to enable {} validation",
                        target_kind.keyword()
                    )),
                );
                return;
            }
        }
    }

    // It's a dangling reference (E001) — find did-you-mean suggestions
    let suggestion = did_you_mean(raw, &symbols.all_ids());
    let mut diag = Diagnostic::new(
        ValidationCode::E001,
        span.clone(),
        format!("unresolved reference `{raw}`"),
    );
    if let Some(suggestion) = suggestion {
        diag = diag.with_help(format!("did you mean `{suggestion}`?"));
    }
    diagnostics.push(diag);
}

/// Find the closest match using Levenshtein distance.
fn did_you_mean<'a>(input: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let max_distance = 3;
    candidates
        .iter()
        .filter_map(|&candidate| {
            let dist = strsim::levenshtein(input, candidate);
            if dist <= max_distance && dist > 0 {
                Some((candidate, dist))
            } else {
                None
            }
        })
        .min_by_key(|(_, dist)| *dist)
        .map(|(candidate, _)| candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use specforge_parser::parse;

    fn resolve_source(sources: &[(&str, &str)]) -> ResolvedProject {
        let files: Vec<SpecFile> = sources
            .iter()
            .map(|(path, source)| parse(source, path))
            .collect();
        resolve(files, "spec")
    }

    #[test]
    fn resolve_valid_references() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
invariant data_persistence "Test" {
  guarantee """OK"""
}
behavior create_user "Test" {
  invariants [data_persistence]
  contract """OK"""
}
"#,
        )]);
        assert_eq!(project.diagnostics.error_count(), 0);
    }

    #[test]
    fn detect_duplicate_id() {
        let project = resolve_source(&[
            (
                "a.spec",
                r#"invariant data_persistence "First" { guarantee """OK""" }"#,
            ),
            (
                "b.spec",
                r#"invariant data_persistence "Second" { guarantee """OK""" }"#,
            ),
        ]);
        let errors: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::E002)
            .collect();
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn detect_dangling_reference() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
behavior create_user "Test" {
  invariants [nonexistent_inv]
  contract """OK"""
}
"#,
        )]);
        let errors: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::E001)
            .collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("nonexistent_inv"));
    }

    #[test]
    fn did_you_mean_suggestion() {
        let suggestion = did_you_mean(
            "data_persistance",
            &["data_persistence", "email_uniqueness", "create_user"],
        );
        assert!(suggestion.is_some());
        assert_eq!(suggestion.unwrap(), "data_persistence");
    }

    #[test]
    fn soft_ref_unknown_plugin_field() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
behavior create_user "Test" {
  adrs [use_postgresql]
  contract """OK"""
}
"#,
        )]);
        // No spec root -> core-only -> governance not installed -> I004
        let infos: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::I004)
            .collect();
        assert_eq!(infos.len(), 1);
        assert!(infos[0]
            .help
            .as_ref()
            .unwrap()
            .contains("@specforge/governance"));
    }

    #[test]
    fn cross_plugin_with_plugin_installed() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
spec "test" {
  version "1.0"
  plugins ["@specforge/governance"]
}
behavior create_user "Test" {
  adrs [use_postgresql]
  contract """OK"""
}
"#,
        )]);
        // Plugin installed, but use_postgresql not declared -> E001 (not I004)
        let errors: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::E001)
            .collect();
        assert_eq!(errors.len(), 1);
        let infos: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::I004)
            .collect();
        assert_eq!(infos.len(), 0);
    }

    #[test]
    fn detect_import_cycle() {
        let mut fg = FileGraph::new();
        fg.add_import("a.spec", "b.spec");
        fg.add_import("b.spec", "a.spec");
        let cycles = fg.find_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn extract_config_from_spec() {
        let project = resolve_source(&[(
            "specforge.spec",
            r#"
spec "specforge" {
  version "1.0"
  plugins ["@specforge/product", "@specforge/governance"]

  persona developer "Developer" {
    description "Writes specs"
  }

  surface cli "CLI" {
    type terminal
  }
}
"#,
        )]);
        assert_eq!(project.config.name, "specforge");
        assert_eq!(project.config.plugins.len(), 2);
        assert!(project.config.has_persona("developer"));
        assert!(project.config.has_surface("cli"));
    }

    #[test]
    fn e012_unknown_kind_on_known_scheme() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
spec "test" {
  version "1.0"
  providers {
    gh "test" {
      package "@specforge/gh"
      kinds [issue, pr]
    }
  }
}
ref gh.bogus:42 "Bad Kind"
"#,
        )]);
        let errors: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::E012)
            .collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("bogus"));
    }

    #[test]
    fn e012_valid_kind_no_diagnostic() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
spec "test" {
  version "1.0"
  providers {
    gh "test" {
      package "@specforge/gh"
      kinds [issue, pr]
    }
  }
}
ref gh.issue:42 "Valid Kind"
"#,
        )]);
        let errors: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::E012)
            .collect();
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn e012_no_kinds_registered_skips() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
spec "test" {
  version "1.0"
  providers {
    gh "test" {
      package "@specforge/gh"
    }
  }
}
ref gh.anything:42 "No Kinds Declared"
"#,
        )]);
        let errors: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::E012)
            .collect();
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn e011_invalid_identifier_format() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
spec "test" {
  version "1.0"
  providers {
    gh "test" {
      package "@specforge/gh"
      kinds [issue, pr]
      id_patterns {
        issue "^\d+$"
        pr    "^\d+$"
      }
    }
  }
}
ref gh.issue:abc "Non-numeric Issue ID"
"#,
        )]);
        let errors: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::E011)
            .collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("abc"));
        assert!(errors[0].message.contains("gh.issue"));
    }

    #[test]
    fn e011_valid_identifier_passes() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
spec "test" {
  version "1.0"
  providers {
    gh "test" {
      package "@specforge/gh"
      kinds [issue, pr]
      id_patterns {
        issue "^\d+$"
        pr    "^\d+$"
      }
    }
  }
}
ref gh.issue:42 "Valid Numeric ID"
"#,
        )]);
        let errors: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::E011)
            .collect();
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn e011_no_patterns_skips_check() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
spec "test" {
  version "1.0"
  providers {
    gh "test" {
      package "@specforge/gh"
      kinds [issue, pr]
    }
  }
}
ref gh.issue:abc "No Patterns Declared"
"#,
        )]);
        let errors: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::E011)
            .collect();
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn e011_scheme_format_uppercase() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
spec "test" {
  version "1.0"
  providers {
    gh "test" {
      package "@specforge/gh"
    }
  }
}
ref GH.ISSUE:42 "Uppercase Scheme"
"#,
        )]);
        let errors: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::E011)
            .collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("invalid scheme format"));
    }

    #[test]
    fn e011_identifier_too_long() {
        let long_id = "x".repeat(300);
        let source = format!(
            r#"
spec "test" {{
  version "1.0"
  providers {{
    gh "test" {{
      package "@specforge/gh"
      kinds [issue]
    }}
  }}
}}
ref gh.issue:{long_id} "Too Long ID"
"#
        );
        let project = resolve_source(&[("test.spec", &source)]);
        let errors: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::E011)
            .collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("too long"));
    }

    #[test]
    fn enforced_by_dangling_reference_produces_e001() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
invariant data_persistence "Test" {
  guarantee """OK"""
  enforced_by [nonexistent_enforcer]
}
"#,
        )]);
        let errors: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::E001)
            .collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("nonexistent_enforcer"));
    }

    #[test]
    fn enforced_by_valid_reference_no_error() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
behavior validate_input "Validate Input" {
  contract """OK"""
}
invariant data_persistence "Test" {
  guarantee """OK"""
  enforced_by [validate_input]
}
"#,
        )]);
        let errors: Vec<_> = project
            .diagnostics
            .iter()
            .filter(|d| d.code == ValidationCode::E001)
            .collect();
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn e011_extract_id_patterns_from_config() {
        let project = resolve_source(&[(
            "test.spec",
            r#"
spec "test" {
  version "1.0"
  providers {
    gh "test" {
      package "@specforge/gh"
      kinds [issue, pr]
      id_patterns {
        issue "^\d+$"
        pr    "^\d+$"
      }
    }
  }
}
"#,
        )]);
        assert_eq!(
            project.config.get_id_pattern("gh", "issue"),
            Some(r"^\d+$")
        );
        assert_eq!(project.config.get_id_pattern("gh", "pr"), Some(r"^\d+$"));
        assert_eq!(project.config.get_id_pattern("gh", "release"), None);
    }
}
