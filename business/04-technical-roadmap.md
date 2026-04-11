# TECHNICAL ROADMAP

## 1. Compiler Pipeline Architecture

SpecForge compiles `.spec` files through a five-stage pipeline implemented as a Rust workspace of 11 crates. Each stage is isolated behind a clean interface, enabling independent testing, parallel development, and incremental re-compilation in watch mode. The compiler has **zero built-in entity types** -- all domain vocabulary comes from Wasm extensions.

```
.spec files (disk)
       |
       v
+------------------+     crate: tree-sitter-specforge
| Stage 1: PARSE   |     crate: specforge-parser
| tree-sitter      |     ----------------------------------------
| grammar -> CST   |     Tree-sitter grammar (grammar.js -> C -> Rust binding)
| CST -> typed AST |     Error-recovering incremental parser
|                  |     Per-file AST with span information
+--------+---------+     String interning via lasso (specforge-common)
         |
         v
+------------------+     crate: specforge-resolver
| Stage 2: RESOLVE |     ----------------------------------------
| import graph     |     `use` path resolution relative to spec root
| symbol binding   |     Topological file ordering (cycle detection -> E003)
| scope analysis   |     Symbol binding: references -> declarations
|                  |     Selective import narrowing
+--------+---------+     Levenshtein suggestions for unresolved refs (strsim)
         |
         v
+------------------+     crate: specforge-graph
| Stage 3: BUILD   |     ----------------------------------------
| entity nodes     |     petgraph directed graph (DiGraph<Entity, Edge>)
| typed edges      |     Extension-defined node types and edge types
| mutable graph    |     Field-name-based edge routing (EdgeType::from_field_name)
|                  |     Soft reference bookkeeping for cross-extension refs
+--------+---------+     Incremental graph mutation for watch mode / LSP
         |
         v
+------------------+     crate: specforge-validator
| Stage 4: VALIDATE|     ----------------------------------------
| diagnostics      |     Core structural codes + extension-declared codes
| structural rules |     Extension-scoped rules (fire only when extension installed)
| orphan detection |     ariadne-powered rich diagnostics with spans
|                  |     Declarative validation rule patterns from ManifestV2
+--------+---------+
         |
         v
+------------------+     crate: specforge-emitter
| Stage 5: EXPORT  |     ----------------------------------------
| Graph Protocol   |     JSON graph export (the primary product)
| agent context    |     Agent-context token-optimized export
| renderers        |     Extension-contributed renderers (non-code outputs only)
+------------------+     Multi-resolution queries (--scope, --hop, --format)
```

### Two-Phase Compilation

The pipeline separates **structural parsing** (Stages 1-3) from **semantic validation** (Stages 4-5). Stage 1-3 run with zero domain knowledge -- the tree-sitter grammar parses any `keyword name { fields }` block generically. Stage 4-5 apply extension-defined validation rules after extensions register their entity kinds, edge types, and constraints.

```
Phase 1 (Structural)         Phase 2 (Semantic)
+-------------------------+  +---------------------------+
| Parse any keyword block |  | Extension manifests load  |
| Resolve use imports     |->| KindRegistry populates    |
| Build raw graph         |  | Validation rules execute  |
| (domain-agnostic)       |  | Graph Protocol exports    |
+-------------------------+  +---------------------------+
```

---

## 2. Crate/Module Structure

### Crate Dependency DAG

```
specforge-common (lasso, serde)
       |
       +---> tree-sitter-specforge (tree-sitter grammar)
       |           |
       |           v
       +---> specforge-parser (tree-sitter, tree-sitter-language)
       |           |
       |           v
       +---> specforge-resolver (strsim)
       |           |
       |           v
       +---> specforge-graph (petgraph)
       |           |
       |           v
       +---> specforge-validator (ariadne)
       |           |
       |           v
       +---> specforge-emitter
       |           |
       |    +------+------+
       |    v             v
       +- specforge-cli  specforge-lsp
       |  (clap, walkdir, (tower-lsp, tokio,
       |   notify, ctrlc,  walkdir, strsim)
       |   inquire)
       |
       +---> specforge-wasm (extism)
       |
       +---> specforge-coverage
```

### Crate Descriptions

