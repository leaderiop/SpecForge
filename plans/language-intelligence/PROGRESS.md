# Language Intelligence Extensions — Progress Tracker

## Status: DONE

| Phase | Description | Status | Notes |
|-------|-------------|--------|-------|
| 1 | Deduplicate and isolate hardcoded knowledge | DONE | |
| 2 | Add analyzer contribution protocol to ManifestV2 | DONE | |
| 3 | Ship @specforge/rust scanner (Wasm) | DONE | |
| 4 | Remove hardcoded fallback | DONE | |

## Phase 1: Deduplicate and Isolate (no behavior change)

- [x] 1a. Add `SourceDiscoveryConfig` struct to `specforge-common/src/inference/discovery.rs`
- [x] 1b. Add `SourceDiscoveryConfig::hardcoded_defaults()` returning today's 13 extensions and 8 excluded dirs
- [x] 1c. Move `discover_source_files`, `walk_source_dir`, `is_source_file`, `is_excluded_dir` into `specforge-common/src/inference/discovery.rs` as public functions taking `&SourceDiscoveryConfig`
- [x] 1d. Delete local copies from `infer_status.rs`, `infer_progress.rs`, `prompts/infer.rs` — replace with calls to shared implementation
- [x] 1e. Move `scan_rust_pub_items`, `parse_rust_pub_item`, `to_snake_case`, `is_test_or_build_file` into `specforge-common/src/inference/fallback_rust.rs` (deprecated)
- [x] 1f. Update `specforge-common/src/lib.rs` exports
- [x] 1g. Remove `.ends_with(".rs")` filter from `compute_inference_diagnostics` (density applies to all languages)
- [x] 1h. Run all tests — behavior is identical, code is in one place

## Phase 2: Analyzer Contribution Protocol

- [x] 2a. Add `AnalyzerContribution` struct to `specforge-registry/src/manifest/types.rs`
- [x] 2b. Add `analyzer_contributions: Vec<AnalyzerContribution>` to `ManifestV2`
- [x] 2c. Add `analyzers: bool` to `ExtensionContributions` in `specforge-registry`
- [x] 2d. Add `AnalyzerDescriptor` to `specforge-wasm/src/protocol/types.rs`
- [x] 2e. Add `analyzers: bool` to `ContributionFlags` in `specforge-wasm`
- [x] 2f. Add `"analyzers"` to `SUPPORTED_CATEGORIES` (now 13)
- [x] 2g. Add `CallSite::Analyzer` to host function permissions (read_file + emit_diagnostic + query_graph)
- [x] 2h. Add `SourceDiscoveryConfig::from_analyzer_configs()` with `AnalyzerConfig` input type
- [x] 2i. Rename `InferenceGap` → `SourceItem`, `InferenceGapReport` → `GapReport`
- [x] 2j. Add `scanner` field to `SourceItem`, `scanners_used` to `GapReport`
- [x] 2k. Add scanner protocol types: `ScanRequest`, `ScanResponse`, `ClassifyRequest`, `ClassifyResponse`, `MapSymbolRequest`, `MapSymbolResponse`
- [x] 2l. Run all tests — 0 failures, clippy clean

## Phase 3: Ship @specforge/rust Scanner

- [x] 3a. Declare `analyzer_contributions` in `@specforge/rust` manifest (builtin extension)
- [x] 3b. Implement `scan__rust` via `call_analyzer` on `BuiltinExtension` trait
- [x] 3c. Implement `classify__rust` — classifies items into behavior/type/port/event
- [x] 3d. Implement `map__rust` — snake_case mapping with exact/generated strategies
- [x] 3e. Integration test: parity verified — same items as fallback_rust for identical source
- [x] 3f. Source anchoring: `specforge-anchors.json` with `AnchorManifest`, bidirectional lookups
- [x] 3g. MCP tools: `specforge.find_implementation`, `specforge.find_spec_for_source`

## Phase 4: Remove Fallback

- [x] 4a. Delete `specforge-common/src/inference/fallback_rust.rs`
- [x] 4b. Remove `hardcoded_defaults()` from `SourceDiscoveryConfig`
- [x] 4c. Update gap analysis label from "approximate, Rust only" to scanner-derived
- [x] 4d. Grep codebase for `.rs"`, `"rust"`, `"Rust"` in inference paths — zero hits in production code
- [x] 4e. Run full test suite — 0 failures, clippy clean

## Phase 5: Generic Scanner Dispatch + @specforge/typescript

- [x] 5a. Create `scanner_dispatch.rs` in specforge-emitter — generic multi-scanner dispatch from `analyzer_contributions`
- [x] 5b. Update `infer_gaps.rs` and `infer_status.rs` to use `scan_source_files()` instead of hardcoded `RustExtension`
- [x] 5c. Update `infer_progress.rs` and `prompts/infer.rs` to derive discovery config from manifests
- [x] 5d. Implement `TypeScriptExtension` — 14 export patterns, classification heuristics, test file detection (28 unit tests)
- [x] 5e. Register in `default_runtime()`, protocol round-trip + scanner integration tests
- [x] 5f. Scanner dispatch tests — 6 tests covering single/multi scanner, empty inputs, missing files
- [x] 5g. MCP test infrastructure — mixed-language `infer_progress_discovers_both_rust_and_typescript` test
- [x] 5h. Full test suite — 0 failures across all 57 test suites
