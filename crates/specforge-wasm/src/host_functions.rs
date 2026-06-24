use specforge_common::{Diagnostic, Severity};
use specforge_registry::{ManifestV2, SandboxPolicy};
use std::path::Path;

/// Call sites from which host functions can be invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallSite {
    Validator,
    Renderer,
    Provider,
    Parser,
    Collector,
    Analyzer,
}

/// Scope of graph queries an extension is allowed to perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryScope {
    All,
    Own,
    Explicit(Vec<String>),
}

/// Compute the query scope for an extension based on its manifest.
pub fn compute_extension_query_scope(manifest: &ManifestV2) -> QueryScope {
    match manifest.query_scope.as_deref() {
        Some("own") => QueryScope::Own,
        Some("all") | None => QueryScope::All,
        Some(other) => {
            // Parse comma-separated list of entity kind names
            let kinds: Vec<String> = other
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if kinds.is_empty() {
                QueryScope::All
            } else {
                QueryScope::Explicit(kinds)
            }
        }
    }
}

/// Parse a diagnostic emitted by a Wasm extension via host_emit_diagnostic.
/// Returns Ok(Diagnostic) on success, Err(Diagnostic) if the JSON is malformed.
pub fn host_emit_diagnostic(
    extension_name: &str,
    json_bytes: &[u8],
) -> Result<Diagnostic, Diagnostic> {
    serde_json::from_slice::<Diagnostic>(json_bytes).map_err(|e| Diagnostic {
        code: "E028".to_string(),
        severity: Severity::Error,
        message: format!(
            "extension '{}': host_emit_diagnostic received malformed JSON: {}",
            extension_name, e
        ),
        span: None,
        suggestion: Some("ensure the diagnostic JSON matches the Diagnostic schema".to_string()),
    })
}

/// Check if a host_read_file call is allowed by the sandbox policy.
/// The path must be under spec_root and must not escape via `..`.
/// Only Validator, Provider, and Parser call sites are allowed to read files.
pub fn host_read_file_check(
    extension_name: &str,
    path: &Path,
    spec_root: &Path,
    call_site: CallSite,
    policy: &SandboxPolicy,
) -> Result<(), Diagnostic> {
    match call_site {
        CallSite::Validator | CallSite::Provider | CallSite::Parser | CallSite::Analyzer => {}
        other => {
            return Err(Diagnostic {
                code: "E031".to_string(),
                severity: Severity::Error,
                message: format!(
                    "extension '{}': host_read_file not allowed from {:?} call site",
                    extension_name, other
                ),
                span: None,
                suggestion: None,
            });
        }
    }

    // Filesystem access must be enabled
    if policy.file_system_access != Some(true) {
        return Err(Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': host_read_file denied — file_system_access is disabled",
                extension_name
            ),
            span: None,
            suggestion: None,
        });
    }

    // Path must not contain `..` components
    let path_str = path.to_string_lossy();
    if path_str.contains("..") {
        return Err(Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': host_read_file denied — path contains '..' escape: {}",
                extension_name,
                path.display()
            ),
            span: None,
            suggestion: None,
        });
    }

    // Resolve symlinks via canonicalize to prevent sandbox escape.
    // If canonicalize succeeds (file exists), use the resolved paths for comparison.
    // If it fails (file doesn't exist), fall back to the lexical starts_with check
    // since a nonexistent file will fail to read anyway.
    let (effective_path, effective_root) = match (
        std::fs::canonicalize(path),
        std::fs::canonicalize(spec_root),
    ) {
        (Ok(canon_path), Ok(canon_root)) => (canon_path, canon_root),
        _ => (path.to_path_buf(), spec_root.to_path_buf()),
    };

    // Path must be under spec_root
    if !effective_path.starts_with(&effective_root) {
        return Err(Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': host_read_file denied — path '{}' is not under spec_root '{}'",
                extension_name,
                path.display(),
                spec_root.display()
            ),
            span: None,
            suggestion: None,
        });
    }

    Ok(())
}

/// Code file extensions that are never allowed in renderer output.
const CODE_EXTENSIONS: &[&str] = &[
    ".rs", ".py", ".js", ".ts", ".go", ".java", ".c", ".cpp", ".rb", ".swift", ".kt", ".sh",
];

/// Check if a host_emit_file call is allowed by the sandbox policy.
/// Output must go to the output directory and must not be a code extension.
pub fn host_emit_file_check(
    extension_name: &str,
    path: &Path,
    output_dir: &Path,
    policy: &SandboxPolicy,
) -> Result<(), Diagnostic> {
    // Path must be under output_dir
    if !path.starts_with(output_dir) {
        return Err(Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': host_emit_file denied — path '{}' is not under output dir '{}'",
                extension_name,
                path.display(),
                output_dir.display()
            ),
            span: None,
            suggestion: None,
        });
    }

    // Check file extension against code blocklist
    if let Some(ext_os) = path.extension() {
        let ext = format!(".{}", ext_os.to_string_lossy());
        if CODE_EXTENSIONS.contains(&ext.as_str()) {
            return Err(Diagnostic {
                code: "E031".to_string(),
                severity: Severity::Error,
                message: format!(
                    "extension '{}': host_emit_file denied — code file extension '{}' is blocked",
                    extension_name, ext
                ),
                span: None,
                suggestion: None,
            });
        }

        // Check against policy allowlist
        if !policy.allowed_output_extensions.is_empty()
            && !policy.allowed_output_extensions.contains(&ext)
        {
            return Err(Diagnostic {
                code: "E031".to_string(),
                severity: Severity::Error,
                message: format!(
                    "extension '{}': host_emit_file denied — extension '{}' not in allowed_output_extensions",
                    extension_name, ext
                ),
                span: None,
                suggestion: None,
            });
        }
    }

    Ok(())
}