| Crate | Purpose | Key Dependencies |
|-------|---------|-----------------|
| `tree-sitter-specforge` | Tree-sitter grammar for `.spec` files. Generates C parser + Rust binding. Provides `highlights.scm`, `folds.scm`, `indents.scm` for editor integration. | tree-sitter |
| `specforge-common` | Shared types: `EntityId`, `EntityKind`, `Span`, `Diagnostic`, `KindRegistry`, `FieldRegistry`, `InternedId`, `FormatVersion`, config structs. String interning via `lasso::ThreadedRodeo`. | lasso, serde |
| `specforge-parser` | CST-to-AST lowering with span preservation. Error-recovering parse of any `keyword name { fields }` block. Snapshot-tested with `insta`. | tree-sitter, specforge-common |
| `specforge-resolver` | `use` import resolution, topological file ordering, cycle detection, symbol binding. Levenshtein suggestions for unresolved references. | strsim, specforge-parser |
| `specforge-graph` | `petgraph::DiGraph` population from resolved AST. Extension-registered edge types. Incremental graph mutation. Subgraph extraction. File index for invalidation. | petgraph, specforge-resolver |
| `specforge-validator` | Core structural validation + extension-contributed validation rules. Declarative validation patterns. ariadne-rendered diagnostics. | ariadne, specforge-graph |
| `specforge-emitter` | Graph Protocol JSON export. Agent-context export. DOT visualization. Markdown rendering. Traceability chain computation. Statistics. | serde_json, specforge-graph |
| `specforge-wasm` | Wasm/Extism extension runtime. Extension manifest parsing. AOT compilation + caching. Warm engine instances. Sandbox enforcement. Host function dispatch. | extism, specforge-graph |
| `specforge-coverage` | Test report consumption (`specforge-report.json`). Four-level coverage (declared/linked/executed/passing). JUnit XML and libtest JSON parsing. Report merging. | specforge-graph |
| `specforge-lsp` | LSP server: go-to-definition, references, hover, completion, rename, code actions, semantic tokens, document symbols. Shares incremental pipeline with watch mode. | tower-lsp, tokio, specforge-graph |
| `specforge-cli` | CLI binary: `check`, `init`, `watch`, `export`, `trace`, `coverage`, `add`, `remove`, `doctor`, `collect`, `format`, `migrate`. | clap, notify, inquire, specforge-* |

---

## 3. Technology Stack

| Concern | Technology | Why |
|---------|-----------|-----|
| Parser | tree-sitter 0.25 | Incremental parsing, error recovery, editor integration out of the box. Sub-5ms re-parse on file change. |
| Graph | petgraph | Rust standard for directed graphs. Stable node indices for mutable graphs. Battle-tested topological sort and cycle detection. |
| String interning | lasso (`ThreadedRodeo`) | Fastest thread-safe interner in Rust. O(1) identifier comparison via u32 keys. Required for LSP multi-threaded operation. |
| Error diagnostics | ariadne | Rust-compiler-grade output with multi-file spans, inline labels, and color. |
| LSP framework | tower-lsp | Integrates with tokio/tower ecosystem. Handles protocol lifecycle and routing. |
| CLI framework | clap (derive) | Declarative subcommand definitions. Shell completion generation. Industry standard for Rust CLIs. |
| Serialization | serde + serde_json | Ecosystem standard. Derive macros for Graph Protocol export and extension communication. |
| Extension runtime | Extism (Wasm) | Language-agnostic extension SDK. Sandboxed execution. AOT compilation via Cranelift. Host function interface. |
| File watching | notify | Cross-platform abstraction (inotify/FSEvents/ReadDirectoryChanges). Debouncing support. |
| Snapshot testing | insta | `cargo insta review` TUI. JSON snapshot support for AST and graph testing. |
| Fuzzy matching | strsim | Levenshtein and Jaro-Winkler for "did you mean?" suggestions. Minimal dependency. |
| Async runtime | tokio | Required by tower-lsp. De facto Rust async standard. Only used in LSP binary; CLI is synchronous. |
| Rust edition | 2024 | Latest stable edition. |

---

## 4. Extension System Architecture

The extension system is the architectural core of SpecForge's multi-domain strategy. Following the Terraform provider model, the compiler has zero domain knowledge -- all entity types, edge types, validation rules, and domain-specific tooling come from Wasm extensions.

### Extension Manifest V2

Every extension declares its contributions via a structured manifest:

