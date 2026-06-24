// Slice 16: Host Functions & Query Scope Integration Tests
//
// Tests host function permission checks, query scope computation,
// and graph filtering through public API.

use specforge_common::{Diagnostic, Severity};
use specforge_registry::{ManifestV2, SandboxPolicy};
use specforge_wasm::{
    compute_extension_query_scope, filter_graph_by_query_scope, host_add_graph_edge_check,
    host_add_graph_node_check, host_emit_diagnostic, host_emit_file_check, host_http_get_check,
    host_read_file_check, is_host_function_allowed, CallSite, QueryScope,
};
use std::collections::HashSet;
use std::path::Path;

fn default_manifest() -> ManifestV2 {
    ManifestV2 {
        name: String::new(),
        version: String::new(),
        manifest_version: 2,
        wasm_path: String::new(),
        contributes: Default::default(),
        entity_kinds: vec![],
        edge_types: vec![],
        fields: vec![],
        validation_rules: vec![],
        verify_kinds: vec![],
        reserved_keywords: vec![],
        peer_dependencies: vec![],
        sandbox_policy: None,
        incremental: None,
        migration_hook: None,
        host_api_version: None,
        entity_enhancements: vec![],
        starter_template: None,
        grammar_contributions: vec![],
        body_parser_contributions: vec![],
        ext_short: None,
        query_scope: None,
        collector_contributions: vec![],
        analyzer_contributions: vec![],
        surfaces: None,
    }
}

fn fs_enabled_policy() -> SandboxPolicy {
    SandboxPolicy {
        max_memory_mb: None,
        max_execution_ms: None,
        allowed_domains: vec![],
        allowed_paths: vec![],
        allowed_output_extensions: vec![],
        network_access: None,
        file_system_access: Some(true),
    }
}

fn network_enabled_policy(domains: Vec<String>) -> SandboxPolicy {
    SandboxPolicy {
        max_memory_mb: None,
        max_execution_ms: None,
        allowed_domains: domains,
        allowed_paths: vec![],
        allowed_output_extensions: vec![],
        network_access: Some(true),
        file_system_access: None,
    }
}

// ============================================================
// B:is_host_function_allowed
// ============================================================

// B:is_host_function_allowed — verify integration "host_emit_diagnostic allowed from all call sites"
#[test]
fn host_emit_diagnostic_allowed_all_sites() {
    for site in [
        CallSite::Validator,
        CallSite::Renderer,
        CallSite::Provider,
        CallSite::Parser,
        CallSite::Collector,
    ] {
        assert!(
            is_host_function_allowed(site, "host_emit_diagnostic"),
            "should be allowed from {:?}",
            site
        );
    }
}

// B:is_host_function_allowed — verify integration "host_read_file allowed from Validator/Provider/Parser, denied from Renderer/Collector"
#[test]
fn host_read_file_call_site_permissions() {
    assert!(is_host_function_allowed(
        CallSite::Validator,
        "host_read_file"
    ));
    assert!(is_host_function_allowed(
        CallSite::Provider,
        "host_read_file"
    ));
    assert!(is_host_function_allowed(
        CallSite::Parser,
        "host_read_file"
    ));
    assert!(!is_host_function_allowed(
        CallSite::Renderer,
        "host_read_file"
    ));
    assert!(!is_host_function_allowed(
        CallSite::Collector,
        "host_read_file"
    ));
}

// B:is_host_function_allowed — verify integration "host_http_get only from Provider"
#[test]
fn host_http_get_only_provider() {
    assert!(is_host_function_allowed(
        CallSite::Provider,
        "host_http_get"
    ));
    assert!(!is_host_function_allowed(
        CallSite::Validator,
        "host_http_get"
    ));
    assert!(!is_host_function_allowed(
        CallSite::Renderer,
        "host_http_get"
    ));
    assert!(!is_host_function_allowed(
        CallSite::Parser,
        "host_http_get"
    ));
    assert!(!is_host_function_allowed(
        CallSite::Collector,
        "host_http_get"
    ));
}

