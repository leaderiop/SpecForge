# Phase 1: P0 Manifest Bug Fixes

**Goal**: Eliminate 4 graph-ambiguity bugs where shared edge types or missing edge labels make the typed graph non-deterministic.

**Scope**: JSON manifests + Rust test files only. No compiler logic changes.

**Estimated effort**: ~1 hour of focused edits + test runs.

---

## Bug 1.1: Module ports edge collision (`@specforge/software`)

**File**: `extensions/software/manifest.json` lines 220-228
**Root cause**: `entityEnhancements` for product's `module` entity declares two fields (`ports` and `ports_defined`) both using `"edge": "UsesPort"`. Graph consumers cannot distinguish "module consumes port" from "module defines port." The existing `UsesPort` edge type (line 175) has `sourceKind: behavior`, so it is also semantically wrong for module-sourced edges.

### Step 1.1.1 -- Update entityEnhancements fields

**File**: `extensions/software/manifest.json`

Replace lines 224-226:
```json
{ "name": "ports", "fieldType": "reference_list", "edge": "UsesPort", "targetKind": "port" },
{ "name": "ports_defined", "fieldType": "reference_list", "edge": "UsesPort", "targetKind": "port" }
```
With:
```json
{ "name": "ports", "description": "Port interfaces this module consumes", "fieldType": "reference_list", "edge": "ModuleConsumesPort", "targetKind": "port" },
{ "name": "ports_defined", "description": "Port interfaces this module defines", "fieldType": "reference_list", "edge": "ModuleDefinesPort", "targetKind": "port" }
```

Changes:
- `ports.edge`: `UsesPort` -> `ModuleConsumesPort`
- `ports_defined.edge`: `UsesPort` -> `ModuleDefinesPort`
- Both fields gain `description` (was missing)

### Step 1.1.2 -- Add two new edge types to edgeTypes array

**File**: `extensions/software/manifest.json`

Insert before the closing `]` of the `edgeTypes` array (after the `MilestoneBehavior` entry at line 218):

```json
{
  "label": "ModuleConsumesPort",
  "description": "Module consumes a port interface",
  "sourceKind": "module",
  "targetKind": "port",
  "edgeStyle": "dashed",
  "edgeColor": "#00695C"
},
{
  "label": "ModuleDefinesPort",
  "description": "Module defines a port interface",
  "sourceKind": "module",
  "targetKind": "port",
  "edgeStyle": "solid",
  "edgeColor": "#00695C"
}
```

This brings software edgeTypes from 11 to 13.

### Step 1.1.3 -- Update software manifest tests

**File**: `crates/specforge-registry/tests/software_manifest.rs`

**Change 1**: `test_software_manifest_has_11_edge_types` -- rename and update count.

Replace:
```rust
fn test_software_manifest_has_11_edge_types() {
    let manifest = load_software_manifest();
    assert_eq!(manifest.edge_types.len(), 11);
```
With:
```rust
fn test_software_manifest_has_13_edge_types() {
    let manifest = load_software_manifest();
    assert_eq!(manifest.edge_types.len(), 13);
```

Add assertions for the two new edge labels:
```rust
assert!(labels.contains(&"ModuleConsumesPort"));
assert!(labels.contains(&"ModuleDefinesPort"));
```

**Change 2**: `test_software_manifest_populates_registries` -- add edge assertions.

Add:
```rust
assert!(edge_reg.contains("ModuleConsumesPort"));
assert!(edge_reg.contains("ModuleDefinesPort"));
```

### Step 1.1.4 -- Verify zero_entity_registries.rs

**File**: `crates/specforge-registry/tests/zero_entity_registries.rs`

This file uses a **hand-built** `software_manifest()` helper (lines 41-79), not the real manifest JSON. It has only 1 edge type (`enforces`). No changes needed -- the tests are self-contained with synthetic data.

### Step 1.1.5 -- Verification

```bash
cargo test -p specforge-registry --test software_manifest
cargo test -p specforge-registry --test zero_entity_registries
```

---

## Bug 1.2: Feature.features unnamed edge (`@specforge/product`)