```
Extension Manifest V2 Schema
+---------------------------+
| name: "@specforge/software"
| version: "1.0.0"
| specforge_version: ">=0.8"
|
| entity_kinds:
|   - name: behavior
|     fields: [title, contract, ...]
|     testable: true
|     lsp_icon: method
|   - name: invariant
|     fields: [guarantee, ...]
|     testable: false
|     lsp_icon: constant
|
| edge_types:
|   - name: implements
|     from: behavior
|     to: feature
|     field: features
|
| validation_rules:
|   - code: W001
|     pattern: no_incoming_edges
|     entity_kind: behavior
|     severity: warning
|     message: "orphan behavior"
|   - code: W004
|     pattern: missing_field_on_testable
|     field: verify
|     severity: warning
|
| contributions:
|   entities: [behavior, invariant, ...]
|   validators: [orphan_check, ...]
|   renderers: [markdown_summary]
|   providers: [gh_issue_validator]
|   collectors: [rust_test_collector]
+---------------------------+
```

### Contribution Types

| Type | Purpose | Example |
|------|---------|---------|
| **entities** | Register entity kinds with fields, testability, LSP metadata | `behavior`, `regulation`, `atom` |
| **validators** | Declarative or Wasm-implemented validation rules | Orphan detection, missing field checks |
| **renderers** | Non-code output generators (reports, dashboards, matrices) | Markdown summary, traceability matrix |
| **providers** | Reference validation for external systems | GitHub issue validator, Jira epic validator |
| **collectors** | Test result collection from external test runners | JUnit XML parser, libtest JSON parser |

### Wasm Runtime Architecture

```
+----------------------------------------------------------+
|                    SpecForge Core                          |
|                                                          |
|  +------------------+    +---------------------------+   |
|  | Extension Loader |    | Host Functions            |   |
|  |                  |    |                           |   |
|  | Discover         |    | specforge.query_graph     |   |
|  | Validate manifest|    | specforge.emit_diagnostic |   |
|  | AOT compile      |    | specforge.register_entity |   |
|  | Cache artifact   |    | specforge.register_edge   |   |
|  | Warm engine      |    | specforge.emit_file       |   |
|  +--------+---------+    | specforge.http_get        |   |
|           |              +-------------+-------------+   |
|           v                            |                 |
|  +------------------+                  |                 |
|  | Sandbox Policy   |    +-------------v-------------+   |
|  |                  |    |    Wasm/Extism Runtime     |   |
|  | No filesystem    |    |                           |   |
|  | No network       |    | Extension A (.wasm)       |   |
|  | No env vars      |    | Extension B (.wasm)       |   |
|  | CPU time limits  |    | Extension C (.wasm)       |   |
|  | Memory limits    |    +---------------------------+   |
|  +------------------+                                    |
+----------------------------------------------------------+
```

### AOT Compilation and Warm Engines

| Mode | Strategy | Target Latency |
|------|----------|---------------|
| CLI (cold start) | AOT compilation via Cranelift; cached in `.specforge/cache/`. Self-healing on corruption. Invalidated on runtime upgrade. | <50ms per extension load |
| LSP (long-running) | Warm engine instances kept in memory. Re-used across compilations. | <5ms per extension call |
| MCP (long-running) | Same warm engine pool as LSP. | <5ms per extension call |

### Sandbox Enforcement

Extensions execute in a Wasm sandbox with no default capabilities:
- No filesystem access (unless `specforge.emit_file` host function is granted)
- No network access (unless `specforge.http_get` host function is granted)
- No environment variable access
- CPU time limits per invocation
- Memory limits per extension instance
- Per-call-site permission enforcement (least privilege)

### Current Extensions

| Extension | Entity Kinds | Domain |
|-----------|-------------|--------|
| `@specforge/software` | behavior, invariant, feature, event, type, port | Software engineering |
| `@specforge/product` | journey, deliverable, milestone, module, term | Product management |
| `@specforge/governance` | decision, constraint, failure_mode | Technical governance |
| `@specforge/compliance` | regulation, control, evidence, audit | Regulatory compliance |

Combined, the three original extensions (`software` + `product` + `governance`) reproduce all 14 domain entities plus 2 structural keywords (`spec`, `ref`) = 16 total entity kinds, maintaining backward compatibility.

---

## 5. Graph Protocol Specification

The Graph Protocol is the primary product. It is a JSON schema that any AI agent framework can consume. The compiler is the reference implementation; the schema is the standard.

