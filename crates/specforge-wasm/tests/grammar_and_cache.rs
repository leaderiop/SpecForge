// Slice 12: Grammar, Body Parser & Cache Integration Tests
//
// Tests behaviors through the public API:
// - B:verify_wasm_integrity
// - B:load_extension_grammar
// - B:validate_grammar_wasm
// - B:compose_grammar_injections (integration-level conflict tests)
// - B:dispatch_body_parser
// - B:cache_grammar_artifacts

use specforge_common::Severity;
use specforge_registry::{GrammarContribution, ManifestV2};
use specforge_wasm::{
    cache_grammar_artifact, compose_grammar_injections, dispatch_body_parser, has_cached_grammar,
    hex_sha256, load_extension_grammar, validate_grammar_wasm, verify_wasm_integrity,
    GrammarConflictPolicy, WasmCallResult, WasmRuntime, WasmTrapInfo,
};
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

// -- Local mock runtime for integration tests --

struct MockRuntime {
    call_results: std::collections::HashMap<String, WasmCallResult>,
}

impl MockRuntime {
    fn new() -> Self {
        Self {
            call_results: std::collections::HashMap::new(),
        }
    }

    fn with_call_ok(mut self, export: &str, output: Vec<u8>) -> Self {
        self.call_results
            .insert(export.to_string(), WasmCallResult::Ok(output));
        self
    }

    fn with_call_trap(mut self, export: &str, trap: WasmTrapInfo) -> Self {
        self.call_results
            .insert(export.to_string(), WasmCallResult::Trap(trap));
        self
    }
}

impl WasmRuntime for MockRuntime {
    fn load_module(&self, _: &Path, _: Option<&Path>) -> Result<(), String> {
        Ok(())
    }
    fn call_export(&self, _: &str, export_name: &str, _: &[u8]) -> WasmCallResult {
        self.call_results
            .get(export_name)
            .cloned()
            .unwrap_or(WasmCallResult::Ok(vec![]))
    }
    fn has_cached_module(&self, _: &str) -> bool {
        false
    }
}

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
        surfaces: None,
    }
}

fn write_temp_wasm(content: &[u8]) -> (NamedTempFile, String) {
    use std::io::Write;
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content).unwrap();
    f.flush().unwrap();
    let hash = hex_sha256(content);
    (f, hash)
}

// ============================================================================
// B:verify_wasm_integrity — integration tests
// ============================================================================

// B:verify_wasm_integrity — verify integration "matching SHA256 → Ok"
#[test]
fn test_verify_integrity_matching_hash_passes() {
    let (f, hash) = write_temp_wasm(b"\x00asm\x01\x00\x00\x00test_binary");
    let result = verify_wasm_integrity(f.path(), &hash);
    assert!(result.is_ok());
}

// B:verify_wasm_integrity — verify integration "mismatched SHA256 → E032 (tampering)"
#[test]
fn test_verify_integrity_mismatch_produces_e032() {
    let (f, _) = write_temp_wasm(b"\x00asm\x01\x00\x00\x00test_binary");
    let err = verify_wasm_integrity(f.path(), "deadbeefdeadbeef").unwrap_err();
    assert_eq!(err.code, "E032");
    assert_eq!(err.severity, Severity::Error);
    assert!(err.message.contains("tampering"));
}

// B:verify_wasm_integrity — verify contract "requires hash + bytes, ensures integrity check"
#[test]
fn test_verify_integrity_contract() {
    let content = b"module_binary_content";
    let (f, hash) = write_temp_wasm(content);

    // ensures: correct hash passes
    assert!(verify_wasm_integrity(f.path(), &hash).is_ok());

    // ensures: wrong hash fails with E032
    let err = verify_wasm_integrity(f.path(), "badhash").unwrap_err();
    assert_eq!(err.code, "E032");

    // ensures: missing file fails with E028
    let err = verify_wasm_integrity(Path::new("/no/such/file.wasm"), &hash).unwrap_err();
    assert_eq!(err.code, "E028");
}

