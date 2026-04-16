# PRD-001: Extension Protocol

**Status:** Draft
**Author:** Mohammad AL Mechkor
**Date:** 2026-04-12

---

## Problem Statement

SpecForge extensions currently declare their capabilities through hand-written `manifest.json` files that live alongside Wasm binaries. This creates a dual source of truth: the extension's Rust code defines its actual behavior, while a separate JSON file declares its metadata, entity kinds, edge types, validation rules, and surface contributions. These two sources can drift apart silently. A manifest can declare an entity kind the code doesn't handle, or the code can implement a validator the manifest doesn't register.

Beyond drift, the manifest approach has fundamental limitations:

1. **No bidirectional communication.** Extensions cannot query the host's entity graph, emit diagnostics, or resolve cross-extension references during validation or command execution. The host calls extensions; extensions cannot call back.

2. **No runtime flexibility.** Extensions are loaded at startup by parsing JSON. Adding or removing an extension requires restarting the CLI, LSP, or MCP server. There is no hot plug/unplug.

3. **Context-blind loading.** The host loads all manifest metadata regardless of context. The CLI loads MCP tool schemas it will never use. The LSP loads DOT visualization colors it will never render. There is no way for the host to request only the metadata categories it needs.

4. **Testing is hardcoded into the core extensions.** The `testable`, `supportsVerify`, `allowedVerifyKinds`, `tests`, `gherkin`, and `TestedBy` constructs are baked into `@specforge/software` and scattered across all four manifests. Projects that don't use BDD testing cannot opt out.

5. **Extension authoring is error-prone.** Writing a manifest.json by hand requires knowing 18 contribution categories, correct camelCase field names, cross-referencing edge labels with field declarations, and manually keeping validation rule codes unique. There is no compile-time validation of the manifest against the extension code.

## Solution

Replace the static `manifest.json` approach with a **live Wasm discovery protocol** and a **Rust SDK with attribute macros** that generate protocol-conformant Wasm exports from declarative code.

The protocol establishes a multi-step negotiation between the host and each extension:

1. **Handshake** (`__handshake`): Extension identifies itself, declares which contribution categories it provides, its sandbox policy, and peer dependencies.

2. **Describe** (`__describe(category)`): Host selectively requests metadata for each category the extension flagged. The host only requests categories relevant to the current context (CLI, LSP, MCP).

3. **Operate**: Host invokes named exports (`cmd__*`, `validate__*`, `mcp__*`, `parse__*`, `collect__*`) as needed. Extensions can call host-imported functions (`query`, `emit_diagnostic`, `resolve_ref`, `read_file`) during execution.

4. **Disconnect**: Extension is unloaded gracefully. Its entities become untyped nodes, its validation rules stop firing, and its surfaces are removed. Reconnection restores full semantics.

The `specforge-extension-sdk` crate provides 24 attribute macros covering all 18 manifest contribution types plus metadata. Extension authors write Rust structs and functions annotated with `#[extension]`, `#[entity_kind]`, `#[field]`, `#[edge]`, `#[validator]`, `#[cli_command]`, etc. The SDK generates all Wasm exports. No hand-written JSON. The Rust compiler catches inconsistencies at build time.

As part of this initiative, all testing concerns (`testable`, `supportsVerify`, `allowedVerifyKinds`, `tests`, `gherkin`, `TestedBy`, `W004`, `W009`) are extracted from the four core extensions into a new `@specforge/software-testing` enhancement-only extension. Projects that don't use BDD testing simply don't install it.

## User Stories

1. As an extension author, I want to declare entity kinds with Rust struct attributes, so that the compiler catches field type mismatches and missing metadata at build time instead of at runtime.

2. As an extension author, I want a single `specforge-extension-sdk` dependency in my Cargo.toml, so that I don't need to learn the manifest JSON schema or the Wasm export naming conventions.

3. As an extension author, I want to call `host().query("kind:behavior")` from inside a validator function, so that my custom validation rules can inspect the full entity graph, not just the single entity being validated.

4. As an extension author, I want to call `host().emit_diagnostic(...)` from inside a CLI command, so that my command's output integrates with SpecForge's unified diagnostic system (terminal, JSON, LSP).

5. As an extension author, I want to call `host().resolve_ref("entity_id")` to look up cross-extension entity references, so that my validator can check whether referenced entities actually exist in the graph.

6. As an extension author, I want to call `host().read_file("tests/auth.feature")` to read project files, so that my collector can parse test result files discovered by auto-detection patterns.

7. As an extension author, I want to write `#[validator(code = "W050", ...)]` on a function, so that the SDK generates the `validate__*` Wasm export and the validation rule descriptor without manual wiring.

