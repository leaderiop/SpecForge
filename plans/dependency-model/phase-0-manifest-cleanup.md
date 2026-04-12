# Phase 0: Manifest Cleanup — Strict One-Way Dependencies

**Goal**: Move cross-extension fields/edges from product to software's entityEnhancements. Remove all reverse and redundant peer_dependencies. Product becomes a true standalone root.

**Status**: PENDING

## Tracking

| # | Item | Status |
|---|------|--------|
| 0.1 | Remove `ports_defined`, `ports_consumed` fields from product's `library` entityKind | PENDING |
| 0.2 | Remove `behaviors` field from product's `roadmap` entityKind | PENDING |
| 0.3 | Remove `LibraryDefinesPort`, `LibraryConsumesPort`, `RoadmapPlansBehavior` from product's edgeTypes | PENDING |
| 0.4 | Set product's `peerDependencies` to `[]` | PENDING |
| 0.5 | Add `library` and `roadmap` entityEnhancements to software's manifest | PENDING |
| 0.6 | Add 3 edge types (`LibraryDefinesPort`, `LibraryConsumesPort`, `RoadmapPlansBehavior`) to software's edgeTypes | PENDING |
| 0.7 | Remove governance → product optional dep (transitive via software) | PENDING |
| 0.8 | Verify product manifest passes consistency validation (zero W021) | PENDING |
| 0.9 | Verify `cargo test --workspace` — zero regressions | PENDING |

## Details

### 0.1–0.2: Remove cross-extension fields from product

**File**: `extensions/product/manifest.json`

Remove from `library` entityKind fields array (lines ~261-262):
```json
{ "name": "ports_defined", "description": "Ports (interfaces) defined by this library", "fieldType": "reference_list", "edge": "LibraryDefinesPort", "targetKind": "port" },
{ "name": "ports_consumed", "description": "Ports (interfaces) consumed by this library", "fieldType": "reference_list", "edge": "LibraryConsumesPort", "targetKind": "port" }
```

Remove from `roadmap` entityKind fields array (line ~279):
```json
{ "name": "behaviors", "description": "Behaviors planned in this roadmap", "fieldType": "reference_list", "edge": "RoadmapPlansBehavior", "targetKind": "behavior" }
```

These fields reference software's `port` and `behavior` kinds. Product (the root) should not reference kinds from software (its dependent). The dependency arrow goes software → product, not the other way.

### 0.3: Remove cross-extension edge types from product

**File**: `extensions/product/manifest.json`

Remove from `edgeTypes` array (lines ~314-316):
```json
{ "label": "LibraryDefinesPort", "sourceKind": "library", "targetKind": "port", "edgeStyle": "solid", "edgeColor": "#795548" },
{ "label": "LibraryConsumesPort", "sourceKind": "library", "targetKind": "port", "edgeStyle": "dashed", "edgeColor": "#795548" },
{ "label": "RoadmapPlansBehavior", "sourceKind": "roadmap", "targetKind": "behavior", "edgeStyle": "solid", "edgeColor": "#3F51B5" }
```

These edges cross from product-owned kinds (library, roadmap) to software-owned kinds (port, behavior). They belong in the extension that knows about both sides — software.

### 0.4: Product standalone root

**File**: `extensions/product/manifest.json`

Change:
```json
"peerDependencies": [
  { "name": "@specforge/software", "version": "^1.0", "optional": true }
]
```
To:
```json
"peerDependencies": []
```

Product is the root of the dependency DAG. It has zero knowledge of any other extension.

### 0.5: Add entityEnhancements to software

**File**: `extensions/software/manifest.json`

Add to existing `entityEnhancements` array (after the milestone enhancement, ~line 270):
```json
{
  "targetKind": "library",
  "sourceExtension": "@specforge/product",
  "fields": [
    { "name": "ports_defined", "description": "Ports (interfaces) defined by this library", "fieldType": "reference_list", "edge": "LibraryDefinesPort", "targetKind": "port" },
    { "name": "ports_consumed", "description": "Ports (interfaces) consumed by this library", "fieldType": "reference_list", "edge": "LibraryConsumesPort", "targetKind": "port" }
  ]
},
{
  "targetKind": "roadmap",
  "sourceExtension": "@specforge/product",
  "fields": [
    { "name": "behaviors", "description": "Behaviors planned in this roadmap", "fieldType": "reference_list", "edge": "RoadmapPlansBehavior", "targetKind": "behavior" }
  ]
}
```

This follows the existing pattern: software already enhances product's `module` (ports, ports_defined) and `milestone` (behaviors). Now it also enhances `library` and `roadmap`.

### 0.6: Add edge types to software

**File**: `extensions/software/manifest.json`

Add to `edgeTypes` array (after existing edges):
```json
{ "label": "LibraryDefinesPort", "description": "Library defines a port interface (enhancement on @specforge/product library)", "sourceKind": "library", "targetKind": "port", "edgeStyle": "solid", "edgeColor": "#795548" },
{ "label": "LibraryConsumesPort", "description": "Library consumes a port interface (enhancement on @specforge/product library)", "sourceKind": "library", "targetKind": "port", "edgeStyle": "dashed", "edgeColor": "#795548" },
{ "label": "RoadmapPlansBehavior", "description": "Roadmap plans a behavior (enhancement on @specforge/product roadmap)", "sourceKind": "roadmap", "targetKind": "behavior", "edgeStyle": "solid", "edgeColor": "#3F51B5" }
```

### 0.7: Remove governance → product optional dep

**File**: `extensions/governance/manifest.json`

Change:
```json
"peerDependencies": [
  { "name": "@specforge/software", "version": "^1.0" },
  { "name": "@specforge/product", "version": "^1.0", "optional": true }
]
```
To:
```json
"peerDependencies": [
  { "name": "@specforge/software", "version": "^1.0" }
]
```

Governance → product is transitive (via governance → software → product). No explicit declaration needed.

### 0.8–0.9: Verification

Run manifest consistency validation: product should have zero W021 warnings (no more cross-extension kind references). Governance should also pass (still has software as peer dep, so W021 suppression works for its cross-extension edge references to product's `feature` kind — those are resolved transitively).

Run `cargo test --workspace` to catch any regressions from the manifest changes.