// ============================================================================
// B:load_extension_grammar — integration tests
// ============================================================================

// B:load_extension_grammar — verify integration "valid grammar wasm → GrammarLoadResult"
#[test]
fn test_load_grammar_valid_returns_result() {
    let bytes = vec![0u8; 100];
    let exports = vec!["tree_sitter_specforge".to_string()];
    let result = load_extension_grammar(
        "/grammars/specforge.wasm",
        &bytes,
        "tree_sitter_specforge",
        &exports,
        14,
        14,
        1024,
    )
    .unwrap();
    assert_eq!(result.grammar_path, "/grammars/specforge.wasm");
    assert_eq!(result.abi_version, 14);
    assert_eq!(result.content_hash.len(), 64);
}

// B:load_extension_grammar — verify integration "invalid grammar → diagnostic"
#[test]
fn test_load_grammar_missing_export_fails() {
    let bytes = vec![0u8; 100];
    let exports = vec!["wrong_export".to_string()];
    let err = load_extension_grammar(
        "/grammars/bad.wasm",
        &bytes,
        "tree_sitter_specforge",
        &exports,
        14,
        14,
        1024,
    )
    .unwrap_err();
    assert_eq!(err.code, "E036");
}

// B:load_extension_grammar — verify contract "requires wasm bytes, ensures grammar or error"
#[test]
fn test_load_grammar_contract() {
    let bytes = vec![0u8; 100];
    let exports = vec!["tree_sitter_specforge".to_string()];

    // ensures: valid → Ok with deterministic hash
    let r1 = load_extension_grammar("/g.wasm", &bytes, "tree_sitter_specforge", &exports, 14, 14, 1024).unwrap();
    let r2 = load_extension_grammar("/g.wasm", &bytes, "tree_sitter_specforge", &exports, 14, 14, 1024).unwrap();
    assert_eq!(r1.content_hash, r2.content_hash);

    // ensures: ABI mismatch → E037
    let err = load_extension_grammar("/g.wasm", &bytes, "tree_sitter_specforge", &exports, 13, 14, 1024).unwrap_err();
    assert_eq!(err.code, "E037");

    // ensures: oversized → E038
    let big = vec![0u8; 2000];
    let err = load_extension_grammar("/g.wasm", &big, "tree_sitter_specforge", &exports, 14, 14, 1024).unwrap_err();
    assert_eq!(err.code, "E038");
}

// ============================================================================
// B:validate_grammar_wasm — integration tests
// ============================================================================

// B:validate_grammar_wasm — verify integration "valid ABI + exports → Ok"
#[test]
fn test_validate_grammar_valid_passes() {
    let bytes = vec![0u8; 100];
    let exports = vec!["tree_sitter_specforge".to_string()];
    let result = validate_grammar_wasm(&bytes, "tree_sitter_specforge", &exports, 14, 14, 1024);
    assert!(result.is_ok());
}

// B:validate_grammar_wasm — verify integration "wrong ABI version → E037"
#[test]
fn test_validate_grammar_wrong_abi_produces_e037() {
    let bytes = vec![0u8; 100];
    let exports = vec!["tree_sitter_specforge".to_string()];
    let err = validate_grammar_wasm(&bytes, "tree_sitter_specforge", &exports, 13, 14, 1024).unwrap_err();
    assert_eq!(err.code, "E037");
    assert!(err.message.contains("13"));
    assert!(err.message.contains("14"));
}

// B:validate_grammar_wasm — verify contract "requires grammar bytes, ensures validation"
#[test]
fn test_validate_grammar_contract() {
    let bytes = vec![0u8; 100];
    let exports = vec!["tree_sitter_specforge".to_string()];

    // ensures: missing export → E036
    let err = validate_grammar_wasm(&bytes, "missing_export", &exports, 14, 14, 1024).unwrap_err();
    assert_eq!(err.code, "E036");

    // ensures: oversized → E038
    let big = vec![0u8; 2000];
    let err = validate_grammar_wasm(&big, "tree_sitter_specforge", &exports, 14, 14, 1024).unwrap_err();
    assert_eq!(err.code, "E038");
}