// B:is_host_function_allowed — verify integration "host_add_graph_node/edge only from Parser"
#[test]
fn host_graph_mutations_only_parser() {
    for func in ["host_add_graph_node", "host_add_graph_edge"] {
        assert!(is_host_function_allowed(CallSite::Parser, func));
        assert!(!is_host_function_allowed(CallSite::Validator, func));
        assert!(!is_host_function_allowed(CallSite::Renderer, func));
        assert!(!is_host_function_allowed(CallSite::Provider, func));
        assert!(!is_host_function_allowed(CallSite::Collector, func));
    }
}

// ============================================================
// B:host_read_file_check
// ============================================================

// B:host_read_file_check — verify integration "valid path under spec_root with fs access"
#[test]
fn read_file_valid_path_ok() {
    let spec_root = Path::new("/project/spec");
    let path = Path::new("/project/spec/behaviors/auth.spec");
    let policy = fs_enabled_policy();
    let result = host_read_file_check("@ext/a", path, spec_root, CallSite::Validator, &policy);
    assert!(result.is_ok());
}

// B:host_read_file_check — verify integration "path with .. escape produces E031"
#[test]
fn read_file_dotdot_escape_e031() {
    let spec_root = Path::new("/project/spec");
    let path = Path::new("/project/spec/../secrets/key.pem");
    let policy = fs_enabled_policy();
    let err =
        host_read_file_check("@ext/a", path, spec_root, CallSite::Validator, &policy).unwrap_err();
    assert_eq!(err.code, "E031");
    assert!(err.message.contains(".."));
}

// B:host_read_file_check — verify integration "path outside spec_root produces E031"
#[test]
fn read_file_outside_spec_root_e031() {
    let spec_root = Path::new("/project/spec");
    let path = Path::new("/other/directory/file.spec");
    let policy = fs_enabled_policy();
    let err =
        host_read_file_check("@ext/a", path, spec_root, CallSite::Validator, &policy).unwrap_err();
    assert_eq!(err.code, "E031");
    assert!(err.message.contains("not under spec_root"));
}

// B:host_read_file_check — verify integration "disallowed call site (Renderer) produces E031"
#[test]
fn read_file_disallowed_call_site_e031() {
    let spec_root = Path::new("/project/spec");
    let path = Path::new("/project/spec/file.spec");
    let policy = fs_enabled_policy();
    let err =
        host_read_file_check("@ext/a", path, spec_root, CallSite::Renderer, &policy).unwrap_err();
    assert_eq!(err.code, "E031");
    assert!(err.message.contains("not allowed from"));
}

// ============================================================
// B:host_emit_file_check
// ============================================================

// B:host_emit_file_check — verify integration "valid path under output_dir"
#[test]
fn emit_file_valid_path_ok() {
    let output_dir = Path::new("/project/output");
    let path = Path::new("/project/output/report.html");
    let policy = SandboxPolicy {
        allowed_output_extensions: vec![],
        ..Default::default()
    };
    let result = host_emit_file_check("@ext/a", path, output_dir, &policy);
    assert!(result.is_ok());
}

// B:host_emit_file_check — verify integration "path outside output_dir produces E031"
#[test]
fn emit_file_outside_output_dir_e031() {
    let output_dir = Path::new("/project/output");
    let path = Path::new("/project/src/main.rs");
    let policy = SandboxPolicy::default();
    let err = host_emit_file_check("@ext/a", path, output_dir, &policy).unwrap_err();
    assert_eq!(err.code, "E031");
    assert!(err.message.contains("not under output dir"));
}

// B:host_emit_file_check — verify integration "code file extension blocked with E031"
#[test]
fn emit_file_code_extension_e031() {
    let output_dir = Path::new("/project/output");
    let path = Path::new("/project/output/exploit.rs");
    let policy = SandboxPolicy::default();
    let err = host_emit_file_check("@ext/a", path, output_dir, &policy).unwrap_err();
    assert_eq!(err.code, "E031");
    assert!(err.message.contains("code file extension"));
}

// ============================================================
// B:host_http_get_check
// ============================================================