**File**: `extensions/product/manifest.json` line 38
**Root cause**: The `features` field on the `feature` entity has `"targetKind": "feature"` but no `"edge"` key. Every other reference/reference_list field across all 4 manifests has a named edge. This creates a broken/unnamed edge in the graph. The `depends_on` field already uses `FeatureDependsOn`, so this field represents a non-dependency relationship ("related features").

### Step 1.2.1 -- Add edge key to features field

**File**: `extensions/product/manifest.json`

Replace line 38:
```json
{ "name": "features", "description": "Related features referenced by this feature", "fieldType": "reference_list", "targetKind": "feature" },
```
With:
```json
{ "name": "features", "description": "Related features referenced by this feature", "fieldType": "reference_list", "edge": "FeatureRelatesTo", "targetKind": "feature" },
```

### Step 1.2.2 -- Add FeatureRelatesTo to edgeTypes array

**File**: `extensions/product/manifest.json`

Insert a new entry into the `edgeTypes` array. Place it after `FeatureDependsOn` (line 288) for logical grouping:

```json
{ "label": "FeatureRelatesTo", "description": "Feature has a non-dependency relationship to another feature", "sourceKind": "feature", "targetKind": "feature", "edgeStyle": "dotted", "edgeColor": "#2196F3" },
```

This brings product edgeTypes from 27 to 28.

### Step 1.2.3 -- Update product manifest tests

**File**: `crates/specforge-registry/tests/product_manifest.rs`

**Change 1**: `test_product_manifest_has_27_edge_types` -- rename and update count.

Replace:
```rust
fn test_product_manifest_has_27_edge_types() {
    let manifest = load_product_manifest();
    assert_eq!(manifest.edge_types.len(), 27);
```
With:
```rust
fn test_product_manifest_has_28_edge_types() {
    let manifest = load_product_manifest();
    assert_eq!(manifest.edge_types.len(), 28);
```

Add assertion:
```rust
assert!(labels.contains(&"FeatureRelatesTo"));
```

**Change 2**: `test_product_manifest_populates_registries` -- add edge assertion.

Add to the edge-type assertions block (around line 146):
```rust
assert!(edge_reg.contains("FeatureRelatesTo"));
```

### Step 1.2.4 -- Verify consistency validation still passes

The `test_product_manifest_passes_consistency_validation` test calls `validate_manifest_consistency()`. After this fix, the `features` field will have an `edge` key pointing to `FeatureRelatesTo`, and `FeatureRelatesTo` will be in `edgeTypes`. Both the field edge check and the edge source/target check should pass cleanly.

### Step 1.2.5 -- Verification

```bash
cargo test -p specforge-registry --test product_manifest
```

---

## Bug 1.3: Refinement edge ambiguity (`@specforge/formal`)

**File**: `extensions/formal/manifest.json` lines 106-107
**Root cause**: The `refinement` entity has `abstract_entity` and `concrete_entity` fields both using `"edge": "RefinesTo"`. Given a `RefinesTo` edge from refinement R to behavior B, you cannot tell whether B is the abstract source or the concrete target.

### Step 1.3.1 -- Split RefinesTo into RefinesAbstract and RefinesConcrete

**File**: `extensions/formal/manifest.json`

Replace lines 106-107:
```json
{ "name": "abstract_entity", "description": "Reference to the abstract behavior being refined", "fieldType": "reference", "edge": "RefinesTo", "targetKind": "behavior" },
{ "name": "concrete_entity", "description": "Reference to the concrete behavior that implements the refinement", "fieldType": "reference", "edge": "RefinesTo", "targetKind": "behavior" },
```
With:
```json
{ "name": "abstract_entity", "description": "Reference to the abstract behavior being refined", "fieldType": "reference", "edge": "RefinesAbstract", "targetKind": "behavior" },
{ "name": "concrete_entity", "description": "Reference to the concrete behavior that implements the refinement", "fieldType": "reference", "edge": "RefinesConcrete", "targetKind": "behavior" },
```

### Step 1.3.2 -- Replace RefinesTo edge type with two new entries

**File**: `extensions/formal/manifest.json`

