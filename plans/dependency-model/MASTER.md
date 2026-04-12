# Dependency Model — Strict One-Way DAG + Transitive Computation + Visibility Modes

**Goal**: Enforce strictly one-way dependencies (no reverse arrows, not even optional), compute transitive dependencies with optional propagation, and add three dependency visibility modes (`--deps=direct|effective|full`).

**Created**: 2026-04-12
**Status**: COMPLETE

## Progress

| Phase | Name | Items | Status | Plan |
|-------|------|-------|--------|------|
| 0 | Manifest Cleanup | 9 | COMPLETE | [phase-0](phase-0-manifest-cleanup.md) |
| 1 | Transitive Computation | 8 | COMPLETE | [phase-1](phase-1-transitive-computation.md) |
| 2 | Visibility Modes | 7 | COMPLETE | [phase-2](phase-2-visibility-modes.md) |
| 3 | Tests | 10 | COMPLETE | [phase-3](phase-3-tests.md) |

**Total tracking items**: 34

## Dependency Graph

```
Phase 0 (manifest cleanup — move fields, remove reverse deps)
  |
  v
Phase 1 (transitive computation — builder enrichment)
  |
  v
Phase 2 (visibility modes — CLI flag + renderer filtering)
  |
  v
Phase 3 (tests — all phases verified)
```

Strictly sequential: each phase depends on the previous.

## The Problem

### 1. Reverse dependencies exist (architectural violation)

Product declares `peerDependencies: [{ name: "@specforge/software", optional: true }]`. This creates a reverse arrow in the DAG. The reason: product's `library` entity has `ports_defined`/`ports_consumed` fields targeting software's `port` kind, and `roadmap` has `behaviors` targeting software's `behavior` kind.

**Fix**: Move those 3 fields + 3 edge types from product's manifest to software's `entityEnhancements`. Software already does this pattern (enhances product's `module` and `milestone`). Product becomes a true standalone root with zero peer_dependencies.

### 2. Governance has redundant transitive dep

Governance declares `peerDependencies: [software, product(optional)]`. But governance → software → product already covers the transitive path. The explicit governance → product dep is redundant.

**Fix**: Remove governance → product. Compute it transitively at build time.

### 3. No transitive dependency computation

The outline builder maps `peerDependencies` 1:1 to `OutlineDependency`. It has no concept of transitive deps. Users can't see that governance implicitly depends on product through software.

### 4. No visibility control

All dependencies are always shown. Users need three levels:
- **Direct**: Only explicitly declared peer_dependencies (clean graph)
- **Effective**: Direct + transitive deps where the extension actually references kinds/edges from that dep
- **Full**: All transitive deps, including unused ones

## Correct Dependency DAG (after fixes)

```
product (root, ZERO deps)
  ^
  |  required
  |
software
  ^         ^
  |          \
  | required   required
  |             \
formal       governance
```

All arrows point upward. No reverse arrows. No optional reverse deps. Product is the standalone root.

## Transitive Computation Rules

1. **Closure**: If A → B and B → C, then A has transitive dep on C
2. **Optional propagation**: If any link in the chain is optional, the transitive dep is optional (weakest link)
3. **Effective detection**: A transitive dep A → C is "effective" if A references any kind owned by C (via edge_types targetKind/sourceKind, entity_enhancement targetKind, or field targetKind)
4. **No self-loops**: Skip transitive deps where from == to

## Three Visibility Modes

| Mode | CLI flag | What's shown | Default for |
|------|----------|-------------|-------------|
| `direct` | `--deps=direct` | Only declared peer_dependencies | mermaid, dot |
| `effective` | `--deps=effective` | Direct + transitive deps with actual kind references | markdown |
| `full` | `--deps=full` | All transitive deps including unused | (explicit only) |

## Fields/Edges to Move (Phase 0)

### From product to software (as entityEnhancements)

**Fields to remove from product's `library` entityKind:**
- `ports_defined` (edge: `LibraryDefinesPort`, targetKind: `port`)
- `ports_consumed` (edge: `LibraryConsumesPort`, targetKind: `port`)

**Fields to remove from product's `roadmap` entityKind:**
- `behaviors` (edge: `RoadmapPlansBehavior`, targetKind: `behavior`)

**Edge types to remove from product's `edgeTypes`:**
- `LibraryDefinesPort` (sourceKind: library, targetKind: port)
- `LibraryConsumesPort` (sourceKind: library, targetKind: port)
- `RoadmapPlansBehavior` (sourceKind: roadmap, targetKind: behavior)

**New entityEnhancements to add to software's manifest:**
```json
{
  "targetKind": "library",
  "sourceExtension": "@specforge/product",
  "fields": [
    { "name": "ports_defined", "description": "Ports (interfaces) defined by this library", "fieldType": "reference_list", "edge": "LibraryDefinesPort", "targetKind": "port" },
    { "name": "ports_consumed", "description": "Ports (interfaces) consumed by this library", "fieldType": "reference_list", "edge": "LibraryConsumesPort", "targetKind": "port" }
  ]
}
```
```json
{
  "targetKind": "roadmap",
  "sourceExtension": "@specforge/product",
  "fields": [
    { "name": "behaviors", "description": "Behaviors planned in this roadmap", "fieldType": "reference_list", "edge": "RoadmapPlansBehavior", "targetKind": "behavior" }
  ]
}
```

**New edge types to add to software's `edgeTypes`:**
- `LibraryDefinesPort` (sourceKind: library, targetKind: port, edgeStyle: solid, edgeColor: #795548)
- `LibraryConsumesPort` (sourceKind: library, targetKind: port, edgeStyle: dashed, edgeColor: #795548)
- `RoadmapPlansBehavior` (sourceKind: roadmap, targetKind: behavior, edgeStyle: solid, edgeColor: #3F51B5)

## Files Summary

**Modified files:**
- `extensions/product/manifest.json` — remove 3 fields, 3 edge types, set peerDependencies to []
- `extensions/software/manifest.json` — add 2 entityEnhancements, 3 edge types
- `extensions/governance/manifest.json` — remove product optional dep
- `crates/specforge-emitter/src/outline/mod.rs` — add DependencyKind enum, DependencyDepth enum, update OutlineDependency, OutlineOptions
- `crates/specforge-emitter/src/outline/build.rs` — add transitive closure computation, effective detection
- `crates/specforge-emitter/src/outline/mermaid.rs` — filter deps by visibility mode
- `crates/specforge-emitter/src/outline/markdown.rs` — filter deps, label transitive/effective
- `crates/specforge-emitter/src/outline/dot.rs` — filter deps by visibility mode
- `crates/specforge-emitter/src/outline/json.rs` — include dependency kind in output
- `crates/specforge-cli/src/main.rs` — add --deps flag to Outline command
- `crates/specforge-cli/src/outline.rs` — pass deps option through
- `crates/specforge-emitter/tests/outline.rs` — new + updated tests

## Verification Plan

After all phases:

1. `cargo build --workspace` — zero errors
2. `cargo test --workspace` — zero failures, zero regressions
3. `specforge outline` — product has zero deps, no reverse arrows
4. `specforge outline --deps=direct` — only declared peer_dependencies shown
5. `specforge outline --deps=effective` — governance shows transitive dep on product (used via cross-edges)
6. `specforge outline --deps=full` — all transitive deps shown
7. `specforge outline --format=mermaid` — clean one-way DAG
8. `cargo clippy --workspace` — zero warnings
9. Product manifest passes consistency validation with zero W021 warnings