// B:host_http_get_check — verify integration "provider + allowed domain + network enabled"
#[test]
fn http_get_provider_allowed_domain_ok() {
    let policy = network_enabled_policy(vec!["api.example.com".to_string()]);
    let result = host_http_get_check("@ext/a", "api.example.com", CallSite::Provider, &policy);
    assert!(result.is_ok());
}

// B:host_http_get_check — verify integration "non-provider call site produces E031"
#[test]
fn http_get_non_provider_e031() {
    let policy = network_enabled_policy(vec![]);
    let err =
        host_http_get_check("@ext/a", "api.example.com", CallSite::Validator, &policy).unwrap_err();
    assert_eq!(err.code, "E031");
    assert!(err.message.contains("not allowed from"));
}

// B:host_http_get_check — verify integration "domain not in allowlist produces E031"
#[test]
fn http_get_domain_not_allowed_e031() {
    let policy = network_enabled_policy(vec!["safe.example.com".to_string()]);
    let err =
        host_http_get_check("@ext/a", "evil.example.com", CallSite::Provider, &policy).unwrap_err();
    assert_eq!(err.code, "E031");
    assert!(err.message.contains("not in allowed_domains"));
}

// ============================================================
// B:host_add_graph_node_check
// ============================================================

// B:host_add_graph_node_check — verify integration "parser + declared kind"
#[test]
fn add_node_parser_declared_kind_ok() {
    let kinds = vec!["behavior".to_string(), "invariant".to_string()];
    let result = host_add_graph_node_check("@ext/a", "behavior", CallSite::Parser, &kinds);
    assert!(result.is_ok());
}

// B:host_add_graph_node_check — verify integration "non-parser call site produces E031"
#[test]
fn add_node_non_parser_e031() {
    let kinds = vec!["behavior".to_string()];
    let err =
        host_add_graph_node_check("@ext/a", "behavior", CallSite::Validator, &kinds).unwrap_err();
    assert_eq!(err.code, "E031");
}

// B:host_add_graph_node_check — verify integration "undeclared entity kind produces E031"
#[test]
fn add_node_undeclared_kind_e031() {
    let kinds = vec!["behavior".to_string()];
    let err =
        host_add_graph_node_check("@ext/a", "widget", CallSite::Parser, &kinds).unwrap_err();
    assert_eq!(err.code, "E031");
    assert!(err.message.contains("not declared"));
}

// ============================================================
// B:host_add_graph_edge_check
// ============================================================

// B:host_add_graph_edge_check — verify integration "parser + declared label + known nodes"
#[test]
fn add_edge_valid_ok() {
    let labels = vec!["implements".to_string()];
    let nodes: HashSet<String> = ["b1".to_string(), "f1".to_string()].into_iter().collect();
    let result = host_add_graph_edge_check(
        "@ext/a",
        "implements",
        "b1",
        "f1",
        CallSite::Parser,
        &labels,
        &nodes,
    );
    assert!(result.is_ok());
}

// B:host_add_graph_edge_check — verify integration "undeclared edge label produces E031"
#[test]
fn add_edge_undeclared_label_e031() {
    let labels = vec!["implements".to_string()];
    let nodes: HashSet<String> = ["b1".to_string(), "f1".to_string()].into_iter().collect();
    let err = host_add_graph_edge_check(
        "@ext/a",
        "unknown_label",
        "b1",
        "f1",
        CallSite::Parser,
        &labels,
        &nodes,
    )
    .unwrap_err();
    assert_eq!(err.code, "E031");
    assert!(err.message.contains("not declared"));
}

// B:host_add_graph_edge_check — verify integration "unknown source/target node produces E031"
#[test]
fn add_edge_unknown_node_e031() {
    let labels = vec!["implements".to_string()];
    let nodes: HashSet<String> = ["b1".to_string()].into_iter().collect();
    let err = host_add_graph_edge_check(
        "@ext/a",
        "implements",
        "b1",
        "missing_target",
        CallSite::Parser,
        &labels,
        &nodes,
    )
    .unwrap_err();
    assert_eq!(err.code, "E031");
    assert!(err.message.contains("target node"));
}

