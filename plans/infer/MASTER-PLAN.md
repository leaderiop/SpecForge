# SpecForge Infer — Master Plan

**Goal:** Enable AI agents to infer .spec files from existing codebases with progress tracking, gap detection, and quality validation. SpecForge provides the feedback loop; agents do the creative work.

**Core principle:** The graph IS the progress state. No special runtime — agents use MCP tools/prompts, write .spec files, validate with `specforge check`, and track progress via `specforge-infer.json`.

**Invariant:** `specforge-infer.json` is operational metadata — it NEVER appears in Graph Protocol exports. The graph contains only validated spec entities.

---

## What Already Exists

| Component | Status | Location |
|-----------|--------|----------|
| `specforge://prompts/infer` (overview, kind:X, file:X) | Implemented | `crates/specforge-mcp/src/prompts/infer.rs` |
| `inference_guide` field on ManifestEntityKind | Implemented | All 4 builtin extensions declare guides |
| `InferenceConfig` in specforge.json (global + per-kind overrides) | Implemented | `crates/specforge-common/src/project.rs` |
| `specforge validate` MCP tool | Implemented | Agent can validate after writing specs |
| `specforge stats` MCP tool | Implemented | Agent can measure progress |
| `specforge://prompts/review` (coverage gaps) | Implemented | Agent can identify gaps |
| `specforge://prompts/explore` (orphans, topology) | Implemented | Agent can find disconnected entities |

## What Needs to Be Built

---

## Phase 1: Inference Progress Manifest (`specforge-infer.json`)

**Leaf phase** — the data model for tracking which source files have been analyzed, which entities were produced, and what remains.

**Deliverables:**
- `InferenceManifest` type in `specforge-common` (Serde-serializable)
  - `version: u32` (schema version, starts at 1)
  - `source_roots: Vec<String>` (relative paths to source trees — supports workspaces with multiple crate/module roots)
  - `source_index: HashMap<String, SourceFileEntry>` (source path → analysis state; serialized as sorted Vec for stable diffs)
- `SourceFileEntry` type
  - `content_hash: String` (SHA-256 for staleness detection)
  - `entities_produced: Vec<String>` (entity IDs)
  - `analyzed_at: String` (ISO 8601)
- `InferenceSummary` type (computed on load, NEVER persisted)
  - `files_total: usize` (discovered source files across all source_roots)
  - `files_analyzed: usize`
  - `entities_produced: usize`
  - `needs_review: usize`
- Load/save functions: `load_inference_manifest(path)`, `save_inference_manifest(path, manifest)`
- Atomic write via temp-file-then-rename (new infrastructure — no precedent in codebase)
- `specforge init` adds `specforge-infer.json` to `.gitignore` (machine-local state)
- Unit tests: round-trip serialization, summary computation from source_index, staleness detection, missing file returns default

**Design decisions (from expert review):**
- Summary is computed-on-load, not stored — prevents drift between source_index and summary
- Sessions deferred to Phase 2 — Phase 1 manifest is minimal (just source_index + metadata)
- HashMap in memory, sorted Vec in JSON — O(1) lookup + stable diffs
- source_roots is Vec to support Cargo workspaces (e.g., `["crates/specforge-mcp/src", "crates/specforge-cli/src"]`)

**Dependencies:** None
**Estimated changes:** ~200 lines in `crates/specforge-common/src/`

---

## Phase 2: MCP Tools for Inference Progress

**Leaf phase** — two MCP tools: one read-only, one mutating. Split per Expert 9 review.

**Deliverables:**

### Tool 1: `specforge_infer_progress` (read-only)
- Returns current manifest summary + unanalyzed files + stale files
- Auto-discovers source files from `source_roots` (respects .gitignore, excludes target/, node_modules/, etc.)
- Content hash computation for staleness detection (SHA-256)
- Returns: `{ "summary": InferenceSummary, "unanalyzed": [paths], "stale": [paths] }`

### Tool 2: `specforge_infer_session` (mutating)
- `action: "start" | "mark_analyzed" | "end"`
- `start`: Creates new session entry in manifest, returns session_id
  - Adds `InferenceSession` to manifest (session_id, started_at, agent, status=active)
  - Only one active session allowed