Replace lines 192-199 (the single `RefinesTo` edge type):
```json
{
  "label": "RefinesTo",
  "description": "Refinement maps abstract to concrete behavior",
  "sourceKind": "refinement",
  "targetKind": "behavior",
  "edgeStyle": "solid",
  "edgeColor": "#1B5E20"
},
```
With two entries:
```json
{
  "label": "RefinesAbstract",
  "description": "Refinement maps from this abstract behavior",
  "sourceKind": "refinement",
  "targetKind": "behavior",
  "edgeStyle": "dashed",
  "edgeColor": "#1B5E20"
},
{
  "label": "RefinesConcrete",
  "description": "Refinement maps to this concrete behavior",
  "sourceKind": "refinement",
  "targetKind": "behavior",
  "edgeStyle": "solid",
  "edgeColor": "#1B5E20"
},
```

This changes formal edgeTypes from 11 to 12.

### Step 1.3.3 -- Update spec files referencing RefinesTo

The following `.spec` files reference `RefinesTo` by name in descriptions, comments, and verify statements. These are documentation/spec-level references that should be updated to reference both new edge names:

1. `spec/extensions/formal/manifest.spec` (lines 48, 87, 95, 125)
2. `spec/extensions/formal/features.spec` (lines 68, 218)
3. `spec/extensions/formal/behaviors.spec` (lines 62, 77)
4. `spec/extensions/formal/specification-layering.spec` (lines 134, 141, 150)
5. `spec/extensions/formal/validation-rules.spec` (lines 300, 311, 312, 319)
6. `spec/extensions/formal/invariants.spec` (lines 89, 95)
7. `spec/extensions/formal/decisions.spec` (line 145)

**Strategy**: Update `RefinesTo` -> `RefinesAbstract/RefinesConcrete` in edge type enumerations, and `RefinesTo` -> `RefinesAbstract` or `RefinesConcrete` based on context. In places where the spec just says "RefinesTo edges" generically, replace with "RefinesAbstract/RefinesConcrete edges."

**Note**: These are `.spec` files (human-readable specifications), not compiled code. They do not affect test correctness, but should be updated for accuracy. This can be done in a follow-up pass if time-constrained.

### Step 1.3.4 -- Verify no Rust tests reference RefinesTo

Grep confirms no `.rs` files reference `RefinesTo`. There are no formal manifest test files (`crates/specforge-registry/tests/*formal*` returned no results). No Rust test changes needed.

### Step 1.3.5 -- Update docs referencing RefinesTo

**Files**:
- `docs/entity-model.md`
- `docs/README.md`
- `.claude/skills/specforge-domain/SKILL.md`

Update edge type references from `RefinesTo` to `RefinesAbstract`/`RefinesConcrete`. These are documentation files and do not affect compilation or tests.

### Step 1.3.6 -- Verification

```bash
# Manifest deserializes and passes validation
cargo test -p specforge-registry -- manifest
# Full registry test suite
cargo test -p specforge-registry
```

---

## Bug 1.4: Decision supersedes bidirectional redundancy (`@specforge/governance`)

**File**: `extensions/governance/manifest.json` lines 31 and 40
**Root cause**: The `decision` entity has `superseded_by` (line 31) and `supersedes` (line 40), both using `"edge": "Supersedes"`. This creates redundant, ambiguous edges. One field + graph direction traversal provides full semantics.

### Step 1.4.1 -- Remove the `supersedes` field

**File**: `extensions/governance/manifest.json`

Remove line 40:
```json
{ "name": "supersedes", "description": "Reference to an older decision that this one replaces", "fieldType": "reference", "edge": "Supersedes", "targetKind": "decision" },
```

The `superseded_by` field (line 31) remains as the canonical direction: old decision -> new decision (meaning "this decision is superseded by X"). Traversing the `Supersedes` edge in reverse gives "X supersedes this decision."

The `Supersedes` edge type in `edgeTypes` (lines 117-123) remains unchanged.

### Step 1.4.2 -- Verify no Rust tests reference supersedes

Grep confirms no `.rs` files reference `supersedes` or `superseded_by`. There are no governance manifest test files. No test changes needed.

### Step 1.4.3 -- Verification

