# Outline V2 — Dependency Model Fix + Mermaid Renderer Redesign

**Goal**: Fix the dependency model (circular deps, missing `optional` flag) and rewrite the Mermaid renderer to use the subgraph card design validated through visual iteration.
**Created**: 2026-04-12
**Status**: PENDING

## Progress

| Phase | Name | Items | Status | Plan |
|-------|------|-------|--------|------|
| 0 | Dependency Model Fixes | 8 | PENDING | [phase-0](phase-0-dependency-model.md) |
| 1 | Mermaid Renderer Rewrite | 12 | PENDING | [phase-1](phase-1-mermaid-rewrite.md) |
| 2 | Other Renderer Updates | 5 | PENDING | [phase-2](phase-2-renderer-updates.md) |
| 3 | Tests | 10 | PENDING | [phase-3](phase-3-tests.md) |

**Total tracking items**: 35

## Dependency Graph

```
Phase 0 (dependency model)
  |
  +--------> Phase 2 (other renderers: md, dot, json)
  |
  v
Phase 1 (mermaid rewrite)
  |
  v
Phase 3 (tests)
```

Phase 2 can run in parallel with Phase 1 (both depend only on Phase 0).

## Bugs Discovered

### Bug 1: Circular dependency product↔software

**Evidence** (manifest.json files):
- `extensions/product/manifest.json:547` → `peerDependencies: [{ name: "@specforge/software" }]`
- `extensions/software/manifest.json:21` → `peerDependencies: [{ name: "@specforge/product" }]`

Both declare each other as required peer dependencies. This violates the DAG constraint and should trigger W063 (circular dependency detection).

**Root cause analysis**: Software legitimately depends on product — the `Implements` edge (behavior→feature) and `MilestoneBehavior` entity_enhancement both cross from software into product's entity kinds. Product's dependency on software appears unjustified — product's 16 edge types (JourneyFeature, DeliverableJourney, etc.) all target product-owned entity kinds. No product edge type targets a software entity kind.

**Fix**: Remove product's peer_dependency on software. Product is the standalone root.

### Bug 2: `PeerDependency` struct drops `optional` field

**Evidence**:
- `extensions/governance/manifest.json:13` → `{ name: "@specforge/product", version: "^1.0", optional: true }`
- `crates/specforge-registry/src/manifest/types.rs:179-182` → struct only has `name` and `version`

The `optional: true` JSON field is silently dropped during deserialization. Governance's optional dependency on product is treated identically to a required dependency.

**Fix**: Add `#[serde(default)] pub optional: bool` to `PeerDependency`.

## Correct Dependency DAG (after fixes)

```
product (root, standalone — no peer deps)
  ^             ^
  |             |
  | required    | optional
  |             |
software    governance
  ^    ^
  |     \
  |      | required    required
  |      |
formal  governance
```

All arrows point upward. Product is the root. Software is the hub.

## Architecture: Mermaid Card Design

Validated through 5+ visual iteration rounds. Key pattern:

```
flowchart TB
    subgraph ext_name["  @specforge/name v1.0  "]
        card["<b>stats line</b><br>───────<br>keywords<br>───────<br><i>extras</i>"]
    end
```

- **Subgraph** = colored card background (extension color)
- **Single inner node** = white card body with color-matched border stroke
- **`<br>` + `───` dividers** = simulate sections (stats / keywords / extras)
- **`<b>` stats, regular keywords, `<i>` extras** = typographic hierarchy
- **Solid arrows** = required peer_dependency (one-way)
- **Dashed arrows** = optional peer_dependency
- **Dotted arrows** = entity_enhancements
- **`~~~` invisible links** = position orphan nodes

Color palette (Material Design):
| Extension | Subgraph fill | Subgraph stroke | Inner card stroke |
|-----------|---------------|-----------------|-------------------|
| product | #e8f5e9 | #2e7d32 | #66bb6a |
| software | #e3f2fd | #1565c0 | #42a5f5 |
| governance | #fffde7 | #f9a825 | #ffca28 |
| formal | #f3e5f5 | #7b1fa2 | #ab47bc |
| (fallback) | #f5f5f5 | #616161 | #9e9e9e |

## Key Decisions

1. **Product is the root** — remove its unjustified peer_dependency on software
2. **`optional` field on PeerDependency** — `#[serde(default)]` for backward compatibility
3. **Single-node-per-subgraph** — eliminates Mermaid's uncontrollable vertical gaps
4. **Color-matched inner borders** — inner card stroke matches extension color for visual cohesion
5. **Three edge styles** — solid (required dep), dashed (optional dep), dotted (enhancement)
6. **Dynamic color assignment** — cycle through palette for unknown extensions, not just 4 hardcoded

## Files Summary

**Modified files (6)**:
- `crates/specforge-registry/src/manifest/types.rs` — add `optional: bool` to `PeerDependency`
- `crates/specforge-emitter/src/outline/mod.rs` — add `optional: bool` to `OutlineDependency`
- `crates/specforge-emitter/src/outline/build.rs` — populate `optional` from manifest
- `crates/specforge-emitter/src/outline/mermaid.rs` — full rewrite (subgraph card design)
- `crates/specforge-emitter/src/outline/markdown.rs` — show optional indicator on deps
- `crates/specforge-emitter/src/outline/dot.rs` — dashed edges for optional deps
- `crates/specforge-emitter/src/outline/json.rs` — include optional field
- `crates/specforge-emitter/tests/outline.rs` — update + new tests
- `extensions/product/manifest.json` — remove peer_dependency on software

## Verification Plan

After all phases:

1. `cargo build --workspace` — zero errors
2. `cargo test --workspace` — zero failures, zero regressions
3. `specforge outline --format=mermaid` → subgraph card diagram, all deps one-way
4. `specforge outline --format=mermaid` → no circular dep edges
5. `specforge outline --format=json` → dependencies include `optional` field
6. `specforge outline --format=markdown` → optional deps marked
7. `cargo clippy --workspace` — zero warnings
8. Manual Mermaid render → clean card layout matching approved design