```
Graph Protocol JSON Schema (simplified)
{
  "version": "1.0",
  "project": { "name": "...", "spec_root": "..." },
  "nodes": [
    {
      "id": "auth_login",
      "kind": "behavior",
      "extension": "@specforge/software",
      "fields": {
        "title": "User Login",
        "contract": "Validates credentials and returns JWT",
        "features": ["authentication"],
        "invariants": ["session_integrity"]
      },
      "testability": {
        "testable": true,
        "verify_count": 3,
        "coverage": "passing"
      },
      "source": { "file": "spec/behaviors/auth.spec", "line": 12 }
    }
  ],
  "edges": [
    {
      "from": "auth_login",
      "to": "authentication",
      "type": "implements",
      "field": "features"
    }
  ],
  "diagnostics": [...],
  "extensions": [...]
}
```

### Multi-Resolution Queries

| Format | Command | Use Case |
|--------|---------|----------|
| `graph` | `specforge export --format=graph` | Complete entity graph JSON for tools and dashboards |
| `context` | `specforge export --format=context` | Token-optimized output for AI agent context windows |
| `brief` | `specforge export --format=brief` | Minimal entity IDs + contracts for quick orientation |
| Scoped | `--scope=payments --hop=2` | Subgraph extraction for focused agent tasks |

---

## 6. MCP Server Architecture

The MCP (Model Context Protocol) server enables AI agents to consume the spec graph without CLI invocation. It runs as a service alongside the LSP server.

```
+---------------------+       +-------------------------+
| AI Agent            |       | SpecForge MCP Server    |
| (Claude, GPT, etc.) |       |                         |
|                     |       | Resources:              |
| reads resource -----+------>|   specforge://graph     |
|                     |       |   specforge://schema    |
| calls tool ---------+------>|                         |
|                     |       | Tools:                  |
| subscribes ---------+------>|   specforge.query       |
|                     |       |   specforge.validate    |
+---------------------+       |   specforge.export      |
                              |                         |
        <-- delta notifs -----| Notifications:          |
                              |   graph_changed (delta) |
                              +-------------------------+
```

Key design decisions:
- Resources expose `specforge://graph` and `specforge://schema` for direct reads
- Tools provide `specforge.query`, `specforge.validate`, and `specforge.export` for interactive use
- Delta notifications push `GraphDelta` (added/removed/modified nodes and edges) to subscribed agents after incremental rebuilds
- Shares the warm Wasm engine pool and incremental pipeline with the LSP server

---

## 7. Federation Architecture

Federation enables cross-project references for large organizations with multiple repositories.

```
Project A (auth-service)          Project B (api-gateway)
+-------------------------+       +-------------------------+
| behavior auth_login     |       | behavior route_request  |
|   ...                   |       |   invariants [           |
|                         |       |     auth::session_integrity
+-------------------------+       |   ]                     |
                                  +-------------------------+
         |                                   |
         v                                   v
+-----------------------------------------------------------+
|                  Federated Graph                           |
|                                                           |
| project::entity_id syntax resolves cross-project refs     |
| Remote project graphs loaded from path or registry        |
| Cross-project edge compatibility validated                |
| Missing remotes degrade gracefully (I006 info)            |
+-----------------------------------------------------------+
```

- `specforge export --federated` merges local and remote graphs
- Cross-project edge types validated for compatibility
- Graceful degradation: missing remote projects produce I006 info-level diagnostics, not errors

---

## 8. Performance Targets

### Compiler Performance

| Metric | Target | Rationale |
|--------|--------|-----------|
| Parse time (1000 entities) | <30ms | Tree-sitter is ~10x faster than hand-written parsers |
| Resolve time (1000 entities) | <15ms | Linear in file count; HashMap lookups on interned keys |
| Graph build time (1000 entities) | <10ms | petgraph node/edge insertion is O(1) amortized |
| Validation time (1000 entities) | <20ms | Graph traversal O(V+E); ariadne rendering deferred |
| Emit time (JSON, 1000 entities) | <15ms | Linear in graph size |
| **Total compile time (1000 entities)** | **<100ms** | Sub-human-perception for batch mode |
| Incremental re-compile (1 file change) | <10ms | Tree-sitter edit + graph patch |

### Memory Usage

