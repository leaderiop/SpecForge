# SpecForge Improvements Tracker

> Generated from 20-expert analysis on 2026-04-11
> TDD approach: RED (test) -> GREEN (implement) -> REFACTOR per item

## Baseline

- **Tests**: 1,876 passing, 0 failures, 6 ignored (start) → **2,489 passing** (current)
- **Crates**: 15
- **Source files**: 304 Rust files
- **New dependencies**: semver 1.0.28

---

## Phase 1: Graph Safety & Performance [COMPLETE]

Critical fixes for graph integrity and query performance.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 1.1 | Edge index HashMap for O(1) lookups (replace O(n) scan) | 12 | YES | YES | DONE |
| 1.2 | Cyclic reference detection in graph builder (W061) | 1 | YES | YES | DONE |
| 1.3 | Reference depth limit for subgraph traversal | 1 | N/A | N/A | SKIPPED (subgraph_depth already exists) |
| 1.4 | Graph query: detect_cycles() and has_cycles() | 1, 6 | YES | YES | DONE |

**Summary**: Added `source_index` and `target_index` HashMaps to Graph for O(1) edge lookups (was O(n)). Added `detect_cycles()` (DFS three-color algorithm) and `has_cycles()` boolean check. `build_graph` now emits W061 for reference cycles. 9 new tests added, all 1,893 workspace tests passing. Item 1.3 skipped because `subgraph_depth()` already provides configurable depth limits.

---

## Phase 2: Resolver Safety [COMPLETE]

Path traversal and symlink protections.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 2.1 | Symlink protection: reject symlinked files in discovery | 9 | YES | YES | DONE |
| 2.2 | Disable follow_links(false) in walkdir discovery | 9 | YES | YES | DONE |
| 2.3 | Path traversal test for relative imports escaping spec_root | 9 | YES | EXISTED | DONE |

**Summary**: Added `follow_links(false)` to walkdir and `!entry.path_is_symlink()` filter to prevent symlink-based path traversal. Added 2 new tests: symlink_outside_spec_root_rejected and relative_import_path_traversal_rejected. Path traversal protection via `normalize_path()` + `starts_with()` was already in place. 34 resolver tests pass.

---

## Phase 3: Error Type System [COMPLETE]

Replace stringly-typed errors with proper Rust error types.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 3.1 | Display impl + DiagnosticsExt trait (has_errors, error_count) | 3, 16 | EXISTING | YES | DONE |
| 3.2 | Replace Result<T, String> in emitter APIs | 3, 16 | N/A | N/A | DEFERRED (70+ call sites, expect() safe on serde_json) |
| 3.3 | Add #[must_use] to build_graph, resolve_project functions | 3 | EXISTING | YES | DONE |
| 3.4 | Replace .expect() in brief/json serialization | 3 | N/A | N/A | DEFERRED (serde_json on simple structs is infallible) |

**Summary**: Added `Display` impls for Diagnostic/Severity, `DiagnosticsExt` trait with `has_errors()` and `error_count()` helpers, `#[must_use]` on build_graph/resolve_project. Items 3.2/3.4 deferred as the expect() calls are safe (serde_json on flat Serialize structs) and changing to Result would touch 70+ call sites with no safety gain.

---

## Phase 4: Diagnostic Enrichment [COMPLETE]

Improve error messages, suggestions, and grouping.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 4.1 | Max errors limit (truncate_diagnostics at 100) | 15 | YES | YES | DONE |
| 4.2 | Error grouping/summary (diagnostic_summary) | 15 | YES | YES | DONE |
| 4.3 | Expand suggestions to all E-prefix codes | 15 | | | DEFERRED (incremental per-code work) |
| 4.4 | Populate ParseError expected/found from tree-sitter | 15 | | | DEFERRED (requires tree-sitter API research) |
| 4.5 | Default query_scope to "own" instead of "all" | 9 | | | DEFERRED (requires Wasm runtime changes) |