8. As an extension author, I want to write `#[cli_command(id = "validate", ...)]` on a function, so that the SDK generates the `cmd__validate` Wasm export, CLI argument descriptors, and automatic MCP tool promotion.

9. As an extension author, I want to write `#[mcp_tool(name = "model", ...)]` on a function, so that the SDK generates the `mcp__model` Wasm export and JSON Schema input validation.

10. As an extension author, I want to write `#[enhance(target_kind = "module", owner = "@specforge/product")]` on a struct, so that my extension adds fields to entity kinds owned by other extensions without modifying their code.

11. As an extension author, I want to write `#[compiler_pass(name = "condition_check", after = "resolve")]` on a function, so that my extension contributes a compiler pass that runs in the correct order relative to other passes.

12. As an extension author, I want to write `#[collector(name = "rust", formats = ["junit-xml"])]` on a function, so that the SDK generates the `collect__rust` Wasm export and auto-detection configuration.

13. As an extension author, I want to test my extension with `cargo test` using native Rust tooling for logic, and with the SDK's `TestExtension::load(wasm_path)` harness for protocol conformance.

14. As an extension author, I want to run `cargo build --target wasm32-wasi --release` and get a single `.wasm` file that the host can load, so that my extension is self-contained with no sidecar JSON files.

15. As a SpecForge host developer, I want the host to call `__handshake(host_version)` before anything else, so that I can check protocol version compatibility and reject incompatible extensions early.

16. As a SpecForge host developer, I want the host to call `__describe("entities")` only when it needs entity metadata, so that the CLI doesn't waste time loading LSP-specific metadata it will never use.

17. As a SpecForge host developer, I want the host to support both manifest.json and protocol-based extensions simultaneously, so that extensions can migrate incrementally without a big-bang switch.

18. As a SpecForge host developer, I want a `ProtocolRuntime` trait separate from the existing `WasmRuntime` trait, so that the protocol's handshake/describe semantics don't leak into the existing dispatch model.

19. As a SpecForge host developer, I want graceful degradation when an extension disconnects at runtime, so that entities from that extension become untyped nodes rather than causing errors.

20. As a SpecForge host developer, I want the host to emit `I004` diagnostics when entities reference disconnected extensions, so that users understand why validation is incomplete rather than seeing silent gaps.

21. As an LSP user, I want extensions to connect and disconnect without restarting the LSP server, so that I can install or update an extension and see the effects immediately.

22. As an MCP server operator, I want extensions to be hot-pluggable, so that adding or removing extensions doesn't require restarting the server or dropping active connections.

23. As a project author who doesn't use BDD testing, I want to omit `@specforge/software-testing` from my extensions list, so that I don't see `gherkin` fields or `W004` warnings in my output.

24. As a project author who uses BDD testing, I want to install `@specforge/software-testing`, so that all 13 supported entity kinds gain the `gherkin` field and test result collection.

25. As a SpecForge contributor, I want the golden test extension to exercise every SDK macro and every protocol category, so that protocol conformance regressions are caught by CI.

26. As a SpecForge contributor, I want macro expansion snapshot tests, so that changes to SDK macros produce visible diffs in the generated code rather than silent behavioral changes.

27. As an extension author, I want the SDK to derive Wasm export names from function names following a consistent convention (`cmd__`, `validate__`, `mcp__`, `parse__`, `collect__`), so that I don't need to manually assign export names.

28. As an extension author, I want `#[sandbox_override(fs_read = true)]` on a specific CLI command, so that a generally sandboxed extension can grant filesystem access to one command that needs it.

29. As an extension author, I want `#[feature_flag(name = "warning_level", ...)]` to declare configurable flags, so that users can tune my extension's behavior via `specforge.json` without code changes.

30. As a SpecForge host developer, I want all data crossing the Wasm boundary to use JSON serialization, so that protocol interactions are debuggable and inspectable without binary decoding tools.

31. As an extension author, I want `#[edge_type(label = "References")]` for standalone edge types that exist independently of any field, so that edges computed by validators or compiler passes can be declared.

32. As an extension author, I want `#[grammar(entity_kind = "type", wasm_path = "...")]` to contribute a Tree-sitter grammar for custom entity body parsing, so that my extension can define new syntax for entity content.

33. As an extension registry user, I want `specforge add @specforge/software` to download a published extension by name, so that I don't need to build extensions from source.

## Implementation Decisions

### Major Modules

**1. ProtocolRuntime (new trait in `specforge-wasm`)**