// ============================================================================
// B:compose_grammar_injections — integration-level conflict tests
// ============================================================================

// B:compose_grammar_injections — verify integration "Error policy with two extensions same kind → E018"
#[test]
fn test_compose_grammar_error_policy_conflict() {
    let mut m1 = default_manifest();
    m1.name = "@ext/a".to_string();
    m1.grammar_contributions = vec![GrammarContribution {
        entity_kind: "behavior".to_string(),
        grammar_wasm_path: "/grammars/a.wasm".to_string(),
        export_name: None,
    }];

    let mut m2 = default_manifest();
    m2.name = "@ext/b".to_string();
    m2.grammar_contributions = vec![GrammarContribution {
        entity_kind: "behavior".to_string(),
        grammar_wasm_path: "/grammars/b.wasm".to_string(),
        export_name: None,
    }];

    let err = compose_grammar_injections(&[m1, m2], GrammarConflictPolicy::Error).unwrap_err();
    assert_eq!(err.len(), 1);
    assert_eq!(err[0].code, "E018");
    assert!(err[0].message.contains("behavior"));
}

// B:compose_grammar_injections — verify integration "Priority policy → first contributor wins"
#[test]
fn test_compose_grammar_priority_policy() {
    let mut m1 = default_manifest();
    m1.name = "@ext/a".to_string();
    m1.grammar_contributions = vec![GrammarContribution {
        entity_kind: "behavior".to_string(),
        grammar_wasm_path: "/grammars/a.wasm".to_string(),
        export_name: None,
    }];

    let mut m2 = default_manifest();
    m2.name = "@ext/b".to_string();
    m2.grammar_contributions = vec![GrammarContribution {
        entity_kind: "behavior".to_string(),
        grammar_wasm_path: "/grammars/b.wasm".to_string(),
        export_name: None,
    }];

    let result = compose_grammar_injections(&[m1, m2], GrammarConflictPolicy::Priority).unwrap();
    // First contributor wins
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].2, "@ext/a");
}

// B:compose_grammar_injections — verify integration "Namespace policy → both included"
#[test]
fn test_compose_grammar_namespace_policy() {
    let mut m1 = default_manifest();
    m1.name = "@ext/a".to_string();
    m1.grammar_contributions = vec![GrammarContribution {
        entity_kind: "behavior".to_string(),
        grammar_wasm_path: "/grammars/a.wasm".to_string(),
        export_name: None,
    }];

    let mut m2 = default_manifest();
    m2.name = "@ext/b".to_string();
    m2.grammar_contributions = vec![GrammarContribution {
        entity_kind: "behavior".to_string(),
        grammar_wasm_path: "/grammars/b.wasm".to_string(),
        export_name: None,
    }];

    let result = compose_grammar_injections(&[m1, m2], GrammarConflictPolicy::Namespace).unwrap();
    assert_eq!(result.len(), 2);
}

// ============================================================================
// B:dispatch_body_parser — integration tests
// ============================================================================

// B:dispatch_body_parser — verify integration "valid body content → parsed output"
#[test]
fn test_dispatch_body_parser_success() {
    let json_output = serde_json::json!({"key": "value"});
    let runtime =
        MockRuntime::new().with_call_ok("parse_body", serde_json::to_vec(&json_output).unwrap());

    let result = dispatch_body_parser("ext", "parse_body", "some body text", &runtime);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), json_output);
}

// B:dispatch_body_parser — verify integration "Wasm trap → fallback error"
#[test]
fn test_dispatch_body_parser_trap_returns_error() {
    let runtime = MockRuntime::new().with_call_trap(
        "parse_body",
        WasmTrapInfo {
            kind: "unreachable".to_string(),
            message: "parser panic".to_string(),
            export_name: "parse_body".to_string(),
        },
    );

    let err = dispatch_body_parser("ext", "parse_body", "body", &runtime).unwrap_err();
    assert_eq!(err.code, "E028");
    assert!(err.message.contains("trapped"));
}

