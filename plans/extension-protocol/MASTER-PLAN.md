# Extension Protocol Migration — Master Plan

**PRD:** PRD-001 (Extension Protocol)
**Goal:** Replace static manifest.json with live Wasm protocol (`__handshake` + `__describe`), build SDK with 24 proc macros, migrate all 4 extensions, build `@specforge/software-testing`, remove legacy path.

---

## Phase 1: Protocol Types & Wire Format
**Leaf phase** — use `/plan`

Define all JSON-serializable Rust types for the wire protocol in `specforge-wasm`. These are the shared vocabulary between host and extension.

**Deliverables:**
- `HandshakeRequest` / `HandshakeResponse` types
- `DescribeRequest` / `DescribeResponse` types
- 11 category descriptor types (EntityKindDescriptor, EdgeTypeDescriptor, FieldDescriptor, ValidationRuleDescriptor, SharedFieldDescriptor, EntityEnhancementDescriptor, SurfaceDescriptor, GrammarDescriptor, BodyParserDescriptor, CollectorDescriptor, CompilerPassDescriptor, FeatureFlagDescriptor)
- `ProtocolError` type
- Serde JSON serialization for all types
- Unit tests: round-trip serialization for every type

**Dependencies:** None
**Estimated new files:** 2-3 in `crates/specforge-wasm/src/protocol/`

---

## Phase 2: ProtocolRuntime Trait & Host Implementation
**Leaf phase** — use `/plan`

Build the host-side runtime that loads Wasm extensions via the protocol. This is the core engine.

**Deliverables:**
- `ProtocolRuntime` trait (load, handshake, describe, call_export, disconnect, is_connected)
- Concrete implementation using existing Extism/Wasmtime infrastructure
- Protocol version validation (host rejects incompatible extensions)
- Peer dependency validation
- Host import functions: `query`, `emit_diagnostic`, `resolve_ref`, `read_file`
- Sandbox enforcement at import call site
- Tests: mock Wasm modules, handshake validation, describe dispatch, error handling

**Dependencies:** Phase 1 (types)
**Estimated new files:** 3-4 in `crates/specforge-wasm/src/protocol/`

---

## Phase 3: Registry Population from Protocol
**Leaf phase** — use `/plan`

Bridge protocol responses into the existing registry system. After this phase, protocol-loaded extensions produce identical `(KindRegistry, FieldRegistry, EdgeRegistry)` output as manifest-loaded extensions.

**Deliverables:**
- `populate_from_protocol()` function — converts describe responses → registries
- Maps EntityKindDescriptor → KindRegistryEntry
- Maps FieldDescriptor → FieldRegistryEntry
- Maps EdgeTypeDescriptor → EdgeRegistryEntry
- Maps EntityEnhancementDescriptor → FieldEnhancement
- Maps ValidationRuleDescriptor → ValidationRulePattern
- Maps SurfaceDescriptor → SurfaceRegistryEntry
- Tests: protocol-populated registries match manifest-populated registries for same data

**Dependencies:** Phase 1 (types), Phase 2 (runtime)
**Estimated new files:** 1-2 in `crates/specforge-registry/src/compilation/`

---

## Phase 4: Dual-Mode Host Integration
**Leaf phase** — use `/plan`

Wire protocol loading into the compilation pipeline alongside existing manifest loading. Both paths coexist.

**Deliverables:**
- Extension detection: manifest.json present → legacy, `.wasm` with `__handshake` → protocol
- `load_extension()` dispatcher — routes to legacy or protocol path per extension
- Updated `compile()` in specforge-emitter — supports mixed extension sets
- Updated McpState initialization — supports protocol-loaded extensions
- CompilationContext carries protocol metadata alongside manifest metadata
- Tests: mixed manifest + protocol extensions compile correctly, cross-extension refs work

**Dependencies:** Phase 3 (registry population)
**Estimated modified files:** 3-4 (compile.rs, lifecycle.rs, state.rs)

---

## Phase 5: Extension SDK
**Nested phase** — contains sub-phases

Build `specforge-extension-sdk` proc macro crate. Extensions use these macros to declare their contributions, and the macros generate the `__handshake` and `__describe` Wasm exports.

### Phase 5.1: SDK Core Types & Runtime Library
**Leaf phase** — use `/plan`

- `HostApi` struct (query, emit_diagnostic, resolve_ref, read_file)
- Host import function bindings (extern "C" wrappers)
- `Entity`, `EntityRef`, `Diagnostic`, `ParseResult`, `CollectionResult` types
- `Block`, `FieldValue` types
- `TestExtension` test harness struct

### Phase 5.2: Handshake Macros
**Leaf phase** — use `/plan`

- `#[extension]` → generates `__handshake` export with name, version, contribution_flags
- `#[sandbox]` → sandbox policy in handshake response
- `#[peer_dependency]` → peer dependencies in handshake response
- Expansion snapshot tests (trybuild + insta)

### Phase 5.3: Entity & Field Macros
**Leaf phase** — use `/plan`

- `#[entity_kind]` → EntityKindDescriptor, contributes to `__describe("entities")`
- `#[field]` → FieldDescriptor (type inference from Rust types)
- `#[edge]` → EdgeTypeDescriptor from reference field
- `#[shared_field]` → SharedFieldDescriptor
- `#[lsp]` → semantic_token, icon metadata
- `#[dot]` → shape, color, fillcolor metadata
- Expansion snapshot tests

### Phase 5.4: Enhancement & Validation Macros
**Leaf phase** — use `/plan`