// ============================================================
// B:compute_extension_query_scope
// ============================================================

// B:compute_extension_query_scope — verify integration "no query_scope defaults to All"
#[test]
fn query_scope_default_all() {
    let manifest = default_manifest();
    assert_eq!(compute_extension_query_scope(&manifest), QueryScope::All);
}

// B:compute_extension_query_scope — verify integration "'own' returns Own"
#[test]
fn query_scope_own() {
    let mut manifest = default_manifest();
    manifest.query_scope = Some("own".to_string());
    assert_eq!(compute_extension_query_scope(&manifest), QueryScope::Own);
}

// B:compute_extension_query_scope — verify integration "comma-separated list returns Explicit"
#[test]
fn query_scope_explicit() {
    let mut manifest = default_manifest();
    manifest.query_scope = Some("behavior,invariant".to_string());
    assert_eq!(
        compute_extension_query_scope(&manifest),
        QueryScope::Explicit(vec!["behavior".to_string(), "invariant".to_string()])
    );
}

// ============================================================
// B:filter_graph_by_query_scope
// ============================================================

// B:filter_graph_by_query_scope — verify integration "All scope returns full graph"
#[test]
fn filter_all_scope_unchanged() {
    let graph = serde_json::json!({
        "entities": [
            {"id": "b1", "kind": "behavior"},
            {"id": "i1", "kind": "invariant"}
        ],
        "edges": [
            {"source": "b1", "target": "i1", "label": "maintains"}
        ]
    });
    let result = filter_graph_by_query_scope(&graph, &QueryScope::All);
    assert_eq!(result, graph);
}

// B:filter_graph_by_query_scope — verify integration "Explicit scope filters entities + edges"
#[test]
fn filter_explicit_scope_filters() {
    let graph = serde_json::json!({
        "entities": [
            {"id": "b1", "kind": "behavior"},
            {"id": "i1", "kind": "invariant"},
            {"id": "e1", "kind": "event"}
        ],
        "edges": [
            {"source": "b1", "target": "i1", "label": "maintains"},
            {"source": "b1", "target": "e1", "label": "triggers"}
        ]
    });
    let scope = QueryScope::Explicit(vec!["behavior".to_string(), "invariant".to_string()]);
    let result = filter_graph_by_query_scope(&graph, &scope);

    let entities = result["entities"].as_array().unwrap();
    assert_eq!(entities.len(), 2);
    assert!(entities.iter().all(|e| {
        let kind = e["kind"].as_str().unwrap();
        kind == "behavior" || kind == "invariant"
    }));

    // Edge from b1→e1 should be removed (e1 is filtered out)
    let edges = result["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["target"].as_str().unwrap(), "i1");
}

// B:filter_graph_by_query_scope — verify integration "Own scope with no kinds returns empty entities"
#[test]
fn filter_own_scope_empty() {
    let graph = serde_json::json!({
        "entities": [
            {"id": "b1", "kind": "behavior"}
        ],
        "edges": []
    });
    let result = filter_graph_by_query_scope(&graph, &QueryScope::Own);
    let entities = result["entities"].as_array().unwrap();
    assert!(entities.is_empty());
}

// ============================================================
// B:host_emit_diagnostic
// ============================================================

// B:host_emit_diagnostic — verify integration "valid diagnostic JSON parsed"
#[test]
fn emit_diagnostic_valid_json() {
    let diag = Diagnostic {
        code: "W100".to_string(),
        severity: Severity::Warning,
        message: "test warning".to_string(),
        span: None,
        suggestion: None,
    };
    let json = serde_json::to_vec(&diag).unwrap();
    let result = host_emit_diagnostic("@ext/a", &json).unwrap();
    assert_eq!(result.code, "W100");
    assert_eq!(result.message, "test warning");
}

// B:host_emit_diagnostic — verify integration "malformed JSON produces E028"
#[test]
fn emit_diagnostic_malformed_json_e028() {
    let err = host_emit_diagnostic("@ext/a", b"not json {{{").unwrap_err();
    assert_eq!(err.code, "E028");
    assert!(err.message.contains("malformed JSON"));
}