/// Check if a host_http_get call is allowed by the sandbox policy.
/// Only Providers can make HTTP requests, and the domain must be allowlisted.
pub fn host_http_get_check(
    extension_name: &str,
    domain: &str,
    call_site: CallSite,
    policy: &SandboxPolicy,
) -> Result<(), Diagnostic> {
    // Call-site restriction: only providers can make HTTP requests
    if call_site != CallSite::Provider {
        return Err(Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': host_http_get not allowed from {:?} call site",
                extension_name, call_site
            ),
            span: None,
            suggestion: None,
        });
    }

    // Network access must be enabled
    if policy.network_access != Some(true) {
        return Err(Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': host_http_get denied — network_access is disabled",
                extension_name
            ),
            span: None,
            suggestion: None,
        });
    }

    // Domain must be in allowlist (if allowlist is non-empty)
    if !policy.allowed_domains.is_empty()
        && !policy.allowed_domains.iter().any(|d| d == domain)
    {
        return Err(Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': host_http_get denied — domain '{}' not in allowed_domains",
                extension_name, domain
            ),
            span: None,
            suggestion: None,
        });
    }

    Ok(())
}

/// Check if a graph node addition is allowed.
/// Only Parser call sites can add nodes, and the entity kind must be declared.
pub fn host_add_graph_node_check(
    extension_name: &str,
    entity_kind: &str,
    call_site: CallSite,
    declared_kinds: &[String],
) -> Result<(), Diagnostic> {
    if call_site != CallSite::Parser {
        return Err(Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': host_add_graph_node not allowed from {:?} call site",
                extension_name, call_site
            ),
            span: None,
            suggestion: None,
        });
    }

    if !declared_kinds.iter().any(|k| k == entity_kind) {
        return Err(Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': entity kind '{}' is not declared in manifest",
                extension_name, entity_kind
            ),
            span: None,
            suggestion: Some(format!(
                "add '{}' to entity_kinds in the extension manifest",
                entity_kind
            )),
        });
    }

    Ok(())
}

/// Check if a graph edge addition is allowed.
/// Only Parser call sites can add edges, the label must be declared,
/// and both source/target nodes must exist.
pub fn host_add_graph_edge_check(
    extension_name: &str,
    edge_label: &str,
    source_id: &str,
    target_id: &str,
    call_site: CallSite,
    declared_edge_labels: &[String],
    known_node_ids: &std::collections::HashSet<String>,
) -> Result<(), Diagnostic> {
    if call_site != CallSite::Parser {
        return Err(Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': host_add_graph_edge not allowed from {:?} call site",
                extension_name, call_site
            ),
            span: None,
            suggestion: None,
        });
    }

    if !declared_edge_labels.iter().any(|l| l == edge_label) {
        return Err(Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': edge label '{}' is not declared in manifest",
                extension_name, edge_label
            ),
            span: None,
            suggestion: Some(format!(
                "add '{}' to edge_types in the extension manifest",
                edge_label
            )),
        });
    }

    if !known_node_ids.contains(source_id) {
        return Err(Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': source node '{}' does not exist in the graph",
                extension_name, source_id
            ),
            span: None,
            suggestion: None,
        });
    }

    if !known_node_ids.contains(target_id) {
        return Err(Diagnostic {
            code: "E031".to_string(),
            severity: Severity::Error,
            message: format!(
                "extension '{}': target node '{}' does not exist in the graph",
                extension_name, target_id
            ),
            span: None,
            suggestion: None,
        });
    }

    Ok(())
}