// B:dispatch_body_parser — verify contract "requires body content + runtime, ensures parsed or fallback"
#[test]
fn test_dispatch_body_parser_contract() {
    // ensures: valid JSON returned as Value
    let runtime_ok =
        MockRuntime::new().with_call_ok("parse_body", b"{\"x\":1}".to_vec());
    let result = dispatch_body_parser("ext", "parse_body", "text", &runtime_ok);
    assert!(result.is_ok());

    // ensures: invalid JSON output → E028
    let runtime_bad = MockRuntime::new().with_call_ok("parse_body", b"not json".to_vec());
    let err = dispatch_body_parser("ext", "parse_body", "text", &runtime_bad).unwrap_err();
    assert_eq!(err.code, "E028");
    assert!(err.message.contains("invalid JSON"));

    // ensures: trap → E028
    let runtime_trap = MockRuntime::new().with_call_trap(
        "parse_body",
        WasmTrapInfo {
            kind: "trap".to_string(),
            message: "boom".to_string(),
            export_name: "parse_body".to_string(),
        },
    );
    let err = dispatch_body_parser("ext", "parse_body", "text", &runtime_trap).unwrap_err();
    assert_eq!(err.code, "E028");
}

// ============================================================================
// B:cache_grammar_artifacts — integration tests
// ============================================================================

// B:cache_grammar_artifacts — verify integration "store and retrieve grammar by hash + ABI key"
#[test]
fn test_grammar_cache_store_and_retrieve() {
    let dir = TempDir::new().unwrap();
    let cache_dir = dir.path().join("grammar_cache");
    let bytes = b"grammar bytes data";
    let hash = hex_sha256(bytes);

    // Store
    let path = cache_grammar_artifact(&hash, 14, bytes, &cache_dir).unwrap();
    assert!(path.exists());

    // Retrieve
    let cached = has_cached_grammar(&hash, 14, &cache_dir);
    assert!(cached.is_some());
    assert_eq!(cached.unwrap(), path);
}

// B:cache_grammar_artifacts — verify integration "content change invalidates cache entry"
#[test]
fn test_grammar_cache_content_change_invalidates() {
    let dir = TempDir::new().unwrap();
    let cache_dir = dir.path().join("grammar_cache");
    let bytes_v1 = b"grammar v1";
    let hash_v1 = hex_sha256(bytes_v1);

    cache_grammar_artifact(&hash_v1, 14, bytes_v1, &cache_dir).unwrap();

    // Different content hash = cache miss
    let hash_v2 = hex_sha256(b"grammar v2");
    assert!(has_cached_grammar(&hash_v2, 14, &cache_dir).is_none());

    // Different ABI = cache miss
    assert!(has_cached_grammar(&hash_v1, 15, &cache_dir).is_none());

    // Same hash + ABI = cache hit
    assert!(has_cached_grammar(&hash_v1, 14, &cache_dir).is_some());
}

// B:cache_grammar_artifacts — verify contract "requires grammar bytes, ensures cache consistency"
#[test]
fn test_grammar_cache_contract() {
    let dir = TempDir::new().unwrap();
    let cache_dir = dir.path().join("grammar_cache");
    let bytes = b"test grammar";
    let hash = hex_sha256(bytes);

    // ensures: caching writes to disk
    let path = cache_grammar_artifact(&hash, 14, bytes, &cache_dir).unwrap();
    assert!(path.exists());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    // ensures: cache key is composite (hash + ABI)
    assert!(has_cached_grammar(&hash, 14, &cache_dir).is_some());
    assert!(has_cached_grammar(&hash, 99, &cache_dir).is_none());
    assert!(has_cached_grammar("other", 14, &cache_dir).is_none());
}
