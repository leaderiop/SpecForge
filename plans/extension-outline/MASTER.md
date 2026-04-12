# Extension Outline Command — Master Plan

**Goal**: Add `specforge outline` command and `specforge.outline_extensions` MCP tool that renders the extension hierarchy — dependencies, enhancements, contributions, and field attribution.
**Created**: 2026-04-12
**Status**: COMPLETE — all 9 phases done, 30 tests passing, zero clippy warnings

## Progress

| Phase | Name | Items | Status | Plan |
|-------|------|-------|--------|------|
| 0 | IR Types & Builder | 14 | COMPLETE | [phase-0](phase-0-ir-types-builder.md) |
| 1 | Four Renderers | 16 | COMPLETE | [phase-1](phase-1-renderers.md) |
| 2 | CLI Wiring | 8 | COMPLETE | [phase-2](phase-2-cli-wiring.md) |
| 3 | MCP Wiring | 8 | COMPLETE | [phase-3](phase-3-mcp-wiring.md) |
| 4 | Pipeline Integration | 6 | COMPLETE | [phase-4](phase-4-pipeline-integration.md) |
| 5 | Tests | 12 | COMPLETE | [phase-5](phase-5-tests.md) |
| 6 | IR Enrichments | 12 | COMPLETE | [phase-6](phase-6-ir-enrichments.md) |
| 7 | Renderer Improvements | 20 | COMPLETE | [phase-7](phase-7-renderer-improvements.md) |
| 8 | MCP & AI Optimization | 7 | COMPLETE | [phase-8](phase-8-mcp-ai-optimization.md) |

**Total tracking items**: 103

## Dependency Graph

```
Phase 0 (IR types + builder)
  |
  v
Phase 1 (4 renderers: md, mermaid, dot, json)
  |
  +--------> Phase 4 (pipeline: expose manifests)
  |              |
  v              v
Phase 2 (CLI) --+
  |
  v
Phase 3 (MCP)
  |
  v
Phase 5 (tests: builder, snapshots, E2E)
```

Note: Phase 4 can be done in parallel with Phase 1, but must complete before Phase 2.

## Architecture

```
ManifestV2[]  ──>  OutlineIntermediate  ──>  Renderer  ──>  String
                        |
                   from_manifests()        render(outline, options)
```

- **Data source**: `ManifestV2` (not `GraphProtocolSchema`) — outline needs peer_deps, validation_rules, surfaces, contributes flags
- **IR**: `OutlineIntermediate` captures extensions, dependencies, enhancements, cross-edges
- **Formats**: Markdown (default, LLM-friendly), Mermaid (graph TD), DOT (colored), JSON
- **Detail tiers**: None (overview table only), Keys (entity/edge names), All (full field attribution)

## Key Decisions

1. **MCP tool name**: `specforge.outline_extensions` (not `specforge.outline` — already taken by file-level entity outline)
2. **Build from manifests**: ManifestV2 has peer_deps, validation_rules, surfaces, contributes — all needed for the hierarchy view. GraphProtocolSchema deliberately omits these.
3. **Focus on hierarchy**: Markdown renderer emphasizes inter-extension relationships over entity detail
4. **4 formats, no DBML**: DBML is database-oriented, doesn't fit extension hierarchy

## Files Summary

**New files (9)**:
- `crates/specforge-emitter/src/outline/mod.rs` — IR types, enums, render dispatcher
- `crates/specforge-emitter/src/outline/build.rs` — `OutlineIntermediate_from_manifests()`
- `crates/specforge-emitter/src/outline/markdown.rs` — Markdown renderer
- `crates/specforge-emitter/src/outline/mermaid.rs` — Mermaid graph TD renderer
- `crates/specforge-emitter/src/outline/dot.rs` — DOT/Graphviz renderer
- `crates/specforge-emitter/src/outline/json.rs` — JSON renderer with 3 detail levels
- `crates/specforge-cli/src/outline.rs` — CLI handler
- `crates/specforge-mcp/src/tools/outline_extensions.rs` — MCP tool handler
- `crates/specforge-emitter/tests/outline.rs` — 18 tests (builder + renderers)

**Modified files (8)**:
- `crates/specforge-emitter/src/lib.rs` — add `pub mod outline`
- `crates/specforge-emitter/src/compile.rs` — add `manifests` field to `CompilationContext`
- `crates/specforge-emitter/tests/main.rs` — add `mod outline`
- `crates/specforge-cli/src/main.rs` — add mod, `Outline` command variant, dispatch
- `crates/specforge-mcp/src/state.rs` — add `manifests` field to `McpState`
- `crates/specforge-mcp/src/compile.rs` — add `manifests` to `CompileResult`
- `crates/specforge-mcp/src/lifecycle.rs` — wire manifests from CompileResult to McpState
- `crates/specforge-mcp/src/tools/validate.rs` — wire manifests on recompilation
- `crates/specforge-mcp/src/tools/mod.rs` — add mod + dispatch for `outline_extensions`
- `crates/specforge-mcp/src/registry.rs` — add tool descriptor for `outline_extensions`

## Verification Plan

After all phases:

1. `cargo build --workspace` — zero errors
2. `cargo test --workspace` — zero failures, zero regressions
3. `specforge outline` → Markdown showing 4 extensions
4. `specforge outline --format=mermaid|dot|json` → valid output
5. `specforge outline --fields=all` → full field attribution
6. MCP: `specforge.outline_extensions` → Markdown response
7. `cargo clippy --workspace` — zero new warnings