A new `ProtocolRuntime` trait that understands the handshake/describe protocol. Coexists with the existing `WasmRuntime` trait during the transition period. The `ProtocolRuntime` loads a Wasm module, calls `__handshake`, validates protocol version compatibility, calls `__describe` per flagged category, and populates registries. It exposes Host API functions as Wasm imports (`query`, `emit_diagnostic`, `resolve_ref`, `read_file`).

**2. specforge-extension-sdk (new crate in workspace)**

A proc-macro crate plus a runtime library. The proc macros (`#[extension]`, `#[entity_kind]`, `#[field]`, `#[edge]`, `#[enhance]`, `#[validation_rule]`, `#[validator]`, `#[cli_command]`, `#[mcp_tool]`, `#[mcp_resource]`, `#[grammar]`, `#[body_parser]`, `#[collector]`, `#[compiler_pass]`, `#[feature_flag]`, `#[sandbox]`, `#[sandbox_override]`, `#[peer_dependency]`, `#[arg]`, `#[auto_detect]`, `#[lsp]`, `#[dot]`, `#[shared_field]`, `#[edge_type]`) generate Wasm exports. The runtime library provides `HostApi` bindings, shared types (`Entity`, `EntityRef`, `Diagnostic`, `Graph`), and the test harness.

**3. Dual-mode host loader**

The host extension loader detects whether a directory contains `manifest.json` (legacy mode) or a `.wasm` binary with `__handshake` export (protocol mode). Both paths produce the same `(KindRegistry, FieldRegistry, EdgeRegistry, Vec<Diagnostic>)` output. This allows per-extension migration with no big-bang cutover.

**4. Host API Wasm imports**

Four functions imported by extension Wasm modules: `query(pattern) -> Vec<Entity>`, `emit_diagnostic(severity, code, message)`, `resolve_ref(id) -> Option<EntityRef>`, `read_file(path) -> Option<String>`. All serialized as JSON across the Wasm boundary. Subject to the extension's sandbox policy.

**5. Extension migration (4 extensions)**

Convert `@specforge/product`, `@specforge/software`, `@specforge/governance`, and `@specforge/formal` from manifest.json to SDK Rust code. Each becomes a standalone Rust crate in `extensions/*/` that compiles to `wasm32-wasi`. The manifest.json files are deleted after migration.

**6. @specforge/software-testing (new extension)**

Enhancement-only extension. Declares no entity kinds. Adds the `gherkin` field (string_list, file_reference=true) to 13 entity kinds across 4 extensions. Owns the `TestedBy` edge type, the `W004` validation rule, and the Cucumber/Gherkin test result collector.

**7. Golden test extension**

A test-only extension that exercises every SDK macro and every protocol category. Used for end-to-end protocol conformance testing in CI. Not published or shipped.

### Architectural Decisions

- **JSON serialization** across the Wasm boundary. All host API calls and protocol responses use JSON strings. This matches existing specforge patterns, is debuggable, and is trivially versionable.

- **New ProtocolRuntime trait** rather than extending `WasmRuntime`. The protocol's multi-step negotiation (handshake -> describe -> register) is fundamentally different from the existing single-step dispatch model. Separate traits keep both clean.

- **Dual-mode host** during transition. The host auto-detects whether an extension uses manifest.json or the protocol. Both paths feed the same registry population pipeline. No flag or config required.

- **Extensions in `extensions/*/`** as standalone Rust crates in a separate Wasm workspace (`wasm32-wasi` target). The main workspace remains `x86_64` (or `aarch64`). Extensions are cross-compiled.

- **Protocol version** declared in handshake response. The host rejects extensions whose protocol version it doesn't support. Protocol versions are additive (new categories, new host functions) never breaking.

- **Hot plug/unplug with graceful degradation.** Disconnected extensions' entities become untyped nodes. Validation rules stop firing. Surfaces are removed. Reconnection restores full semantics. This matches the existing soft-reference philosophy (I004 for unresolved cross-extension references).

- **Context-aware describe.** The host only calls `__describe` for categories relevant to the current context (CLI skips MCP schemas, MCP skips DOT colors, LSP requests full metadata). The `contribution_flags` from handshake tell the host which categories the extension supports.

### Phased Delivery

**Phase 1: ProtocolRuntime + handshake/describe (host-side)**
- New `ProtocolRuntime` trait in `specforge-wasm`
- Handshake validation (protocol version, sandbox policy, peer dependencies)
- Describe dispatch for all 11 categories
- Registry population from protocol responses (same output as manifest parsing)
- Dual-mode detection: manifest.json vs. protocol-based
- Unit tests with mock Wasm modules