- `#[enhance]` → EntityEnhancementDescriptor for foreign entity kinds
- `#[edge_type]` → standalone EdgeTypeDescriptor (not from field)
- `#[validation_rule]` → declarative ValidationRuleDescriptor
- `#[validator]` → `validate__*` export + custom rule descriptor
- Expansion snapshot tests

### Phase 5.5: Surface Macros
**Leaf phase** — use `/plan`

- `#[cli_command]` → `cmd__*` export + CommandDescriptor
- `#[mcp_tool]` → `mcp__*` export + McpToolDescriptor
- `#[mcp_resource]` → `mcp__*` export + McpResourceDescriptor
- `#[sandbox_override]` → per-surface sandbox
- `#[arg]` → argument descriptors on function params
- Expansion snapshot tests

### Phase 5.6: Grammar, Parser, Collector, Pass Macros
**Leaf phase** — use `/plan`

- `#[grammar]` → GrammarDescriptor
- `#[body_parser]` → `parse__*` export + BodyParserDescriptor
- `#[collector]` → `collect__*` export + CollectorDescriptor
- `#[auto_detect]` → auto-detection config on collector
- `#[compiler_pass]` → CompilerPassDescriptor
- `#[feature_flag]` → FeatureFlagDescriptor
- Expansion snapshot tests

---

## Phase 6: Golden Test Extension
**Leaf phase** — use `/plan`

Build a test-only extension exercising all 24 macros across all 11 describe categories. Compile to `wasm32-wasi`. Load via ProtocolRuntime in tests.

**Deliverables:**
- `crates/specforge-test-extension/` — test-only crate
- Uses every SDK macro at least once
- Declares 2 entity kinds, 2 edge types, 3 fields, 1 enhancement, 2 validation rules, 1 CLI command, 1 MCP tool, 1 collector
- Integration test: load via ProtocolRuntime, verify all registries populated correctly
- Test: handshake returns valid metadata
- Test: each describe category returns correct descriptors
- Test: host API calls work (query, emit_diagnostic, resolve_ref, read_file)

**Dependencies:** Phase 5 (SDK), Phase 2 (ProtocolRuntime)

---

## Phase 7: Extension Migrations
**Nested phase** — contains sub-phases

Migrate each extension from manifest.json to SDK-based Wasm protocol. Order: simplest first.

### Phase 7.1: Migrate @specforge/governance
**Leaf phase** — use `/plan`

Simplest extension: 3 entity kinds, 11 edge types, ~7 validation rules, no enhancements.
- Create `extensions/governance/src/lib.rs` with SDK macros
- Compile to `wasm32-wasi`
- Verify: existing governance tests pass with protocol-loaded extension
- Delete `extensions/governance/manifest.json`

### Phase 7.2: Migrate @specforge/formal
**Leaf phase** — use `/plan`

5 entity kinds, 12 edge types, 2 entity enhancements (behavior, event), 4 compiler passes.
- Create `extensions/formal/src/lib.rs` with SDK macros
- Compile to `wasm32-wasi`
- Verify: existing formal tests pass
- Delete `extensions/formal/manifest.json`

### Phase 7.3: Migrate @specforge/product
**Leaf phase** — use `/plan`

Root extension: 9 entity kinds, 20 edge types, ~24 validation rules, no enhancements, no dependencies.
- Create `extensions/product/src/lib.rs` with SDK macros
- Compile to `wasm32-wasi`
- Verify: existing product tests pass
- Delete `extensions/product/manifest.json`

### Phase 7.4: Migrate @specforge/software
**Leaf phase** — use `/plan`

5 entity kinds, 14 edge types, 12 validation rules, 2 entity enhancements (milestone, module from product).
- Create `extensions/software/src/lib.rs` with SDK macros
- Compile to `wasm32-wasi`
- Verify: existing software tests pass
- Delete `extensions/software/manifest.json`

### Phase 7.5: Build @specforge/software-testing
**Leaf phase** — use `/plan`

New enhancement-only extension (PRD-003): 0 entity kinds, 1 edge type, 13 entity enhancements, 1 validation rule, 1 collector.
- Create `extensions/software-testing/src/lib.rs` with SDK macros
- Compile to `wasm32-wasi`
- Tests: gherkin field on 13 kinds, W004 rule, TestedBy edge, Cucumber collector

---

## Phase 8: Legacy Removal & Cleanup
**Leaf phase** — use `/plan`

Remove all manifest.json-based loading infrastructure.

**Deliverables:**
- Delete `load_extension_manifests()` from compile.rs
- Delete manifest.json validation functions
- Delete `ManifestV2` struct (or keep as SDK-internal type)
- Remove legacy WasmRuntime trait methods that are superseded
- Remove manifest.json files from all extensions
- Update all tests to use protocol path only
- CI compliance check: grep for manifest.json references in core crates
- Final: `cargo test --workspace` all green, `cargo clippy --workspace` zero warnings

**Dependencies:** Phase 7 (all migrations complete)

---

## Dependency Graph

```
Phase 1 (Types)
    │
    ▼
Phase 2 (ProtocolRuntime)
    │
    ▼
Phase 3 (Registry Population)
    │
    ▼
Phase 4 (Dual-Mode Host)
    │
    ├──────────────────────┐
    ▼                      ▼
Phase 5 (SDK)          [existing tests still pass]
    │
    ▼
Phase 6 (Golden Test)
    │
    ▼
Phase 7 (Migrations)
    ├── 7.1 governance
    ├── 7.2 formal
    ├── 7.3 product
    ├── 7.4 software
    └── 7.5 software-testing
    │
    ▼
Phase 8 (Legacy Removal)
```