- `mark_analyzed`: Records source file with content hash + entity IDs
  - Idempotent — re-analyzing updates existing entry
  - Agent should call AFTER `specforge validate` succeeds (retry pattern: fix errors → re-mark)
- `end`: Marks session as completed/paused
- `InferenceSession` type added in this phase (deferred from Phase 1)
  - `session_id: String`, `started_at: String`, `ended_at: Option<String>`, `agent: String`, `status: SessionStatus`

### MCP registration wiring
- New entries in `default_tools()` in `registry.rs` with JSON Schema definitions
- Handler modules in `tools/infer_progress.rs` and `tools/infer_session.rs`

- Unit tests for each action
- Integration test: start → mark × N → end → read progress

**Dependencies:** Phase 1 (manifest types)
**Estimated changes:** ~350 lines in `crates/specforge-mcp/src/tools/`

---

## Phase 3: `specforge infer-status` CLI Command

**Leaf phase** — CLI surface for humans to check inference progress.

**Deliverables:**
- New CLI subcommand `specforge infer-status`
  - No args: prints summary table (files analyzed / total, entities produced, per-directory counts)
  - `--format json`: machine-readable output
  - `--gaps`: lists unanalyzed source files grouped by directory with counts per directory
  - `--stale`: lists files whose content changed since last analysis
- Reads `specforge-infer.json` from project root
- Pretty-printed table output with colors (similar to `specforge stats`)
- If no `specforge-infer.json` exists, prints helpful message ("No inference data yet. Use an AI agent with specforge://prompts/infer to get started.")

**Dependencies:** Phase 1 (manifest types)
**Estimated changes:** ~150 lines in `crates/specforge-cli/src/`

---

## Phase 4: Enhanced Infer Prompt — `scope: "plan"` and `scope: "workflow"` Modes

**Leaf phase** — teaches agents the optimal inference order and the full session protocol.

**Deliverables:**

### `scope: "plan"` mode
- Returns prioritized work plan based on:
  1. Entity kinds with zero existing entities (highest priority)
  2. Dependency order: types before behaviors, behaviors before events
  3. Source files grouped by module/directory
- Integrates with `specforge-infer.json` if present (excludes already-analyzed files)
- Returns structured JSON: `{ "phases": [{ "kind": "type", "priority": 1, "existing_count": 0, "suggested_files": [...] }] }`
- **Spec-file placement guidance** (from Expert 10): each phase includes a `target_spec_directory` hint (e.g., `"spec/types/"` for type entities, `"spec/behaviors/"` for behaviors) so agents know WHERE to write .spec files, not just what to write
- Requires explicit `Some("plan")` match arm in dispatch (Expert 9 note)

### `scope: "workflow"` mode
- Returns the step-by-step agent protocol with tool names:
  1. Call `specforge_infer_session` with action=start
  2. Call `specforge_infer_progress` to get unanalyzed files
  3. Read source files, write .spec files
  4. Call `specforge_validate` to check errors, fix any errors
  5. Call `specforge_infer_session` with action=mark_analyzed
  6. Repeat steps 2-5 until satisfied
  7. Call `specforge_infer_session` with action=end
- Teaches retry pattern: if validate fails, fix .spec → re-validate → then mark

- Unit tests: plan ordering, workflow content, file exclusion

**Dependencies:** Phase 1 (manifest, optional), Phase 2 (MCP tools, optional — workflow references tool names)
**Estimated changes:** ~150 lines in `crates/specforge-mcp/src/prompts/infer.rs`

---

## Phase 5: Inference Gap Analysis

**Renamed from "Inference Coverage" to avoid collision with existing `specforge_coverage` tool (Expert 2).**

**Leaf phase** — measures how much of the codebase has been spec'd. Language-specific heuristics are extension-contributed per Principles 2 & 7.

**Architecture (from Expert 1, 4, 5 review):**
- **Core provides:** aggregation framework — collects gap reports from extensions, cross-references with graph, computes per-directory rollups
- **Extensions contribute:** language-specific item detection via surface commands (e.g., `@specforge/rust` contributes `cmd__scan_public_items` that uses tree-sitter-rust to extract pub items with metadata)
- **Phase 5 v1:** Core aggregation only, with a simple built-in Rust fallback (regex-based `pub fn/struct/enum/trait` detection) explicitly marked as approximate. Real detection deferred to extension surface commands.