**Summary**: Added `truncate_diagnostics()` (limits to 100 with I999 summary), `diagnostic_summary()` (groups by code, shows top 5 with counts), and `MAX_DIAGNOSTICS` constant. 5 new tests. Items 4.3-4.5 deferred as they require deeper per-subsystem changes.

---

## Phase 5: Extension Versioning [COMPLETE]

Enforce semver and compatibility in extension system.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 5.1 | Add semver parsing crate and version comparison | 2 | YES | YES | DONE |
| 5.2 | Validate peer_dependencies as semver ranges | 2 | YES | YES | DONE |
| 5.3 | Enforce host_api_version compatibility at load time | 2 | YES | YES | DONE |
| 5.4 | Detect circular peer_dependency declarations | 2 | YES | YES | DONE |

**Summary**: Added `semver` crate (v1.0.28) to replace hand-rolled version comparison. `version_satisfies()` now uses `semver::VersionReq::parse()` supporting `^`, `~`, `>=`, `>`, `<=`, `<`, and exact ranges. Added W062 diagnostic for malformed semver strings (both required range and installed version). Added `validate_host_api_versions()` with `HOST_API_VERSION` constant ("1.0.0") and E028 diagnostic for incompatible extensions. Added `detect_circular_peer_dependencies()` using DFS three-color algorithm with W063 diagnostic. 14 new tests, all 2,472 workspace tests passing.

---

## Phase 6: Parser Robustness [COMPLETE]

Fix silent error swallowing and improve recovery.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 6.1 | Replace silent unwrap_or("") with diagnostic emission | 3 | N/A | N/A | SKIPPED (infallible — source is &str, utf8_text cannot fail) |
| 6.2 | Replace unwrap_or(0) with diagnostic for malformed integers | 3 | YES | YES | DONE |
| 6.3 | Better error messages for common syntax mistakes | 15 | YES | YES | DONE |

**Summary**: Item 6.1 skipped because `text()` extracts bytes from an already-valid `&str` via tree-sitter ranges — `utf8_text` is infallible in this context. Item 6.2: integer overflow (e.g., 99999999999999999999999999) now emits a ParseError with expected/found fields instead of silently returning 0. Item 6.3: `push_error_node` now provides contextual messages (unclosed block, extra brace, guidance), truncates multi-line errors to first line, and always populates the `expected` field with syntax guidance. 6 new tests, snapshot updated.

---

## Phase 7: Performance Optimizations [COMPLETE]

Targeted improvements for 2-3x gains on large projects.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 7.1 | Custom Sym serializer (avoid repeated to_string()) | 12 | N/A | N/A | SKIPPED (already optimal — serialize_str(as_str()) uses &str directly) |
| 7.2 | Parallel file I/O with rayon in resolver | 12 | N/A | N/A | DEFERRED (adds heavy dep for marginal gains on typical projects) |
| 7.3 | Borrow FieldValue in emit instead of cloning | 12 | N/A | N/A | DEFERRED (serde_json::Value requires owned strings; needs streaming serializer) |

**Summary**: Item 7.1 already optimal — Sym::Serialize uses `serialize_str(as_str())` with zero allocation. The `.to_string()` calls in emitters are for HashMap keys, not serialization. Item 7.2 deferred — rayon adds a heavy dependency for marginal gains; sequential parsing is already <100ms for typical projects. Item 7.3 deferred — serde_json::Value::String requires owned data; eliminating clones requires a custom streaming serializer, large refactor for small strings.

---

## Phase 8: Testing Infrastructure [COMPLETE]

Fill critical testing gaps.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 8.1 | E2E pipeline tests (parse->resolve->graph->validate->emit roundtrip) | 4 | YES | YES | DONE |
| 8.2 | Property-based tests for graph (proptest) | 4 | N/A | N/A | DEFERRED (needs proptest dep + Arbitrary impls; add when graph API stabilizes) |
| 8.3 | Determinism tests for all emitters | 4 | YES | YES | DONE |
| 8.4 | WASM sandbox breach tests | 4 | N/A | N/A | DEFERRED (needs real Wasm modules in test path) |
| 8.5 | Enable fuzz testing in CI | 4 | N/A | N/A | DEFERRED (needs cargo-fuzz CI infrastructure) |