| Metric | Target |
|--------|--------|
| Base memory (empty project) | <5MB |
| Per-entity overhead | ~30-50KB |
| 500-file project | <50MB |
| 5000-entity project | <250MB (linear scaling) |
| String intern table | <2MB for 10K unique identifiers |

### Extension Performance

| Metric | Target |
|--------|--------|
| AOT extension cold load | <50ms per extension |
| Warm engine call | <5ms per validation pass |
| AOT cache rebuild (corruption recovery) | <2s for 5 extensions |

### LSP Performance

| Operation | Target Latency |
|-----------|---------------|
| `textDocument/completion` | <50ms |
| `textDocument/definition` | <10ms |
| `textDocument/references` | <30ms |
| `textDocument/hover` | <20ms |
| `textDocument/rename` | <200ms |
| `textDocument/semanticTokens` | <50ms |
| Diagnostics after edit | <150ms |

### Binary Size

| Binary | Target |
|--------|--------|
| `specforge` (CLI) | <15MB |
| `specforge-lsp` | <18MB |
| npm wrapper extension | <500KB |

---

## 9. Development Phases

### Phase 1: Core Compiler -- COMPLETED

Delivered the foundation: tree-sitter grammar with generic `keyword name { fields }` block rule, CST-to-AST lowering, import resolution with cycle detection, petgraph-based graph construction, core structural validation (E-level errors and W-level warnings), ariadne-rendered diagnostics, CLI scaffolding (`check`, `init`, `add`, `remove`), and project initialization with `specforge.json` configuration.

**Libraries:** tree-sitter-specforge, specforge-parser, specforge-resolver, specforge-graph, specforge-validator, specforge-cli

### Phase 2: CLI and Watch Mode -- COMPLETED

Delivered file-system watching with incremental rebuild, JSON/DOT/Markdown export, traceability chain computation, project statistics, CI integration (`--strict` mode, deterministic output), and diagnostic formatting.

**Libraries:** specforge-emitter, specforge-watch (within specforge-cli)

### Phase 3: LSP Server -- COMPLETED

Delivered go-to-definition, find-all-references, hover, autocomplete (entity IDs, field names, keywords), rename with cross-file propagation, live diagnostics, code actions (add missing import, create entity stub), outline view, workspace symbol search, semantic tokens, and incremental document sync. Shares the incremental pipeline with watch mode.

**Libraries:** specforge-lsp

### Phase 4: Graph Protocol and Agent Export -- COMPLETED

Delivered the Graph Protocol JSON schema, agent-context token-optimized export (`--format=context`), brief export (`--format=brief`), multi-resolution queries (`--scope`, `--hop`), and Wasm extension renderer protocol. The Graph Protocol is the primary product output.

**Libraries:** specforge-emitter

### Phase 5: Extensions and Coverage -- COMPLETED

Delivered Wasm/Extism extension runtime, extension manifest V2, AOT compilation + caching, warm engine instances, sandbox enforcement, host function dispatch (query_graph, emit_diagnostic, register_entity, register_edge, emit_file, http_get), peer dependency validation, extension install/upgrade/remove, contribution-based extension dispatch, per-call-site permissions, lock file with SHA256 integrity, test coverage reporting (four-level: declared/linked/executed/passing), `specforge-report.json` consumption, and coverage gating.

**Libraries:** specforge-wasm, specforge-coverage

### Phase 5b: Wasm Extension Authoring -- COMPLETED

Delivered `specforge extension init` (scaffold), `specforge extension build` (compile to .wasm), `specforge extension test` (run against fixtures), and `specforge extension publish` (upload to npm/OCI/GitHub).

**Libraries:** specforge-wasm

### Phase 6: Rust Test Collection -- COMPLETED

Delivered `specforge collect rust` for JUnit XML and libtest JSON parsing. Three-level entity-to-test mapping: `tests` field > `#[specforge::test("id")]` proc macro > `{id}__{slug}` convention. Workspace report merging. Drop guard for pass/fail recording.

**Libraries:** specforge-collect-rust, specforge-test, specforge-test-macros

### Phase 7: Code Formatting -- COMPLETED

Delivered `specforge format` (format all .spec files), `--check` mode for CI, `--diff` for preview, stdin formatting, configurable format rules, idempotency guarantee, and comment preservation. The `specforge-formatter` crate implements a rule-based formatting engine with diff output, file discovery, and per-rule configuration.

**Libraries:** specforge-formatter (7 modules, 80 tests)

