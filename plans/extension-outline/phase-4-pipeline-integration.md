# Phase 4: Pipeline Integration

**Status**: NOT STARTED
**Depends on**: — (can start in parallel with Phase 1)
**Crates**: `specforge-emitter`, `specforge-cli`, `specforge-mcp`

---

## Goal

Both CLI and MCP need access to raw `ManifestV2` data to build the outline. Currently the compilation pipeline may discard manifests after populating registries. This phase ensures manifests are accessible.

---

## Investigation (do first)

- [ ] Read `crates/specforge-emitter/src/compile.rs` — does `CompilationContext` store manifests?
- [ ] Read `crates/specforge-cli/src/pipeline.rs` (if exists) — where are manifests loaded?
- [ ] Read `crates/specforge-mcp/src/lifecycle.rs` — does `McpState` have access to manifests?

---

## Options

### Option A: Add manifests to CompilationContext (preferred)

- [ ] Add `pub manifests: Vec<ManifestV2>` field to `CompilationContext`
- [ ] Populate during compilation (manifests are already deserialized — just keep them)
- [ ] CLI reads `ctx.manifests` in `outline::run()`
- [ ] MCP reads from state (which wraps or mirrors CompilationContext)

**Pro**: Clean, single source of truth, no duplicate loading
**Con**: Slightly increases CompilationContext memory footprint

### Option B: Build outline from registries

- Use `KindRegistry`, `EdgeRegistry`, `FieldRegistry` (all have `source_extension`)
- **Missing**: peer_dependencies, validation_rules, contributes flags, surfaces
- Would need registry enrichment — more invasive
- **Not recommended** unless Option A is blocked

### Option C: Separate manifest loading

- Add `load_manifests(path) -> Vec<ManifestV2>` standalone function
- CLI outline calls this directly, bypassing compilation
- **Pro**: No changes to CompilationContext
- **Con**: Duplicates file loading logic, may diverge from compilation path

---

## Decision

Determine during investigation. Option A is cleanest if CompilationContext is easily extended. If CompilationContext is frozen or heavily constrained, fall back to Option C.

---

## Checklist (after investigation)

- [ ] Implement chosen option
- [ ] Verify CLI can access manifests: `outline::run()` receives `Vec<ManifestV2>`
- [ ] Verify MCP can access manifests: `outline_extensions::call()` receives `Vec<ManifestV2>`

---

## Verify

```bash
cargo build --workspace  # compiles with manifest exposure
cargo test --workspace  # no regressions from pipeline changes
```
