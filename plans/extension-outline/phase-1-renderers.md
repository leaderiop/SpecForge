# Phase 1: Four Renderers

**Status**: NOT STARTED
**Depends on**: Phase 0 (IR types & builder)
**Crate**: `specforge-emitter`
**Directory**: `crates/specforge-emitter/src/outline/`

---

## Goal

Implement 4 format-specific renderers that convert `OutlineIntermediate` to output strings. Each renderer handles all 3 detail levels (None/Keys/All).

---

## Checklist

### 1.1: Markdown renderer (`outline/markdown.rs`)

- [ ] Create `crates/specforge-emitter/src/outline/markdown.rs`
- [ ] `render_markdown(outline: &OutlineIntermediate, options: &OutlineOptions) -> String`
- [ ] **Detail=None**: Overview table (extension, version, entity count, edge count, rule count, enhances summary) + Dependencies section
- [ ] **Detail=Keys**: + per-extension sections with entity kind names, edge labels, contributes flags, verify kinds
- [ ] **Detail=All**: + full field table per entity kind with source attribution (which extension contributed each field)
- [ ] Focus on hierarchy: Dependencies and Enhancements sections are prominent, entity detail is secondary

**Markdown structure** (at Keys level):
```
# Extension Architecture
## Overview (table)
## Dependencies (arrow list)
## Enhancements (arrow list with field counts)
## Cross-Extension Edges (if any)
## Extensions
### @ext/name (counts)
  Entity kinds: ...
  Contributes: ...
```

### 1.2: Mermaid renderer (`outline/mermaid.rs`)

- [ ] Create `crates/specforge-emitter/src/outline/mermaid.rs`
- [ ] `render_mermaid(outline: &OutlineIntermediate, options: &OutlineOptions) -> String`
- [ ] `graph TD` layout (top-down, hierarchy-friendly)
- [ ] Extension nodes with name + counts in label
- [ ] Solid arrows = required peer dependencies (`-->|"depends ^1.0"|`)
- [ ] Dashed arrows = optional peer dependencies (`-.->|"depends ^1.0 (optional)"|`)
- [ ] Dotted arrows = enhancements (`-. "enhances kind (+N)" .->`)
- [ ] Detail=All adds entity kind names to node labels

### 1.3: DOT renderer (`outline/dot.rs`)

- [ ] Create `crates/specforge-emitter/src/outline/dot.rs`
- [ ] `render_dot(outline: &OutlineIntermediate, options: &OutlineOptions) -> String`
- [ ] Color-coded extension nodes (product=#2ecc71, software=#4a90d9, governance=#e74c3c, formal=#9b59b6)
- [ ] HTML `<table>` labels with extension name, version, entity/edge counts
- [ ] `rankdir=TB` (top to bottom)
- [ ] Solid edges = required deps, dashed = optional deps, dotted = enhancements
- [ ] Detail=All adds entity list inside node table

### 1.4: JSON renderer (`outline/json.rs`)

- [ ] Create `crates/specforge-emitter/src/outline/json.rs`
- [ ] `render_json(outline: &OutlineIntermediate, options: &OutlineOptions) -> String`
- [ ] `serde_json::to_string_pretty` on `OutlineIntermediate`
- [ ] Detail=None: strip entity/edge/rule details (only counts)
- [ ] Detail=Keys: include names but not field-level attribution
- [ ] Detail=All: full serialization including field attribution

### 1.5: Wire renderers to dispatcher

- [ ] Add `mod markdown; mod mermaid; mod dot; mod json;` in `outline/mod.rs`
- [ ] Implement `render()` dispatcher matching on `OutlineFormat`

---

## Verify

```bash
cargo test -p specforge-emitter  # snapshot tests for all 4 formats x 3 detail levels (12 snapshots)
cargo clippy -p specforge-emitter  # zero warnings
```