### Phase 8: Zero-Entity Core -- COMPLETED

The most architecturally significant phase. Removed all remaining hardcoded entity types from the compiler core. Delivered:

- **Empty boot:** `KindRegistry` and `FieldRegistry` boot empty, populated exclusively from extension manifests
- **Declarative validation:** All validation rules are declarative patterns interpreted by core, not hardcoded passes
- **Extension-driven LSP:** Keyword completion, semantic tokens, hover, and icons all driven by extension declarations
- **Two-phase compilation:** Structural parsing (domain-agnostic) separated from semantic validation (extension-aware)
- **Graceful degradation:** Zero extensions installed produces I002 info-level diagnostics, not errors
- **Unknown keyword detection:** E024 diagnostics suggest which extension provides unrecognized keywords
- **Manifest V2 consistency validation:** Ensure extensions do not declare contradictory rules
- **Extension registry:** Registry client with auth, search, publish, version resolution, and integrity verification
- **Surface contributions:** CLI command and MCP tool/resource contributions from extensions
- **Define blocks:** User-defined meta-schema types beyond extension-provided entity kinds

**Exit criteria met:** A new domain extension (`@specforge/compliance`, `@specforge/atomic-design`) works end-to-end without any compiler changes.

**Libraries:** specforge-registry (21 modules, 316 tests)

### Phase 9: MCP Server -- COMPLETED

Delivered full JSON-RPC 2.0 MCP server with lifecycle management (initialize/shutdown), phase guarding, and reinitialization protection. Implemented:

- **Resources:** `specforge://graph`, `specforge://diagnostics`, `specforge://schema` as read-only MCP resources
- **Tools (17):** `specforge.query`, `specforge.validate`, `specforge.export`, `specforge.trace`, `specforge.search`, `specforge.stats`, `specforge.coverage`, `specforge.schema`, `specforge.inspect`, `specforge.format`, `specforge.rename`, `specforge.init`, `specforge.add_extension`, `specforge.remove_extension`, `specforge.migrate`, `specforge.collect`, `specforge.render`, `specforge.doctor`
- **Prompts (4):** `specforge://prompts/context`, `specforge://prompts/review`, `specforge://prompts/trace`, `specforge://prompts/explore`
- **Subscriptions:** Channel-based subscriptions with delta notifications after incremental rebuilds
- **Event system:** Structured event logging for protocol errors, lifecycle transitions, and graph changes
- **Parameter validation:** Severity filtering, format validation, field/value search, coverage annotation, trace gap detection

**Libraries:** specforge-mcp (287 tests across 13 test modules)

### Phase 9b: Federation -- PLANNED

Delivers `project::entity_id` syntax for cross-project references. Remote project graphs loaded from path or registry. Cross-project edge compatibility validation. `specforge export --federated` merges local and remote graphs. Missing remotes degrade gracefully (I006 info).

### Phase 10: Entity Embeddings -- FUTURE

Entity contracts and relationships embedded into vector space. `specforge search --semantic` queries entities by natural language. Integration with agent memory systems for persistent context.

### Phase Summary

| Phase | Name | Status | Tests |
|-------|------|--------|-------|
| 1 | Core Compiler | Completed | 152 |
| 2 | CLI and Watch Mode | Completed | 223 |
| 3 | LSP Server | Completed | 116 |
| 4 | Graph Protocol and Agent Export | Completed | 111 |
| 5 | Extensions and Coverage | Completed | 415 |
| 5b | Wasm Extension Authoring | Completed | — |
| 6 | Rust Test Collection | Completed | — |
| 7 | Code Formatting | Completed | 80 |
| 8 | Zero-Entity Core | Completed | 316 |
| 9 | MCP Server | Completed | 287 |
| 9b | Federation | Planned | — |
| 10 | Entity Embeddings | Future | — |
| | **Total** | **9/11 completed** | **1,700** |

---

## 10. Security Model

### Wasm Sandbox

Extensions execute in a Wasm sandbox with **no default capabilities**. This is enforced by the Extism runtime:

| Capability | Default | Grantable Via |
|-----------|---------|--------------|
| Filesystem read/write | DENIED | `specforge.emit_file` host function (scoped to output directory) |
| Network access | DENIED | `specforge.http_get` host function (allowlist-based) |
| Environment variables | DENIED | Not grantable |
| System calls | DENIED | Not grantable |
| CPU time | LIMITED | Configurable per-extension timeout |
| Memory | LIMITED | Configurable per-extension memory cap |