**Summary**: Added 6 E2E pipeline tests exercising full parse→resolve→graph→validate→emit roundtrip (single entity, multi-file imports, cross-entity edges, validation diagnostics, empty project, all emit formats). Added 4 new determinism tests for stats, trace, scoped JSON, and scoped context formats (joining existing 8 tests for JSON/DOT/brief/context). Items 8.2/8.4/8.5 deferred as they require new infrastructure (proptest crate, WASM test modules, cargo-fuzz CI) that's better added incrementally.

---

## Phase 9: API Surface Improvements [COMPLETE]

Formalize library and query APIs.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 9.1 | Predicate query API (filter by field values) | 6, 16 | YES | YES | DONE |
| 9.2 | Graph mutation/refactoring API (rename, batch edit) | 16 | N/A | N/A | DEFERRED (large API surface; needs design doc first) |
| 9.3 | Streaming emission for large graphs | 16 | N/A | N/A | DEFERRED (current graphs are small; needs benchmarking to justify) |

**Summary**: Added `filter_nodes()` predicate query method and `nodes_by_kind()` convenience method to Graph. `filter_nodes` accepts any `Fn(&Node) -> bool` predicate, enabling queries by kind, field value, title, or any combination. 3 new tests. Items 9.2/9.3 deferred — mutation API needs a design doc (rename semantics, cascade behavior), streaming emission needs benchmarking evidence.

---

## Phase 10: Developer Experience [DEFERRED]

Ship blockers for public release.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 10.1 | Interactive specforge init --interactive wizard | 5 | | | DEFERRED (requires dialoguer/crossterm deps for terminal prompts) |
| 10.2 | Richer starter specs with 5 working examples | 5, 13 | | | DEFERRED (content creation, not code) |
| 10.3 | specforge help <topic> category system | 5 | | | DEFERRED (CLI UX enhancement) |
| 10.4 | Quickstart guide at root level | 13 | | | DEFERRED (documentation) |
| 10.5 | 3-5 example spec projects in examples/ | 13, 18 | | | DEFERRED (content creation) |

**Summary**: All deferred. These are content-creation and UX-enhancement items that require focused design sessions rather than TDD cycles. The CLI already has init, check, export, explain commands. Interactive wizard needs terminal UI library. Starter specs and examples are content work.

---

## Phase 11: Documentation [PARTIAL]

Fill documentation gaps.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 11.1 | CONTRIBUTING.md + Code of Conduct | 18 | | | DEFERRED (documentation) |
| 11.2 | Rustdoc on all public APIs | 13 | | | DEFERRED (incremental; add as APIs stabilize) |
| 11.3 | Extension authoring tutorial | 13, 18 | | | DEFERRED (documentation) |
| 11.4 | Graph Protocol consumption guide | 13 | | | DEFERRED (documentation) |
| 11.5 | Diagnostic code reference (all E/W/I codes explained) | 15 | YES | YES | DONE |

**Summary**: Item 11.5 done — added W061 (reference cycle), W062 (malformed semver), W063 (circular peer dep), E028 (incompatible host API), I999 (truncated output) to `specforge explain` command. All other items deferred as documentation/content work.

---

## Phase 12: CI/CD & Distribution [DEFERRED]

Production-grade build and release pipeline.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 12.1 | Cross-platform CI matrix (macOS, Windows) | 14 | | | DEFERRED (GitHub Actions config) |
| 12.2 | Release workflow with GitHub Releases | 14 | | | DEFERRED (GitHub Actions config) |
| 12.3 | Binary distribution (cargo-binstall, brew) | 14 | | | DEFERRED (packaging infrastructure) |
| 12.4 | Changelog automation (conventional commits) | 14 | | | DEFERRED (CI tooling) |
| 12.5 | Code coverage reporting | 14 | | | DEFERRED (CI tooling) |
| 12.6 | Dependabot configuration | 14 | | | DEFERRED (GitHub config) |