```bash
# Governance manifest still deserializes and validates
cargo test -p specforge-registry -- manifest
```

---

## Cross-cutting verification

After all 4 bugs are fixed, run the full test suite:

```bash
cargo test --workspace
cargo clippy --workspace
```

### Files modified (summary)

| File | Change |
|------|--------|
| `extensions/software/manifest.json` | entityEnhancements: split `UsesPort` into `ModuleConsumesPort`/`ModuleDefinesPort`; add 2 edge types; add descriptions |
| `extensions/product/manifest.json` | feature.features field: add `"edge": "FeatureRelatesTo"`; add 1 edge type |
| `extensions/formal/manifest.json` | refinement fields: split `RefinesTo` into `RefinesAbstract`/`RefinesConcrete`; replace 1 edge type with 2 |
| `extensions/governance/manifest.json` | decision: remove redundant `supersedes` field |
| `crates/specforge-registry/tests/software_manifest.rs` | edge count 11->13, add 2 edge label assertions |
| `crates/specforge-registry/tests/product_manifest.rs` | edge count 27->28, add 1 edge label assertion |

### Files NOT modified (and why)

| File | Reason |
|------|--------|
| `crates/specforge-registry/tests/zero_entity_registries.rs` | Uses synthetic manifest helper, not real JSON. No real-manifest counts to update. |
| `crates/specforge-registry/src/**/*.rs` | No compiler logic changes needed. Edge types are data-driven from manifests. |
| `.spec` files in `spec/extensions/formal/` | Documentation-only references. Can be updated in follow-up. |

---

## Risk assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Snapshot tests contain hardcoded edge counts or names | Medium | Test failures | Run `cargo insta review` after changes; search for `UsesPort`, `RefinesTo`, `Supersedes` in snapshot files |
| MCP/CLI tools reference old edge names | Low | Runtime errors | Grep for old edge names across `crates/specforge-mcp/`, `crates/specforge-cli/` |
| Graph export format tests embed edge labels | Medium | Test failures | Run `cargo test -p specforge-emitter` and review failures |
| `.spec` example files use old edge names in verify blocks | Low | Spec inaccuracy | Follow-up pass to update spec files (non-blocking) |
| `validate_manifest_consistency` rejects new cross-extension edges | Low | Test failures caught immediately | Both manifests have peer dependencies, so the consistency check skips cross-ext target_kind validation when `peer_deps` is non-empty |

---

## Progress

- [x] 1.1.1 Update software manifest entityEnhancements for module ports (split UsesPort -> ModuleConsumesPort/ModuleDefinesPort, add descriptions)
- [x] 1.1.2 Add ModuleConsumesPort/ModuleDefinesPort to software edgeTypes array
- [x] 1.1.3 Update software_manifest.rs tests (edge count 11->13, new label assertions)
- [x] 1.1.4 Verify zero_entity_registries.rs needs no changes
- [x] 1.1.5 Run software manifest tests green
- [x] 1.2.1 Add "edge": "FeatureRelatesTo" to product feature.features field
- [x] 1.2.2 Add FeatureRelatesTo to product edgeTypes array
- [x] 1.2.3 Update product_manifest.rs tests (edge count 27->28, new label assertion)
- [x] 1.2.4 Verify consistency validation still passes
- [x] 1.2.5 Run product manifest tests green
- [x] 1.3.1 Split refinement fields RefinesTo -> RefinesAbstract/RefinesConcrete
- [x] 1.3.2 Replace RefinesTo edge type with RefinesAbstract + RefinesConcrete in edgeTypes
- [ ] 1.3.3 Update .spec files referencing RefinesTo (follow-up OK)
- [x] 1.3.4 Verify no Rust tests reference RefinesTo
- [ ] 1.3.5 Update docs referencing RefinesTo (follow-up OK)
- [x] 1.3.6 Run formal-related tests green
- [x] 1.4.1 Remove supersedes field from governance decision entity
- [x] 1.4.2 Verify no Rust tests reference supersedes
- [x] 1.4.3 Run governance-related tests green
- [x] Final: cargo test --workspace passes (0 failed, ~1950 tests)
- [ ] Final: cargo clippy --workspace passes
