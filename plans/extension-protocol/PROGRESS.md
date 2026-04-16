# Extension Protocol Migration — Progress Tracker

**Started:** 2026-04-12
**Target:** Full migration from manifest.json to Wasm protocol

---

## Overall Progress

| Phase | Description | Status | Tests |
|-------|-------------|--------|-------|
| 1 | Protocol Types & Wire Format | DONE | 49 |
| 2 | ProtocolRuntime Trait & Host | DONE | 16 |
| 3 | Registry Population from Protocol | DONE | 16 |
| 4 | Dual-Mode Host Integration | DONE | 14 |
| 4.5 | BuiltinRuntime (manifest.json → protocol) | DONE | 7 + existing |
| 5 | Extension SDK | NOT STARTED | - |
| 5.1 | SDK Core Types & Runtime Library | NOT STARTED | - |
| 5.2 | Handshake Macros | NOT STARTED | - |
| 5.3 | Entity & Field Macros | NOT STARTED | - |
| 5.4 | Enhancement & Validation Macros | NOT STARTED | - |
| 5.5 | Surface Macros | NOT STARTED | - |
| 5.6 | Grammar, Parser, Collector, Pass Macros | NOT STARTED | - |
| 6 | Golden Test Extension | NOT STARTED | - |
| 7 | Extension Migrations | NOT STARTED | - |
| 7.1 | Migrate @specforge/governance | NOT STARTED | - |
| 7.2 | Migrate @specforge/formal | NOT STARTED | - |
| 7.3 | Migrate @specforge/product | NOT STARTED | - |
| 7.4 | Migrate @specforge/software | NOT STARTED | - |
| 7.5 | Build @specforge/software-testing | NOT STARTED | - |
| 8 | Legacy Removal & Cleanup | NOT STARTED | - |

---

## Phase Details

### Phase 1: Protocol Types & Wire Format ✓
- [x] HandshakeRequest / HandshakeResponse types
- [x] DescribeRequest / DescribeResponse types
- [x] ContributionFlags type
- [x] SandboxPolicy type
- [x] PeerDependency type
- [x] EntityKindDescriptor type
- [x] FieldDescriptor type
- [x] EdgeTypeDescriptor type
- [x] SharedFieldDescriptor type
- [x] EntityEnhancementDescriptor type
- [x] ValidationRuleDescriptor type
- [x] SurfaceDescriptor (commands, mcp_tools, mcp_resources)
- [x] GrammarDescriptor type
- [x] BodyParserDescriptor type
- [x] CollectorDescriptor type
- [x] CompilerPassDescriptor type
- [x] FeatureFlagDescriptor type
- [x] ProtocolError type
- [x] Round-trip serialization tests for all types (49 tests)

### Phase 2: ProtocolHost & Extension Loading ✓
- [x] ProtocolHost struct (wraps &dyn WasmRuntime)
- [x] handshake() — call __handshake, parse HandshakeResponse
- [x] describe() — call __describe(category), parse DescribeResponse
- [x] describe_all() — iterate categories based on ContributionFlags
- [x] validate_protocol_version() — check host/extension version match
- [x] validate_peer_dependencies() — check required deps loaded
- [x] ExtensionDescriptions aggregate type (12 categories)
- [x] ProtocolExtension type (handshake + descriptions)
- [x] load_protocol_extension() orchestrator
- [x] Error handling: trap, bad JSON, incompatible version, unsupported category
- [x] Tests with MockRuntime (category-aware, 16 tests)
- Note: Host imports (query, emit_diagnostic, resolve_ref, read_file) already exist in host_functions.rs — reused by protocol path, no new code needed
- Note: Sandbox enforcement already exists in sandbox.rs — reused, no new code needed
- Note: ProtocolHost is a struct not a trait — single implementation, testable via MockRuntime

### Phase 3: Registry Population from Protocol ✓
- [x] protocol_extension_to_manifest() bridge function
- [x] populate_from_protocol() function (delegates to populate_registries)
- [x] protocol_surfaces_to_manifest() function
- [x] EntityKindDescriptor → ManifestEntityKind (with keyword fallback)
- [x] FieldDescriptor → ManifestField
- [x] EdgeTypeDescriptor → ManifestEdgeType
- [x] EntityEnhancementDescriptor → FieldEnhancement
- [x] ValidationRuleDescriptor → ManifestValidationRule (severity enum→string)
- [x] SharedFieldDescriptor → ManifestField (shared fields)
- [x] ContributionFlags → ExtensionContributions
- [x] PeerDependency → PeerDependency (cross-crate)
- [x] SandboxPolicy → SandboxPolicy (cross-crate)
- [x] SurfaceDescriptor → SurfaceContributions (commands, tools, resources)
- [x] GrammarDescriptor → GrammarContribution
- [x] BodyParserDescriptor → BodyParserContribution
- [x] CollectorDescriptor → CollectorContribution (with auto_detect)
- [x] Parity test: protocol path produces identical registries to manifest path (16 tests)