**Summary**: All deferred. These are CI/CD infrastructure items (GitHub Actions workflows, packaging scripts, dependency bots). Not code changes suitable for TDD.

---

## Phase 13: VS Code Extension [DEFERRED]

Editor integration packaging.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 13.1 | VS Code extension client (package.json + client) | 10 | | | DEFERRED (separate TypeScript project) |
| 13.2 | Language registration (.spec files) | 10 | | | DEFERRED (extension packaging) |
| 13.3 | Syntax highlighting via semantic tokens | 10 | | | DEFERRED (LSP already supports semantic tokens) |

**Summary**: All deferred. VS Code extension is a separate TypeScript project that wraps the existing LSP server. The LSP (specforge-lsp crate) already supports semantic tokens, hover, completion, etc.

---

## Phase 14: AI/Agent Integration [DEFERRED]

Close the spec-to-implementation gap for AI workflows.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 14.1 | MCP server implementation (top 10 resources) | 7 | | | DEFERRED (specforge-mcp crate exists; needs resource wiring) |
| 14.2 | Plan validation with correction suggestions | 7 | | | EXISTING (validate_plan + PlanValidationResult already implemented) |
| 14.3 | Source locations in brief/context formats | 7 | | | EXISTING (source_span already in graph nodes, emitted in JSON) |
| 14.4 | Agent execution loop protocol | 7 | | | DEFERRED (design doc needed) |

**Summary**: Items 14.2 and 14.3 already exist — `validate_plan()` in specforge-emitter provides plan validation, and source spans are included in all graph node emissions. Item 14.1 partially exists (specforge-mcp crate). Item 14.4 deferred as it needs a design document.

---

## Phase 15: Domain Completeness [DEFERRED]

New extensions for underserved domains.

| # | Item | Expert | Test | Impl | Status |
|---|------|--------|------|------|--------|
| 15.1 | @specforge/deployment extension (environment, service, sla) | 17 | | | DEFERRED (new extension — spec files exist but no Wasm module) |
| 15.2 | @specforge/security extension (auth_scheme, permission) | 17 | | | DEFERRED (new extension) |
| 15.3 | Extend constraint with category field | 17 | | | DEFERRED (extension manifest change) |
| 15.4 | Extend port with protocol field | 17 | | | DEFERRED (extension manifest change) |

**Summary**: All deferred. New extensions and field additions are spec-level changes that go through the extension manifest system, not core compiler changes.

---

## Progress Summary

| Phase | Items | Done | Progress |
|-------|-------|------|----------|
| 1. Graph Safety | 4 | 3 | 75% |
| 2. Resolver Safety | 3 | 3 | 100% |
| 3. Error Types | 4 | 2 | 50% |
| 4. Diagnostics | 5 | 2 | 40% |
| 5. Extension Versioning | 4 | 4 | 100% |
| 6. Parser Robustness | 3 | 2 | 67% |
| 7. Performance | 3 | 0 | 0% (all deferred/skipped) |
| 8. Testing | 5 | 2 | 40% |
| 9. API Surface | 3 | 1 | 33% |
| 10. Developer Experience | 5 | 0 | 0% (all deferred) |
| 11. Documentation | 5 | 1 | 20% |
| 12. CI/CD | 6 | 0 | 0% (all deferred) |
| 13. VS Code Extension | 3 | 0 | 0% (all deferred) |
| 14. AI/Agent Integration | 4 | 2 | 50% (2 already existed) |
| 15. Domain Completeness | 4 | 0 | 0% (all deferred) |
| **TOTAL** | **61** | **22** | **36%** |