/// Filter a graph JSON representation based on a QueryScope.
/// Returns the full graph for `All`, or filters entities and edges for restricted scopes.
pub fn filter_graph_by_query_scope(
    graph_json: &serde_json::Value,
    scope: &QueryScope,
) -> serde_json::Value {
    match scope {
        QueryScope::All => graph_json.clone(),
        QueryScope::Own | QueryScope::Explicit(_) => {
            let allowed_kinds: Vec<&str> = match scope {
                QueryScope::Own => vec![], // Own with no kinds = empty
                QueryScope::Explicit(kinds) => kinds.iter().map(|s| s.as_str()).collect(),
                _ => unreachable!(),
            };

            let mut result = graph_json.clone();

            // Filter entities array
            let kept_ids: std::collections::HashSet<String> =
                if let Some(entities) = graph_json.get("entities").and_then(|v| v.as_array()) {
                    let filtered: Vec<serde_json::Value> = entities
                        .iter()
                        .filter(|e| {
                            e.get("kind")
                                .and_then(|k| k.as_str())
                                .is_some_and(|k| allowed_kinds.contains(&k))
                        })
                        .cloned()
                        .collect();
                    let ids: std::collections::HashSet<String> = filtered
                        .iter()
                        .filter_map(|e| e.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
                        .collect();
                    result["entities"] = serde_json::Value::Array(filtered);
                    ids
                } else {
                    std::collections::HashSet::new()
                };

            // Filter edges to only those where both source and target are in kept entities
            if let Some(edges) = graph_json.get("edges").and_then(|v| v.as_array()) {
                let filtered_edges: Vec<serde_json::Value> = edges
                    .iter()
                    .filter(|e| {
                        let src = e.get("source").and_then(|v| v.as_str()).unwrap_or("");
                        let tgt = e.get("target").and_then(|v| v.as_str()).unwrap_or("");
                        kept_ids.contains(src) && kept_ids.contains(tgt)
                    })
                    .cloned()
                    .collect();
                result["edges"] = serde_json::Value::Array(filtered_edges);
            }

            result
        }
    }
}

/// Permission matrix: which host functions are allowed per call site.
pub fn is_host_function_allowed(call_site: CallSite, function_name: &str) -> bool {
    match function_name {
        "host_emit_diagnostic" => true, // all call sites
        "host_read_file" => matches!(
            call_site,
            CallSite::Validator | CallSite::Provider | CallSite::Parser | CallSite::Analyzer
        ),
        "host_emit_file" => matches!(call_site, CallSite::Renderer | CallSite::Collector),
        "host_http_get" => matches!(call_site, CallSite::Provider),
        "host_query_graph" => true, // all call sites (scope-limited)
        "host_add_graph_node" | "host_add_graph_edge" => {
            matches!(call_site, CallSite::Parser)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::default_manifest;

    // -- compute_extension_query_scope --

    // B:provide_host_function_query_graph, B:compute_extension_query_scope — verify unit "defaults to All when query_scope is absent"
    #[test]
    fn test_query_scope_defaults_to_all() {
        let manifest = default_manifest();
        assert_eq!(compute_extension_query_scope(&manifest), QueryScope::All);
    }

    // B:compute_extension_query_scope — verify unit "returns All for explicit 'all' value"
    #[test]
    fn test_query_scope_all_explicit() {
        let mut manifest = default_manifest();
        manifest.query_scope = Some("all".to_string());
        assert_eq!(compute_extension_query_scope(&manifest), QueryScope::All);
    }

    // B:compute_extension_query_scope — verify unit "returns Own for 'own' value"
    #[test]
    fn test_query_scope_own() {
        let mut manifest = default_manifest();
        manifest.query_scope = Some("own".to_string());
        assert_eq!(compute_extension_query_scope(&manifest), QueryScope::Own);
    }

    // B:compute_extension_query_scope — verify unit "parses comma-separated kind list"
    #[test]
    fn test_query_scope_explicit_list() {
        let mut manifest = default_manifest();
        manifest.query_scope = Some("behavior, invariant, event".to_string());
        let scope = compute_extension_query_scope(&manifest);
        assert_eq!(
            scope,
            QueryScope::Explicit(vec![
                "behavior".to_string(),
                "invariant".to_string(),
                "event".to_string()
            ])
        );
    }

    // -- provide_host_function_emit_diagnostic --

    // B:provide_host_function_emit_diagnostic — verify unit "parses valid diagnostic JSON"
    #[test]
    fn test_emit_diagnostic_parses_valid_json() {
        let diag = Diagnostic {
            code: "W100".to_string(),
            severity: Severity::Warning,
            message: "test warning".to_string(),
            span: None,
            suggestion: None,
        };
        let json = serde_json::to_vec(&diag).unwrap();

        let result = host_emit_diagnostic("ext", &json);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.code, "W100");
        assert_eq!(parsed.message, "test warning");
    }

    // B:provide_host_function_emit_diagnostic — verify unit "rejects malformed JSON"
    #[test]
    fn test_emit_diagnostic_rejects_malformed_json() {
        let result = host_emit_diagnostic("ext", b"not json");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "E028");
        assert!(err.message.contains("malformed JSON"));
    }

    // B:provide_host_function_emit_diagnostic — verify unit "rejects incomplete diagnostic JSON"
    #[test]
    fn test_emit_diagnostic_rejects_incomplete_json() {
        let result = host_emit_diagnostic("ext", b"{\"code\": \"W100\"}");
        assert!(result.is_err());
    }

    // -- provide_host_function_read_file --

    // B:provide_host_function_read_file — verify unit "allows read under spec_root"
    #[test]
    fn test_read_file_allows_path_under_spec_root() {
        let policy = SandboxPolicy {
            file_system_access: Some(true),
            ..Default::default()
        };
        let result = host_read_file_check(
            "ext",
            Path::new("/project/spec/file.spec"),
            Path::new("/project/spec"),
            CallSite::Validator,
            &policy,
        );
        assert!(result.is_ok());
    }

    // B:provide_host_function_read_file — verify unit "denies path with '..' escape"
    #[test]
    fn test_read_file_denies_dot_dot_escape() {
        let policy = SandboxPolicy {
            file_system_access: Some(true),
            ..Default::default()
        };
        let result = host_read_file_check(
            "ext",
            Path::new("/project/spec/../../../etc/passwd"),
            Path::new("/project/spec"),
            CallSite::Validator,
            &policy,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains(".."));
    }

    // B:provide_host_function_read_file — verify unit "denies path outside spec_root"
    #[test]
    fn test_read_file_denies_path_outside_spec_root() {
        let policy = SandboxPolicy {
            file_system_access: Some(true),
            ..Default::default()
        };
        let result = host_read_file_check(
            "ext",
            Path::new("/etc/passwd"),
            Path::new("/project/spec"),
            CallSite::Validator,
            &policy,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("not under spec_root"));
    }

    // B:provide_host_function_read_file — verify unit "denies read from Renderer call site"
    #[test]
    fn test_read_file_denies_renderer_call_site() {
        let policy = SandboxPolicy {
            file_system_access: Some(true),
            ..Default::default()
        };
        let result = host_read_file_check(
            "ext",
            Path::new("/project/spec/file.spec"),
            Path::new("/project/spec"),
            CallSite::Renderer,
            &policy,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("call site"));
    }

    // -- provide_host_function_emit_file --

    // B:provide_host_function_emit_file — verify unit "allows output to output dir with allowed extension"
    #[test]
    fn test_emit_file_allows_valid_output() {
        let policy = SandboxPolicy {
            allowed_output_extensions: vec![".json".into(), ".html".into()],
            ..Default::default()
        };
        let result = host_emit_file_check(
            "ext",
            Path::new("/output/result.json"),
            Path::new("/output"),
            &policy,
        );
        assert!(result.is_ok());
    }

    // B:provide_host_function_emit_file — verify unit "denies output outside output dir"
    #[test]
    fn test_emit_file_denies_outside_output_dir() {
        let policy = SandboxPolicy::default();
        let result = host_emit_file_check(
            "ext",
            Path::new("/other/result.json"),
            Path::new("/output"),
            &policy,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("not under output dir"));
    }

    // B:provide_host_function_emit_file — verify unit "denies code file extension (.rs, .js, .ts)"
    #[test]
    fn test_emit_file_denies_code_extension() {
        let policy = SandboxPolicy {
            allowed_output_extensions: vec![".rs".into()], // Even if allowlisted
            ..Default::default()
        };
        let result = host_emit_file_check(
            "ext",
            Path::new("/output/main.rs"),
            Path::new("/output"),
            &policy,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("code file extension"));
    }

    // B:provide_host_function_emit_file — verify unit "denies non-allowlisted extension"
    #[test]
    fn test_emit_file_denies_non_allowlisted_extension() {
        let policy = SandboxPolicy {
            allowed_output_extensions: vec![".json".into()],
            ..Default::default()
        };
        let result = host_emit_file_check(
            "ext",
            Path::new("/output/report.csv"),
            Path::new("/output"),
            &policy,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("not in allowed_output_extensions"));
    }

    // -- provide_host_function_http_get --

    // B:provide_host_function_http_get — verify unit "allows request from Provider with allowed domain"
    #[test]
    fn test_http_get_allows_provider_with_allowed_domain() {
        let policy = SandboxPolicy {
            network_access: Some(true),
            allowed_domains: vec!["api.github.com".into()],
            ..Default::default()
        };
        let result = host_http_get_check("ext", "api.github.com", CallSite::Provider, &policy);
        assert!(result.is_ok());
    }

    // B:provide_host_function_http_get — verify unit "denies request from non-Provider call site"
    #[test]
    fn test_http_get_denies_non_provider_call_site() {
        let policy = SandboxPolicy {
            network_access: Some(true),
            ..Default::default()
        };
        let result = host_http_get_check("ext", "api.github.com", CallSite::Validator, &policy);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("call site"));
    }

    // B:provide_host_function_http_get — verify unit "denies request when network_access is disabled"
    #[test]
    fn test_http_get_denies_when_network_disabled() {
        let policy = SandboxPolicy {
            network_access: Some(false),
            ..Default::default()
        };
        let result = host_http_get_check("ext", "api.github.com", CallSite::Provider, &policy);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("network_access"));
    }

    // -- is_host_function_allowed (permission matrix) --

    // B:enforce_per_call_site_permissions — verify unit "emit_diagnostic allowed from all call sites"
    #[test]
    fn test_emit_diagnostic_allowed_from_all_sites() {
        assert!(is_host_function_allowed(CallSite::Validator, "host_emit_diagnostic"));
        assert!(is_host_function_allowed(CallSite::Renderer, "host_emit_diagnostic"));
        assert!(is_host_function_allowed(CallSite::Provider, "host_emit_diagnostic"));
        assert!(is_host_function_allowed(CallSite::Parser, "host_emit_diagnostic"));
        assert!(is_host_function_allowed(CallSite::Collector, "host_emit_diagnostic"));
    }

    // B:enforce_per_call_site_permissions — verify unit "read_file restricted to validators, providers, parsers"
    #[test]
    fn test_read_file_call_site_restrictions() {
        assert!(is_host_function_allowed(CallSite::Validator, "host_read_file"));
        assert!(is_host_function_allowed(CallSite::Provider, "host_read_file"));
        assert!(is_host_function_allowed(CallSite::Parser, "host_read_file"));
        assert!(!is_host_function_allowed(CallSite::Renderer, "host_read_file"));
        assert!(!is_host_function_allowed(CallSite::Collector, "host_read_file"));
    }

    // B:enforce_per_call_site_permissions — verify unit "http_get restricted to providers only"
    #[test]
    fn test_http_get_call_site_restrictions() {
        assert!(is_host_function_allowed(CallSite::Provider, "host_http_get"));
        assert!(!is_host_function_allowed(CallSite::Validator, "host_http_get"));
        assert!(!is_host_function_allowed(CallSite::Renderer, "host_http_get"));
        assert!(!is_host_function_allowed(CallSite::Parser, "host_http_get"));
        assert!(!is_host_function_allowed(CallSite::Collector, "host_http_get"));
    }

    // -- Wave 2: provide_host_function_emit_file code extension rejections --

    // B:provide_host_function_emit_file — verify unit "emit_file rejects .py extension"
    #[test]
    fn test_emit_file_rejects_py() {
        let policy = SandboxPolicy::default();
        let result = host_emit_file_check("ext", Path::new("/output/script.py"), Path::new("/output"), &policy);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains(".py"));
    }

    // B:provide_host_function_emit_file — verify unit "emit_file rejects .go extension"
    #[test]
    fn test_emit_file_rejects_go() {
        let policy = SandboxPolicy::default();
        let result = host_emit_file_check("ext", Path::new("/output/main.go"), Path::new("/output"), &policy);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains(".go"));
    }

    // B:provide_host_function_emit_file — verify unit "emit_file rejects .js extension"
    #[test]
    fn test_emit_file_rejects_js() {
        let policy = SandboxPolicy::default();
        let result = host_emit_file_check("ext", Path::new("/output/app.js"), Path::new("/output"), &policy);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains(".js"));
    }

    // B:provide_host_function_emit_file — verify unit "emit_file rejects .ts extension"
    #[test]
    fn test_emit_file_rejects_ts() {
        let policy = SandboxPolicy::default();
        let result = host_emit_file_check("ext", Path::new("/output/app.ts"), Path::new("/output"), &policy);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains(".ts"));
    }

    // B:provide_host_function_emit_file — verify unit "emit_file rejects .sh extension"
    #[test]
    fn test_emit_file_rejects_sh() {
        let policy = SandboxPolicy::default();
        let result = host_emit_file_check("ext", Path::new("/output/run.sh"), Path::new("/output"), &policy);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains(".sh"));
    }

    // B:provide_host_function_emit_file — verify unit "file with .html extension is accepted"
    #[test]
    fn test_emit_file_accepts_html() {
        let policy = SandboxPolicy::default();
        let result = host_emit_file_check("ext", Path::new("/output/report.html"), Path::new("/output"), &policy);
        assert!(result.is_ok());
    }

    // B:provide_host_function_emit_file — verify unit "file with .csv extension is accepted"
    #[test]
    fn test_emit_file_accepts_csv() {
        let policy = SandboxPolicy::default();
        let result = host_emit_file_check("ext", Path::new("/output/data.csv"), Path::new("/output"), &policy);
        assert!(result.is_ok());
    }

    // -- Wave 3: enforce_per_call_site_permissions expansion --

    // B:enforce_per_call_site_permissions — verify unit "renderer export additionally allows emit_file"
    #[test]
    fn test_renderer_allows_emit_file() {
        assert!(is_host_function_allowed(CallSite::Renderer, "host_emit_file"));
    }

    // B:enforce_per_call_site_permissions — verify unit "entity contribution export limited to query_graph, add_graph_node, add_graph_edge"
    #[test]
    fn test_parser_graph_mutation_permissions() {
        // Parser gets graph mutation
        assert!(is_host_function_allowed(CallSite::Parser, "host_add_graph_node"));
        assert!(is_host_function_allowed(CallSite::Parser, "host_add_graph_edge"));
        // Provider/Collector do NOT get graph mutation
        assert!(!is_host_function_allowed(CallSite::Provider, "host_add_graph_node"));
        assert!(!is_host_function_allowed(CallSite::Collector, "host_add_graph_node"));
    }

    // B:enforce_per_call_site_permissions — verify unit "collector contribution limited to query_graph and emit_file"
    #[test]
    fn test_collector_permissions() {
        assert!(is_host_function_allowed(CallSite::Collector, "host_query_graph"));
        assert!(is_host_function_allowed(CallSite::Collector, "host_emit_file"));
        assert!(is_host_function_allowed(CallSite::Collector, "host_emit_diagnostic"));
        // Collector cannot read files, make HTTP requests, or mutate graph
        assert!(!is_host_function_allowed(CallSite::Collector, "host_read_file"));
        assert!(!is_host_function_allowed(CallSite::Collector, "host_http_get"));
        assert!(!is_host_function_allowed(CallSite::Collector, "host_add_graph_node"));
    }

    // B:enforce_per_call_site_permissions — verify unit "parser contribution limited to emit_diagnostic, add_graph_node, add_graph_edge, read_file"
    #[test]
    fn test_parser_permissions() {
        assert!(is_host_function_allowed(CallSite::Parser, "host_emit_diagnostic"));
        assert!(is_host_function_allowed(CallSite::Parser, "host_add_graph_node"));
        assert!(is_host_function_allowed(CallSite::Parser, "host_add_graph_edge"));
        assert!(is_host_function_allowed(CallSite::Parser, "host_read_file"));
        // Parser cannot emit files or make HTTP requests
        assert!(!is_host_function_allowed(CallSite::Parser, "host_emit_file"));
        assert!(!is_host_function_allowed(CallSite::Parser, "host_http_get"));
    }

    #[test]
    fn test_analyzer_permissions() {
        assert!(is_host_function_allowed(CallSite::Analyzer, "host_emit_diagnostic"));
        assert!(is_host_function_allowed(CallSite::Analyzer, "host_read_file"));
        assert!(is_host_function_allowed(CallSite::Analyzer, "host_query_graph"));
        assert!(!is_host_function_allowed(CallSite::Analyzer, "host_emit_file"));
        assert!(!is_host_function_allowed(CallSite::Analyzer, "host_http_get"));
        assert!(!is_host_function_allowed(CallSite::Analyzer, "host_add_graph_node"));
        assert!(!is_host_function_allowed(CallSite::Analyzer, "host_add_graph_edge"));
    }

    // B:enforce_per_call_site_permissions — verify unit "unauthorized host function call is rejected"
    #[test]
    fn test_unknown_function_rejected_for_all_sites() {
        let sites = [
            CallSite::Validator,
            CallSite::Renderer,
            CallSite::Provider,
            CallSite::Parser,
            CallSite::Collector,
            CallSite::Analyzer,
        ];
        for site in &sites {
            assert!(
                !is_host_function_allowed(*site, "host_nonexistent_function"),
                "unknown function should be rejected for {:?}",
                site
            );
        }
    }

    // -- M10: symlink resolution in host_read_file_check --

    // M10 — verify unit "canonicalized symlink outside spec_root is denied"
    #[test]
    fn test_read_file_denies_symlink_outside_spec_root() {
        // Create a temp directory structure:
        //   spec_root/   (inside sandbox)
        //   outside/secret.spec  (outside sandbox)
        //   spec_root/link -> ../outside/secret.spec  (symlink escapes)
        let tmp = tempfile::tempdir().unwrap();
        let spec_root = tmp.path().join("spec_root");
        let outside = tmp.path().join("outside");
        std::fs::create_dir_all(&spec_root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.spec"), "secret data").unwrap();

        // Create symlink: spec_root/link -> ../outside/secret.spec
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            outside.join("secret.spec"),
            spec_root.join("link"),
        )
        .unwrap();

        let policy = SandboxPolicy {
            file_system_access: Some(true),
            ..Default::default()
        };

        // The path spec_root/link looks like it's under spec_root, but resolves outside
        let result = host_read_file_check(
            "ext",
            &spec_root.join("link"),
            &spec_root,
            CallSite::Validator,
            &policy,
        );
        // After fix: should be denied because canonicalized path is outside spec_root
        assert!(result.is_err(), "symlink escaping spec_root should be denied");
        assert!(result.unwrap_err().message.contains("not under spec_root"));
    }

    // M10 — verify unit "canonicalized path inside spec_root is allowed"
    #[test]
    fn test_read_file_allows_canonicalized_path_inside_spec_root() {
        let tmp = tempfile::tempdir().unwrap();
        let spec_root = tmp.path().join("spec_root");
        let subdir = spec_root.join("subdir");
        std::fs::create_dir_all(&subdir).unwrap();
        std::fs::write(subdir.join("file.spec"), "data").unwrap();

        // Create symlink within spec_root: spec_root/link -> spec_root/subdir/file.spec
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            subdir.join("file.spec"),
            spec_root.join("link"),
        )
        .unwrap();

        let policy = SandboxPolicy {
            file_system_access: Some(true),
            ..Default::default()
        };

        let result = host_read_file_check(
            "ext",
            &spec_root.join("link"),
            &spec_root,
            CallSite::Validator,
            &policy,
        );
        assert!(result.is_ok(), "symlink within spec_root should be allowed");
    }

    // M10 — verify unit "nonexistent file falls back to lexical check (no canonicalize failure)"
    #[test]
    fn test_read_file_nonexistent_path_under_spec_root_allowed() {
        let policy = SandboxPolicy {
            file_system_access: Some(true),
            ..Default::default()
        };
        // This path doesn't exist on disk, so canonicalize will fail.
        // Fallback to lexical check: path starts with spec_root, no ".." => allowed
        let result = host_read_file_check(
            "ext",
            Path::new("/nonexistent/spec/file.spec"),
            Path::new("/nonexistent/spec"),
            CallSite::Validator,
            &policy,
        );
        assert!(result.is_ok(), "nonexistent path under spec_root should fall back to lexical check");
    }

    // -- Wave 4: provide_host_function_emit_diagnostic + provide_host_function_http_get gaps --

    // B:provide_host_function_emit_diagnostic — verify unit "optional source span omitted without error"
    #[test]
    fn test_emit_diagnostic_span_null_accepted() {
        let json = r#"{"code":"W100","severity":"Warning","message":"test","span":null,"suggestion":null}"#;
        let result = host_emit_diagnostic("ext", json.as_bytes());
        assert!(result.is_ok());
        let diag = result.unwrap();
        assert!(diag.span.is_none());
    }

    // B:provide_host_function_emit_diagnostic — verify unit "diagnostic severity validated"
    #[test]
    fn test_emit_diagnostic_invalid_severity_rejected() {
        let json = r#"{"code":"W100","severity":"catastrophic","message":"test"}"#;
        let result = host_emit_diagnostic("ext", json.as_bytes());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "E028");
    }

    // B:provide_host_function_http_get — verify unit "disallowed domain is rejected"
    #[test]
    fn test_http_get_disallowed_domain_rejected() {
        let policy = SandboxPolicy {
            network_access: Some(true),
            allowed_domains: vec!["api.github.com".into()],
            ..Default::default()
        };
        let result = host_http_get_check("ext", "evil.example.com", CallSite::Provider, &policy);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("not in allowed_domains"));
    }

    // B:provide_host_function_http_get — verify unit "empty domain allowlist allows all domains"
    #[test]
    fn test_http_get_empty_allowlist_allows_all() {
        let policy = SandboxPolicy {
            network_access: Some(true),
            allowed_domains: vec![], // Empty = allow all
            ..Default::default()
        };
        let result = host_http_get_check("ext", "any-domain.example.com", CallSite::Provider, &policy);
        assert!(result.is_ok());
    }

    // -- host_add_graph_node_check --

    // B:provide_host_function_add_graph_node — verify unit "adds graph node instance for declared entity kind"
    #[test]
    fn test_add_graph_node_allows_declared_kind() {
        let declared = vec!["behavior".to_string(), "event".to_string()];
        let result = host_add_graph_node_check("ext", "behavior", CallSite::Parser, &declared);
        assert!(result.is_ok());
    }

    // B:provide_host_function_add_graph_node — verify unit "rejects node for undeclared entity kind"
    #[test]
    fn test_add_graph_node_rejects_undeclared_kind() {
        let declared = vec!["behavior".to_string()];
        let result = host_add_graph_node_check("ext", "widget", CallSite::Parser, &declared);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "E031");
        assert!(err.message.contains("widget"));
        assert!(err.message.contains("not declared"));
    }

    // B:provide_host_function_add_graph_node — verify unit "rejects non-parser call site"
    #[test]
    fn test_add_graph_node_rejects_non_parser_call_site() {
        let declared = vec!["behavior".to_string()];
        let result = host_add_graph_node_check("ext", "behavior", CallSite::Validator, &declared);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "E031");
        assert!(err.message.contains("call site"));
    }

    // B:provide_host_function_add_graph_node — verify contract
    #[test]
    fn test_add_graph_node_contract() {
        let declared = vec!["behavior".to_string(), "event".to_string()];

        // ensures: declared kind from parser succeeds
        assert!(host_add_graph_node_check("ext", "behavior", CallSite::Parser, &declared).is_ok());
        assert!(host_add_graph_node_check("ext", "event", CallSite::Parser, &declared).is_ok());

        // ensures: undeclared kind fails with E031
        let err = host_add_graph_node_check("ext", "unknown", CallSite::Parser, &declared).unwrap_err();
        assert_eq!(err.code, "E031");
        assert_eq!(err.severity, Severity::Error);

        // ensures: non-parser call site fails with E031
        let err = host_add_graph_node_check("ext", "behavior", CallSite::Renderer, &declared).unwrap_err();
        assert_eq!(err.code, "E031");
    }

    // -- host_add_graph_edge_check --

    // B:provide_host_function_add_graph_edge — verify unit "adds graph edge instance for declared edge type"
    #[test]
    fn test_add_graph_edge_allows_declared_label() {
        let labels = vec!["references".to_string(), "depends_on".to_string()];
        let mut nodes = std::collections::HashSet::new();
        nodes.insert("node_a".to_string());
        nodes.insert("node_b".to_string());

        let result = host_add_graph_edge_check(
            "ext", "references", "node_a", "node_b",
            CallSite::Parser, &labels, &nodes,
        );
        assert!(result.is_ok());
    }

    // B:provide_host_function_add_graph_edge — verify unit "rejects edge for undeclared edge label"
    #[test]
    fn test_add_graph_edge_rejects_undeclared_label() {
        let labels = vec!["references".to_string()];
        let mut nodes = std::collections::HashSet::new();
        nodes.insert("a".to_string());
        nodes.insert("b".to_string());

        let result = host_add_graph_edge_check(
            "ext", "unknown_edge", "a", "b",
            CallSite::Parser, &labels, &nodes,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "E031");
        assert!(err.message.contains("unknown_edge"));
        assert!(err.message.contains("not declared"));
    }

    // B:provide_host_function_add_graph_edge — verify unit "rejects edge when source or target node missing"
    #[test]
    fn test_add_graph_edge_rejects_missing_nodes() {
        let labels = vec!["references".to_string()];
        let mut nodes = std::collections::HashSet::new();
        nodes.insert("a".to_string());

        // Missing target
        let result = host_add_graph_edge_check(
            "ext", "references", "a", "missing_target",
            CallSite::Parser, &labels, &nodes,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("missing_target"));

        // Missing source
        let result = host_add_graph_edge_check(
            "ext", "references", "missing_source", "a",
            CallSite::Parser, &labels, &nodes,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("missing_source"));
    }

    // B:provide_host_function_add_graph_edge — verify contract
    #[test]
    fn test_add_graph_edge_contract() {
        let labels = vec!["references".to_string()];
        let mut nodes = std::collections::HashSet::new();
        nodes.insert("a".to_string());
        nodes.insert("b".to_string());

        // ensures: valid edge succeeds
        assert!(host_add_graph_edge_check("ext", "references", "a", "b", CallSite::Parser, &labels, &nodes).is_ok());

        // ensures: non-parser fails with E031
        let err = host_add_graph_edge_check("ext", "references", "a", "b", CallSite::Collector, &labels, &nodes).unwrap_err();
        assert_eq!(err.code, "E031");
        assert_eq!(err.severity, Severity::Error);

        // ensures: undeclared label fails
        let err = host_add_graph_edge_check("ext", "nope", "a", "b", CallSite::Parser, &labels, &nodes).unwrap_err();
        assert_eq!(err.code, "E031");

        // ensures: missing node fails
        let err = host_add_graph_edge_check("ext", "references", "a", "ghost", CallSite::Parser, &labels, &nodes).unwrap_err();
        assert_eq!(err.code, "E031");
    }

    // -- filter_graph_by_query_scope --

    fn sample_graph() -> serde_json::Value {
        serde_json::json!({
            "entities": [
                {"id": "b1", "kind": "behavior", "name": "login"},
                {"id": "e1", "kind": "event", "name": "user_logged_in"},
                {"id": "t1", "kind": "type", "name": "UserCredentials"}
            ],
            "edges": [
                {"source": "b1", "target": "e1", "label": "emits"},
                {"source": "b1", "target": "t1", "label": "uses"},
                {"source": "e1", "target": "t1", "label": "payload"}
            ]
        })
    }

    // B:provide_host_function_query_graph — verify unit "query_graph returns valid JSON graph"
    #[test]
    fn test_filter_graph_all_scope_returns_full_graph() {
        let graph = sample_graph();
        let result = filter_graph_by_query_scope(&graph, &QueryScope::All);
        assert_eq!(result, graph);
    }

    // B:provide_host_function_query_graph — verify unit "graph includes entities and edges"
    #[test]
    fn test_filter_graph_includes_entities_and_edges() {
        let graph = sample_graph();
        let result = filter_graph_by_query_scope(&graph, &QueryScope::All);
        assert!(result.get("entities").unwrap().as_array().unwrap().len() == 3);
        assert!(result.get("edges").unwrap().as_array().unwrap().len() == 3);
    }

    // B:provide_host_function_query_graph — verify unit "restricted scope returns filtered subgraph"
    #[test]
    fn test_filter_graph_restricted_scope_filters() {
        let graph = sample_graph();
        let scope = QueryScope::Explicit(vec!["behavior".to_string(), "event".to_string()]);
        let result = filter_graph_by_query_scope(&graph, &scope);

        let entities = result.get("entities").unwrap().as_array().unwrap();
        assert_eq!(entities.len(), 2);
        assert!(entities.iter().all(|e| {
            let kind = e.get("kind").unwrap().as_str().unwrap();
            kind == "behavior" || kind == "event"
        }));

        // Only the b1->e1 edge should remain (t1 is filtered out)
        let edges = result.get("edges").unwrap().as_array().unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].get("label").unwrap().as_str().unwrap(), "emits");
    }

    // B:provide_host_function_query_graph — verify contract
    #[test]
    fn test_filter_graph_contract() {
        let graph = sample_graph();

        // ensures: All scope returns full graph
        let all = filter_graph_by_query_scope(&graph, &QueryScope::All);
        assert_eq!(all, graph);

        // ensures: Explicit scope filters entities and edges
        let explicit = filter_graph_by_query_scope(
            &graph,
            &QueryScope::Explicit(vec!["type".to_string()]),
        );
        let entities = explicit.get("entities").unwrap().as_array().unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].get("kind").unwrap().as_str().unwrap(), "type");
        // No edges because no edge has both endpoints as "type"
        let edges = explicit.get("edges").unwrap().as_array().unwrap();
        assert!(edges.is_empty());

        // ensures: Own with empty kinds returns empty
        let own = filter_graph_by_query_scope(&graph, &QueryScope::Own);
        let entities = own.get("entities").unwrap().as_array().unwrap();
        assert!(entities.is_empty());
    }
}
