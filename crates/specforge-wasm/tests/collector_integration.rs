// Slice 17: Collector Pipeline & Report Ingestion Integration Tests
//
// Tests collector registration, dispatch, auto-detection, output validation,
// and report ingestion through public API.

use specforge_registry::{ExtensionContributions, ManifestV2};
use specforge_wasm::{
    auto_detect_collector, dispatch_collector, ingest_collector_report,
    is_contribution_disabled, register_collector_contributions, validate_collector_output,
    ContributionToggle, CoverageMetadata, RegisteredCollector, WasmCallResult, WasmRuntime,
    WasmTrapInfo,
};
use std::collections::HashSet;
use std::path::Path;

// -- Local MockRuntime --

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
    fn load_module(&self, _wasm_path: &Path, _aot: Option<&Path>) -> Result<(), String> {
        Ok(())
    }

    fn call_export(&self, _ext: &str, export_name: &str, _input: &[u8]) -> WasmCallResult {
        self.call_results
            .get(export_name)
            .cloned()
            .unwrap_or(WasmCallResult::Ok(vec![]))
    }

    fn has_cached_module(&self, _wasm_hash: &str) -> bool {
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

// ============================================================
// B:register_collector_contributions
// ============================================================

// B:register_collector_contributions — verify integration "manifest with collectors=true registers collector"
#[test]
fn register_collector_from_manifest() {
    let mut manifest = default_manifest();
    manifest.name = "@specforge/rust".to_string();
    manifest.contributes = ExtensionContributions {
        collectors: true,
        ..Default::default()
    };

    let collectors = register_collector_contributions(&[manifest]);
    assert_eq!(collectors.len(), 1);
    assert_eq!(collectors[0].extension_name, "@specforge/rust");
    assert!(collectors[0].export_name.contains("specforge__rust"));
}

// B:register_collector_contributions — verify integration "manifest without collector flag returns empty"
#[test]
fn register_collector_no_flag_empty() {
    let manifest = default_manifest();
    let collectors = register_collector_contributions(&[manifest]);
    assert!(collectors.is_empty());
}

// B:register_collector_contributions — verify integration "multiple manifests register one each"
#[test]
fn register_collectors_multiple() {
    let mut m1 = default_manifest();
    m1.name = "@ext/a".to_string();
    m1.contributes.collectors = true;

    let mut m2 = default_manifest();
    m2.name = "@ext/b".to_string();
    m2.contributes.collectors = true;

    let m3 = default_manifest(); // no collector flag

    let collectors = register_collector_contributions(&[m1, m2, m3]);
    assert_eq!(collectors.len(), 2);
}

// ============================================================
// B:dispatch_collector
// ============================================================

// B:dispatch_collector — verify integration "valid JSON output accepted"
#[test]
fn dispatch_collector_valid_json() {
    let report = serde_json::json!({"entity_results": []});
    let runtime = MockRuntime::new()
        .with_call_ok("collect__ext__test", serde_json::to_vec(&report).unwrap());

    let collector = RegisteredCollector {
        extension_name: "@ext/test".to_string(),
        command_name: "collect_ext__test".to_string(),
        export_name: "collect__ext__test".to_string(),
    };

    let result = dispatch_collector(&collector, &runtime, b"{}");
    assert!(result.is_ok());
}

// B:dispatch_collector — verify integration "invalid JSON output produces E028"
#[test]
fn dispatch_collector_invalid_json_e028() {
    let runtime = MockRuntime::new().with_call_ok("collect__ext__test", b"not json".to_vec());

    let collector = RegisteredCollector {
        extension_name: "@ext/test".to_string(),
        command_name: "collect_ext__test".to_string(),
        export_name: "collect__ext__test".to_string(),
    };

    let err = dispatch_collector(&collector, &runtime, b"{}").unwrap_err();
    assert_eq!(err.code, "E028");
    assert!(err.message.contains("invalid JSON"));
}

// B:dispatch_collector — verify integration "wasm trap produces E028"
#[test]
fn dispatch_collector_trap_e028() {
    let runtime = MockRuntime::new().with_call_trap(
        "collect__ext__test",
        WasmTrapInfo {
            kind: "oom".to_string(),
            message: "out of memory".to_string(),
            export_name: "collect__ext__test".to_string(),
        },
    );

    let collector = RegisteredCollector {
        extension_name: "@ext/test".to_string(),
        command_name: "collect_ext__test".to_string(),
        export_name: "collect__ext__test".to_string(),
    };

    let err = dispatch_collector(&collector, &runtime, b"{}").unwrap_err();
    assert_eq!(err.code, "E028");
    assert!(err.message.contains("trapped"));
}

// ============================================================
// B:auto_detect_collector
// ============================================================

// B:auto_detect_collector — verify integration "matching file pattern returns collector name"
#[test]
fn auto_detect_matching_pattern() {
    let patterns = vec![("Cargo.toml", "rust"), ("package.json", "node")];
    let files = vec!["Cargo.toml".to_string(), "src/main.rs".to_string()];
    let result = auto_detect_collector(&patterns, &files).unwrap();
    assert_eq!(result, "rust");
}

// B:auto_detect_collector — verify integration "no matching pattern returns I013"
#[test]
fn auto_detect_no_match_i013() {
    let patterns = vec![("Cargo.toml", "rust")];
    let files = vec!["go.mod".to_string()];
    let err = auto_detect_collector(&patterns, &files).unwrap_err();
    assert_eq!(err.code, "I013");
}

// B:auto_detect_collector — verify integration "first matching pattern wins"
#[test]
fn auto_detect_first_match_wins() {
    let patterns = vec![("Cargo.toml", "rust"), ("Cargo.toml", "cargo-alt")];
    let files = vec!["Cargo.toml".to_string()];
    let result = auto_detect_collector(&patterns, &files).unwrap();
    assert_eq!(result, "rust");
}

// ============================================================
// B:validate_collector_output
// ============================================================

// B:validate_collector_output — verify integration "all entity IDs known produces no diagnostics"
#[test]
fn validate_output_all_known() {
    let known: HashSet<String> = ["b1".to_string(), "b2".to_string()].into_iter().collect();
    let report = serde_json::json!({
        "entity_results": [
            {"entity_id": "b1", "status": "passed"},
            {"entity_id": "b2", "status": "failed"}
        ],
        "stats": {"total": 2, "passed": 1, "failed": 1, "skipped": 0}
    });
    let diags = validate_collector_output(&report, &known);
    assert!(diags.is_empty());
}

// B:validate_collector_output — verify integration "unknown entity ID produces W029"
#[test]
fn validate_output_unknown_entity_w029() {
    let known: HashSet<String> = ["b1".to_string()].into_iter().collect();
    let report = serde_json::json!({
        "entity_results": [
            {"entity_id": "b1", "status": "passed"},
            {"entity_id": "unknown_entity", "status": "failed"}
        ]
    });
    let diags = validate_collector_output(&report, &known);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "W029");
    assert!(diags[0].message.contains("unknown_entity"));
}