### Phase 4: Dual-Mode Host Integration ✓
- [x] `ExtensionMode` enum + `detect_extension_mode()` + `find_wasm_binary()` (`protocol/detect.rs`)
- [x] `compile_with_runtime()` public API (accepts optional `WasmRuntime`)
- [x] `compile()` delegates to `compile_with_runtime(path, None)` — backward compatible
- [x] `load_extensions_dual_mode()` replaces `load_extension_manifests()`
- [x] Manifest path: detect → parse manifest.json → validate → ManifestV2
- [x] Protocol path: detect → find .wasm → load_wasm_module → ProtocolHost → load → bridge → ManifestV2
- [x] W031 diagnostic when protocol extension found but no runtime available
- [x] W030 diagnostic for empty extension dirs (neither manifest.json nor .wasm)
- [x] E031 diagnostic for protocol failures (trap, version mismatch, bad JSON)
- [x] Protocol errors do not prevent manifest extensions from loading
- [x] `specforge-wasm` added as dependency of `specforge-emitter`
- [x] `compile_with_runtime` exported from `specforge-emitter` public API
- [x] Tests: 6 detection unit tests + 8 dual-mode integration tests (14 total)
- [x] `cargo clippy --workspace` — zero warnings

### Phase 4.5: BuiltinRuntime — Replace manifest.json with Protocol-Native Rust ✓
- [x] `BuiltinExtension` trait + `BuiltinRuntime` implementing `WasmRuntime` (`specforge-wasm/src/builtin.rs`)
- [x] ProductExtension: 9 kinds, 20 edges, 24 rules, 1 shared field (`specforge-emitter/src/builtins/product.rs`)
- [x] GovernanceExtension: 3 kinds, 11 edges, 7 rules (`specforge-emitter/src/builtins/governance.rs`)
- [x] SoftwareExtension: 5 kinds, 14 edges, 12 rules, 2 enhancements (`specforge-emitter/src/builtins/software.rs`)
- [x] FormalExtension: 5 kinds, 12 edges, 1 rule, 2 enhancements (`specforge-emitter/src/builtins/formal.rs`)
- [x] `default_runtime()` registers all 4 extensions (`specforge-emitter/src/builtins/mod.rs`)
- [x] `compile()` uses `default_runtime()` — protocol is the only loading path
- [x] `load_extensions()` replaces `load_extensions_dual_mode()` — protocol-only, no manifest.json
- [x] `normalize_extension_name()` converts path-style specifiers to `@specforge/` canonical names
- [x] Deleted 4 `extensions/*/manifest.json` files
- [x] Deleted `load_manifest_extension()`, `load_extensions_dual_mode()` from compile.rs
- [x] LSP `load_registries()` migrated from manifest.json to BuiltinRuntime protocol pipeline
- [x] `EmitterError` structured error type replaces raw `String` errors
- [x] Protocol parity tests for all 4 extensions (builtins.rs)
- [x] Dual-mode tests updated (manifest-specific tests removed)
- [x] `cargo test --workspace` — zero failures
- [x] `cargo clippy --workspace` — zero warnings

### Phase 5.1: SDK Core Types
- [ ] specforge-extension-sdk crate created
- [ ] HostApi struct (query, emit_diagnostic, resolve_ref, read_file)
- [ ] Host import extern "C" bindings
- [ ] Entity type
- [ ] EntityRef type
- [ ] Diagnostic type
- [ ] ParseResult type
- [ ] CollectionResult type
- [ ] Block / FieldValue types
- [ ] TestExtension harness

### Phase 5.2: Handshake Macros
- [ ] #[extension] macro
- [ ] #[sandbox] macro
- [ ] #[peer_dependency] macro
- [ ] __handshake export generation
- [ ] Expansion snapshot tests

### Phase 5.3: Entity & Field Macros
- [ ] #[entity_kind] macro
- [ ] #[field] macro (type inference)
- [ ] #[edge] macro (reference → edge)
- [ ] #[shared_field] macro
- [ ] #[lsp] macro
- [ ] #[dot] macro
- [ ] __describe("entities") generation
- [ ] __describe("edges") generation
- [ ] __describe("fields") generation
- [ ] Expansion snapshot tests

### Phase 5.4: Enhancement & Validation Macros
- [ ] #[enhance] macro
- [ ] #[edge_type] macro
- [ ] #[validation_rule] macro
- [ ] #[validator] macro
- [ ] __describe("enhancements") generation
- [ ] __describe("validation_rules") generation
- [ ] validate__* export generation
- [ ] Expansion snapshot tests