**Phase 2: specforge-extension-sdk crate (proc macros)**
- `#[extension]`, `#[sandbox]`, `#[peer_dependency]` -> `__handshake` export
- `#[entity_kind]`, `#[field]`, `#[edge]`, `#[lsp]`, `#[dot]` -> entity descriptors
- `#[edge_type]`, `#[shared_field]`, `#[enhance]` -> edges/fields/enhancements
- `#[validation_rule]`, `#[validator]` -> validation descriptors + `validate__*` exports
- `#[cli_command]`, `#[mcp_tool]`, `#[mcp_resource]`, `#[arg]`, `#[sandbox_override]` -> surface descriptors + `cmd__*`/`mcp__*` exports
- `#[grammar]`, `#[body_parser]`, `#[collector]`, `#[auto_detect]` -> contribution descriptors + `parse__*`/`collect__*` exports
- `#[compiler_pass]`, `#[feature_flag]` -> pass/flag descriptors
- Macro expansion snapshot tests (trybuild + insta)

**Phase 3: Host API imports**
- `query(pattern)` -> graph query with existing query engine
- `emit_diagnostic(severity, code, message)` -> diagnostic collection
- `resolve_ref(id)` -> entity lookup
- `read_file(path)` -> sandbox-gated file access
- Integration tests with real Wasm modules

**Phase 4: Golden test extension + end-to-end**
- Build golden extension using all SDK macros
- Compile to wasm32-wasi
- Run full protocol lifecycle: load -> handshake -> describe -> register -> operate -> disconnect
- Assert registry population matches expected state
- Assert host API calls work correctly from inside extension

**Phase 5: Migrate 4 extensions**
- Convert `@specforge/governance` first (smallest: 3 kinds, 11 edges, 7 rules)
- Convert `@specforge/formal` (5 kinds, 12 edges, 1 rule, 2 enhancements, 4 passes)
- Convert `@specforge/software` (5 kinds, 14 edges, 12 rules, 2 enhancements)
- Convert `@specforge/product` (9 kinds, 20 edges, 24 rules)
- Delete manifest.json files after each successful migration
- Run full test suite after each migration

**Phase 6: @specforge/software-testing extraction**
- New extension crate in `extensions/software-testing/`
- 13 entity enhancements adding `gherkin` field
- `TestedBy` edge type
- W004 validation rule
- Cucumber/Gherkin test result collector
- Remove `manifest.json` support from host (all extensions now use protocol)

### Extension Registry (Light Touch)

The extension registry for publishing and downloading extensions is out of scope for implementation in this PRD. However, the design must support it:

- Extension specifier format already exists: `@scope/name@version` (registry), `./path` (local), `git+url#rev` (git)
- The lock file (`specforge.lock`) already stores SHA256 hashes per extension for integrity
- `specforge add` already parses specifiers and validates format
- The `.wasm` binary is the sole publishable artifact (no sidecar files after migration)
- Version resolution, download, and caching will be specified in a separate PRD

## Testing Decisions

### What Makes a Good Test

Tests should verify behavior through public interfaces. For this PRD, the public interfaces are:
- The Wasm export contract (handshake returns valid metadata, describe returns correct descriptors)
- The registry population output (kinds, fields, edges registered correctly)
- The Host API contract (query returns expected entities, emit_diagnostic reaches the collection)
- The SDK macro expansion (generated code matches expected patterns)

Tests should not depend on internal implementation details of the proc macros (AST node structure, intermediate representations) or the runtime (thread scheduling, memory layout).

### Modules to Test

**ProtocolRuntime** -- Unit tests with mock Wasm modules. Test handshake validation (protocol version mismatch, missing fields), describe dispatch (each of 11 categories), registry population from protocol responses, dual-mode detection, error handling (extension crash, malformed response), and hot disconnect/reconnect behavior. Prior art: existing `specforge-wasm/tests/lifecycle_ops.rs` (808-line lifecycle test suite) and `specforge-wasm/tests/wasm_lifecycle.rs`.

**specforge-extension-sdk macros** -- Two levels:
1. *Expansion snapshots* (trybuild + insta): Verify that each macro expands to the correct Rust code. Catches regressions in code generation. Prior art: `specforge-parser/tests/snapshot_tests.rs` (insta-based snapshots).
2. *Golden extension* (end-to-end): A test extension using all 24 macros, compiled to Wasm, loaded by the ProtocolRuntime, and verified against expected registry state. This is the ultimate conformance test.