### Extension Integrity

- Extensions are pinned in `specforge.lock` with SHA256 integrity hashes
- AOT cache validates hashes before loading cached artifacts
- Manifest V2 is validated before any Wasm code executes
- Peer dependency validation prevents incompatible extension combinations

### Supply Chain

- Extensions distributed via npm, OCI registries, or GitHub Releases
- Lock file ensures reproducible builds across environments
- `specforge doctor` reports extension health and cache integrity

---

## 11. Technical Risks and Mitigations

| # | Risk | Prob. | Impact | Mitigation |
|---|------|-------|--------|------------|
| R1 | Tree-sitter grammar complexity exceeds maintainability | Medium | High | Single generic `keyword name { fields }` rule for all entity blocks. Comprehensive corpus tests. Avoid external scanner. |
| R2 | LSP responsiveness degrades on large projects (5000+ entities) | Medium | High | Incremental validation (only affected entities). Debounce edits (100ms). Cache unchanged file ASTs. Tiered validation fallback: syntax-only on keystroke, full on save. |
| R3 | Cross-platform binary distribution fails on edge cases | High | Medium | Test all 5 targets in CI on every release. Static linking on Linux (musl). Code-sign macOS/Windows. Docker image as fallback. |
| R4 | Wasm extension ABI stability becomes a burden | Medium | High | Version the manifest schema (semver). Backward compatibility for 2+ major versions. Manifests are declarative (not compiled code), reducing ABI surface. |
| R5 | Token-efficient agent context format is not efficient enough | Medium | Critical | Hierarchical compressed summaries. `--budget` flag for token limits. Graph centrality ranking. `--focus` for scoped output. MCP server eliminates pre-loading. |
| R6 | Zero-entity core migration breaks existing projects | Medium | High | Three original extensions reproduce all 14 domain entities. Existing projects migrate by adding extensions to `specforge.json`. `specforge migrate` automates the transition. |
| R7 | Competitor ships proprietary context format first | Medium | Critical | Open Graph Protocol schema prevents lock-in. Extension ecosystem creates switching costs. Community moat via open source. Partner with agent frameworks rather than compete. |
| R8 | Wasm sandbox too restrictive for renderer extensions | Low | Medium | Host functions cover 90% of renderer needs. Configurable sandbox policies per extension. Renderers produce non-code outputs only. |
| R9 | MCP protocol evolves incompatibly | Low | Medium | Track MCP specification closely. Version MCP server protocol. Maintain backward compatibility shim. |

### Risk Heatmap

```
                Low Impact    Medium Impact    High Impact    Critical Impact
High Prob   |              |      R3         |              |
Medium Prob |              |                 |  R1, R2, R4  |  R5, R7
            |              |                 |  R6          |
Low Prob    |              |      R8, R9     |              |
```

---

## 12. Build vs. Buy Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Parser technology | Tree-sitter (build grammar) | Incremental parsing, error recovery, and editor integration out of the box. LSP token highlighting comes free. |
| Implementation language | Rust | Single binary distribution. Memory safety without GC. Cargo workspace maps to pipeline stages. Ecosystem has tree-sitter, petgraph, tower-lsp, ariadne, extism. |
| Graph library | petgraph | Stable node indices for mutable graphs. Battle-tested algorithms. No IPC overhead. |
| Error diagnostics | ariadne | Rust-compiler-grade output. Multi-file spans. Active maintenance. |
| LSP framework | tower-lsp | tokio/tower integration. Protocol details handled. |
| String interning | lasso | Fastest thread-safe interner. Multi-threaded for LSP. |
| Extension runtime | Extism (Wasm) | Language-agnostic SDK. Sandboxed execution. AOT compilation. Host function interface. |
| CLI framework | clap (derive) | Industry standard. Shell completion. Declarative subcommands. |
| Serialization | serde + serde_json | Ecosystem standard. Derive macros eliminate boilerplate. |
| Snapshot testing | insta | Review TUI. JSON snapshot support. Inline snapshots. |
| File watching | notify | Cross-platform abstraction. Debouncing. |
| Fuzzy matching | strsim | Levenshtein for "did you mean?" suggestions. Minimal dependency. |
| Async runtime | tokio | Required by tower-lsp. Only used in LSP/MCP binaries. |