**Deliverables:**
- New MCP tool `specforge_infer_gaps`:
  - Scans source files in `source_roots`
  - v1: regex-based pub item detection for Rust (explicitly marked approximate in output)
  - Cross-references with compiled graph entities via `specforge-infer.json` source_index
  - Returns per-directory gap report with structured `InferenceGap` items (not bare strings)
- `InferenceGap` type with `name: String`, `item_kind: String` (fn/struct/enum/trait), `file: String`, `line: usize`
- CLI integration: `specforge infer-status --gaps-detail`

**Design decisions:**
- Coverage metric is **matched/unmatched** (is there at least one entity referencing this source item?) not ratio-based (Expert 4)
- Heuristic is Rust-only for v1, explicitly documented (Expert 4)
- No false claim of multi-language support until extensions contribute

**Dependencies:** Phase 1 (manifest), Phase 3 (CLI for display)
**Estimated changes:** ~250 lines across `crates/specforge-mcp/` and `crates/specforge-cli/`

---

## Phase 6: Inference Quality Diagnostics

**Leaf phase** — optional lint profile for inferred specs.

**Deliverables:**
- New diagnostic codes (info level, `--lint=inferred` profile):
  - `I200`: Source anchor stale — entity's source file changed since inference
  - `I202`: High inference density — >N% of entities in a spec file are from one session (default 80%, configurable via `inference.density_threshold` in specforge.json)
- I201 (low confidence) intentionally omitted — requires LLM self-assessment protocol (out of scope)
- `specforge-infer.json` integration in the diagnostic pipeline:
  - Load manifest during compilation (like `specforge-cache.json`)
  - Cross-reference entity IDs with infer manifest entries
  - Check content hashes for staleness
- `--lint=inferred` flag on `specforge check` and `specforge validate`
- Also configurable via `"lint": ["inferred"]` in specforge.json for CI persistence
- Unit tests for each diagnostic code

**Dependencies:** Phase 1 (manifest), existing diagnostic infrastructure
**Estimated changes:** ~200 lines in `crates/specforge-emitter/` and `crates/specforge-registry/`

---

## Phase 7: Spec Files for All New Behaviors

**Already partially complete — specs written before expert review, now updated with fixes.**

**Deliverables:**
- `spec/behaviors/infer.spec` — behavioral specifications for all new behaviors ✓ (needs update)
- `spec/types/infer.spec` — type definitions for InferenceManifest, InferenceSession, etc. ✓ (needs update)
- `spec/features/infer.spec` — feature declarations for the infer workflow ✓
- All entities must pass `specforge check` with zero errors

**Dependencies:** None (write specs first, implement second)

---

## Implementation Order

```
Phase 7 (specs)     ←── Update with expert review fixes
    ↓
Phase 1 (manifest)  ←── Foundation: data types (minimal — no sessions)
    ↓
Phase 2 (MCP tools) ←── Agent-facing: progress + session (two tools)
Phase 3 (CLI)       ←── Human-facing: progress viewing (parallel with Phase 2)
    ↓
Phase 4 (prompts)   ←── Agent workflow: plan + workflow scopes
    ↓
Phase 5 (gaps)      ←── Measurement: gap analysis (Rust regex v1)
    ↓
Phase 6 (diagnostics) ←── Quality: lint for inferred specs
```

## Out of Scope (Future)

- `cmd__infer_hints` / `cmd__scan_public_items` Wasm surface commands on `@specforge/rust` (static AST analysis — requires tree-sitter-rust in Wasm)
- `@specforge/typescript` inference support
- Confidence scoring per entity / I201 diagnostic (requires LLM self-assessment protocol)
- Automatic "promotion" workflow (inferred → reviewed → accepted)
- Bidirectional sync (code changes → spec staleness alerts in LSP)
- `specforge.write_spec` MCP tool for pure-MCP agents (requires filesystem grant protocol)
- Multi-file entity tracking (many source files → one entity)