**Host API** -- Integration tests with real Wasm modules that call host functions. Test query results, diagnostic emission, ref resolution, file reads, and sandbox enforcement (denied reads, denied network). Prior art: existing `specforge-wasm/tests/host_functions_integration.rs` (1126-line host function test suite).

**Dual-mode host** -- Integration tests that load a mix of manifest.json and protocol-based extensions simultaneously. Verify that registries merge correctly, cross-extension references resolve, and diagnostic codes don't collide.

**Extension migration** -- For each migrated extension, run the full existing test suite (e.g., `specforge-registry/tests/software_manifest.rs`) against the protocol-loaded version. The test suite becomes the migration acceptance gate: if all assertions pass with the protocol-loaded extension, the migration is correct.

**@specforge/software-testing** -- Test that the gherkin field appears on all 13 enhanced entity kinds. Test that W004 fires when gherkin is empty. Test that the TestedBy edge is registered. Test that the collector parses Cucumber JSON/XML results.

## Out of Scope

- **Extension registry implementation.** Publishing, downloading, versioning, and caching of extensions from a remote registry. Covered by a separate PRD. This PRD only ensures the design supports it.

- **Extension marketplace or discovery UI.** No web interface for browsing or installing extensions.

- **Wasm Component Model.** The protocol uses the existing WASI preview 1 ABI. Migration to the Component Model is a future consideration.

- **Extension-to-extension communication.** Extensions communicate only with the host. Peer-to-peer extension messaging is not supported.

- **Network-enabled extensions.** While the sandbox policy declares a `network_access` field, the actual `fetch`-style host function is deferred to a future Host API version (2.0.0).

- **Extension signing and trust.** Code signing, signature verification, and trust chains for published extensions.

- **Migration tooling.** No `specforge migrate-extension` command that auto-converts a manifest.json to SDK code. Migration is manual, guided by the SDK documentation.

- **Backward-compatible manifest.json emission.** The SDK does not generate manifest.json files from Rust code. The protocol replaces manifests entirely.

## Further Notes

### Transition Timeline

The dual-mode host ensures zero disruption during the transition. Extensions can migrate one at a time in any order. The existing 2,600+ test suite serves as the regression gate: if tests pass with both manifest-based and protocol-based extensions loaded, the migration is correct.

After all 4 extensions are migrated and `@specforge/software-testing` is built (Phase 6), the manifest.json loading path and the old `WasmRuntime` dispatch model can be removed. This is a cleanup step, not a deadline.

### Relationship to Existing Codebase

The existing `specforge-wasm` crate already provides:
- `WasmRuntime` trait with load/call/cache semantics
- Extension lifecycle management (Discovered -> Loading -> Initialized -> Validating -> Exporting -> Unloaded)
- Host function infrastructure (call-site access control, graph query scopes, file read sandboxing)
- Lock file integrity (SHA256 hashes)
- Surface registration and auto-promotion (CLI commands -> MCP tools)

The new `ProtocolRuntime` builds on this foundation. It reuses the existing sandbox enforcement, graph query engine, and diagnostic collection. The main addition is the multi-step negotiation layer (handshake + describe) and the bidirectional Host API imports.

### SDK Macro Complexity

The SDK's proc macros are the most complex new component. They must:
- Parse Rust struct/function attributes and generate correct Wasm exports
- Derive field types from Rust types (`String` -> `string`, `Vec<EntityRef>` -> `reference_list`)
- Wire edge declarations from `#[edge]` attributes on struct fields into the `__describe("edges")` response
- Generate JSON serialization for all protocol responses
- Handle error cases (duplicate field names, invalid attribute combinations) with clear compile-time error messages

The dual testing strategy (expansion snapshots + golden extension) mitigates this complexity. Expansion snapshots catch code generation regressions quickly. The golden extension catches protocol conformance issues end-to-end.

### Vision Alignment

This PRD directly serves three SpecForge principles:

- **Principle 2 (zero domain knowledge in core):** The protocol is the sole mechanism for domain vocabulary to enter the compiler. The host discovers everything through `__handshake` and `__describe`.

- **Principle 7 (extensions over built-ins):** Testing was the last built-in concern. Extracting it to `@specforge/software-testing` completes the architectural commitment.

- **Principle 8 (seconds to value):** Context-aware `__describe` loading means the CLI skips categories it doesn't need, reducing startup latency. Hot plug/unplug means the LSP and MCP server never need full restarts.

### Reference Documentation

- `docs/extension-protocol.md` -- Wire protocol specification
- `docs/extension-sdk.md` -- SDK developer guide with complete macro reference
- `docs/extension-inventory.md` -- Catalog of 5 extensions after entity audit and testability extraction