### Phase 5.5: Surface Macros
- [ ] #[cli_command] macro
- [ ] #[mcp_tool] macro
- [ ] #[mcp_resource] macro
- [ ] #[sandbox_override] macro
- [ ] #[arg] macro
- [ ] __describe("surfaces") generation
- [ ] cmd__* export generation
- [ ] mcp__* export generation
- [ ] Expansion snapshot tests

### Phase 5.6: Grammar, Parser, Collector, Pass Macros
- [ ] #[grammar] macro
- [ ] #[body_parser] macro
- [ ] #[collector] macro
- [ ] #[auto_detect] macro
- [ ] #[compiler_pass] macro
- [ ] #[feature_flag] macro
- [ ] __describe("grammars") generation
- [ ] __describe("body_parsers") generation
- [ ] __describe("collectors") generation
- [ ] __describe("passes") generation
- [ ] __describe("feature_flags") generation
- [ ] parse__* export generation
- [ ] collect__* export generation
- [ ] Expansion snapshot tests

### Phase 6: Golden Test Extension
- [ ] crates/specforge-test-extension/ created
- [ ] Uses all 24 SDK macros
- [ ] Compiles to wasm32-wasi
- [ ] Integration test: ProtocolRuntime loads it
- [ ] Test: handshake correct
- [ ] Test: all 11 describe categories correct
- [ ] Test: host API calls work
- [ ] Test: registries populated correctly

### Phase 7.1: Migrate @specforge/governance
- [ ] extensions/governance/src/lib.rs created
- [ ] 3 entity kinds with SDK macros
- [ ] 11 edge types
- [ ] ~7 validation rules
- [ ] Compiles to wasm32-wasi
- [ ] Existing governance tests pass
- [ ] manifest.json deleted

### Phase 7.2: Migrate @specforge/formal
- [ ] extensions/formal/src/lib.rs created
- [ ] 5 entity kinds with SDK macros
- [ ] 12 edge types
- [ ] 2 entity enhancements (behavior, event)
- [ ] 4 compiler passes
- [ ] ~47 validation rules
- [ ] Compiles to wasm32-wasi
- [ ] Existing formal tests pass
- [ ] manifest.json deleted

### Phase 7.3: Migrate @specforge/product
- [ ] extensions/product/src/lib.rs created
- [ ] 9 entity kinds with SDK macros
- [ ] 20 edge types
- [ ] ~24 validation rules
- [ ] Compiles to wasm32-wasi
- [ ] Existing product tests pass
- [ ] manifest.json deleted

### Phase 7.4: Migrate @specforge/software
- [ ] extensions/software/src/lib.rs created
- [ ] 5 entity kinds with SDK macros
- [ ] 14 edge types
- [ ] 12 validation rules
- [ ] 2 entity enhancements (milestone, module)
- [ ] Compiles to wasm32-wasi
- [ ] Existing software tests pass
- [ ] manifest.json deleted

### Phase 7.5: Build @specforge/software-testing
- [ ] extensions/software-testing/src/lib.rs created
- [ ] 0 entity kinds (enhancement-only)
- [ ] 1 edge type (TestedBy)
- [ ] 13 entity enhancements (gherkin field)
- [ ] 1 validation rule (W004)
- [ ] 1 collector (Cucumber)
- [ ] Compiles to wasm32-wasi
- [ ] Tests: gherkin on 13 kinds, W004, TestedBy, collector

### Phase 8: Legacy Removal & Cleanup
- [ ] Delete load_extension_manifests()
- [ ] Delete manifest validation functions
- [ ] Remove ManifestV2 from public API (or keep as internal)
- [ ] Remove legacy WasmRuntime methods superseded by ProtocolRuntime
- [ ] Delete all manifest.json files
- [ ] Update all tests to protocol path
- [ ] CI compliance: no manifest.json refs in core
- [ ] cargo test --workspace: all green
- [ ] cargo clippy --workspace: zero warnings

---

## Session Log

| Date | Phase | Work Done |
|------|-------|-----------|
| 2026-04-12 | Setup | Master plan and progress tracker created |
| 2026-04-12 | Phase 1 | All 28 protocol types + ProtocolError implemented, 49 tests, 0 clippy warnings |
| 2026-04-12 | Phase 2 | ProtocolHost with handshake/describe/describe_all/validate + load_protocol_extension, 16 tests |
| 2026-04-12 | Phase 3 | Bridge pattern: protocol_extension_to_manifest + populate_from_protocol + protocol_surfaces_to_manifest, 16 tests, parity verified |
| 2026-04-13 | Phase 4 | Dual-mode host integration: detect.rs + compile_with_runtime + load_extensions_dual_mode, 14 tests, zero clippy warnings |
| 2026-04-15 | Phase 4.5 | BuiltinRuntime + 4 built-in extensions (product, software, governance, formal), default_runtime(), compile() protocol-only, deleted manifest.json, LSP fix, EmitterError |