// B:validate_collector_output — verify integration "stats inconsistency produces W030"
#[test]
fn validate_output_stats_inconsistency_w030() {
    let known: HashSet<String> = HashSet::new();
    let report = serde_json::json!({
        "entity_results": [],
        "stats": {"total": 10, "passed": 3, "failed": 2, "skipped": 1}
    });
    let diags = validate_collector_output(&report, &known);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "W030");
    assert!(diags[0].message.contains("inconsistent"));
}

// ============================================================
// B:ingest_collector_report
// ============================================================

// B:ingest_collector_report — verify integration "known entity ID mapped with coverage metadata"
#[test]
fn ingest_known_entity_mapped() {
    let known: HashSet<String> = ["b1".to_string()].into_iter().collect();
    let report = serde_json::json!({
        "entity_results": [
            {
                "entity_id": "b1",
                "test_results": [
                    {"name": "test_1", "status": "passed"},
                    {"name": "test_2", "status": "failed"}
                ]
            }
        ]
    });
    let ingested = ingest_collector_report(&report, &known);
    assert_eq!(ingested.mapped_entries.len(), 1);
    assert_eq!(ingested.mapped_entries[0].0, "b1");
    assert!(ingested.unmapped_entries.is_empty());
    assert_eq!(ingested.coverage_updates.len(), 1);
    assert_eq!(
        ingested.coverage_updates[0].1,
        CoverageMetadata {
            total: 2,
            passed: 1,
            failed: 1
        }
    );
}

// B:ingest_collector_report — verify integration "unknown entity ID placed in unmapped"
#[test]
fn ingest_unknown_entity_unmapped() {
    let known: HashSet<String> = HashSet::new();
    let report = serde_json::json!({
        "entity_results": [
            {"entity_id": "nonexistent", "test_results": []}
        ]
    });
    let ingested = ingest_collector_report(&report, &known);
    assert!(ingested.mapped_entries.is_empty());
    assert_eq!(ingested.unmapped_entries.len(), 1);
}

// B:ingest_collector_report — verify integration "coverage metadata computed from test_results"
#[test]
fn ingest_coverage_metadata_computed() {
    let known: HashSet<String> = ["b1".to_string()].into_iter().collect();
    let report = serde_json::json!({
        "entity_results": [
            {
                "entity_id": "b1",
                "test_results": [
                    {"name": "t1", "status": "passed"},
                    {"name": "t2", "status": "passed"},
                    {"name": "t3", "status": "failed"},
                    {"name": "t4", "status": "skipped"}
                ]
            }
        ]
    });
    let ingested = ingest_collector_report(&report, &known);
    let cov = &ingested.coverage_updates[0].1;
    assert_eq!(cov.total, 4);
    assert_eq!(cov.passed, 2);
    assert_eq!(cov.failed, 1);
}

// B:ingest_collector_report — verify integration "empty report produces empty IngestedReport"
#[test]
fn ingest_empty_report() {
    let known: HashSet<String> = ["b1".to_string()].into_iter().collect();
    let report = serde_json::json!({});
    let ingested = ingest_collector_report(&report, &known);
    assert!(ingested.mapped_entries.is_empty());
    assert!(ingested.unmapped_entries.is_empty());
    assert!(ingested.coverage_updates.is_empty());
}

// ============================================================
// B:is_contribution_disabled (additional integration coverage)
// ============================================================

// B:is_contribution_disabled — verify integration "matching toggle returns true"
#[test]
fn contribution_disabled_matching() {
    let toggles = vec![ContributionToggle {
        extension_name: "@ext/a".to_string(),
        disabled: ["collectors".to_string()].into_iter().collect(),
    }];
    assert!(is_contribution_disabled(&toggles, "@ext/a", "collectors"));
}

// B:is_contribution_disabled — verify integration "no matching toggle returns false"
#[test]
fn contribution_not_disabled() {
    let toggles: Vec<ContributionToggle> = vec![];
    assert!(!is_contribution_disabled(
        &toggles,
        "@ext/a",
        "collectors"
    ));
}
