# Phase 7b: Targeted Fixes for Sub-8 Axes

**Status**: COMPLETE
**Depends on**: Phase 7 initial scoring
**Trigger**: 5 axes scored 7 (axes 2, 4, 8, 14, 17) — failure protocol Section 6.2

---

## Root Cause Analysis

| Axis | Score | Root Causes |
|------|-------|-------------|
| 2 - Relationship Correctness | 7 | UsesType overloaded (3 meanings), MaintainsCondition missing prefix |
| 4 - Naming Consistency | 7 | `fieldType` camelCase on type entity, MaintainsCondition inconsistent |
| 8 - Redundancy | 7 | module/library overlap, capability/journey overlap (structural) |
| 14 - Governance | 7 | Missing decision→constraint edge, missing failure_mode→behavior edge |
| 17 - Learnability | 7 | 26 kinds/65 edges large surface area (structural) |

## Fixes Applied

### 7b.1: Split UsesType into 3 specific edges
**Files**: `extensions/software/manifest.json`

- Removed generic `UsesType` edge type (no source/target kinds)
- Added `BehaviorReferencesType` (behavior→type) — "references type in contract"
- Added `EventPayloadType` (event→type) — "carries type as payload"
- Added `TypeComposesType` (type→type) — "composes another type"
- Updated behavior.types field: edge → `BehaviorReferencesType`
- Updated event.payload field: edge → `EventPayloadType`
- Updated type.composed_types field: edge → `TypeComposesType`
- Net: +2 edge types (13→15)

### 7b.2: Rename fieldType to composed_types
**Files**: `extensions/software/manifest.json`

- Renamed type entity field `fieldType` → `composed_types` (fixes camelCase)
- Updated description: "Types composed or referenced by this type"

### 7b.3: Rename MaintainsCondition to BehaviorMaintainsCondition
**Files**: `extensions/formal/manifest.json`

- Renamed edge type label for consistency with BehaviorRequiresCondition/BehaviorEnsuresCondition
- Updated entityEnhancement field reference
- Edge count unchanged (12)

### 7b.4: Add governance traceability edges
**Files**: `extensions/governance/manifest.json`

- Added `constraints` field to decision entity (reference_list → constraint)
- Added `affected_behaviors` field to failure_mode entity (reference_list → behavior)
- Added `DecisionImposesConstraint` edge (decision→constraint)
- Added `FailureModeAffectsBehavior` edge (failure_mode→behavior)
- Net: +2 edge types (9→11)

## Impact Summary

- software: 5 entities, 15 edges (was 13)
- product: 12 entities, 31 edges (unchanged)
- governance: 3 entities, 11 edges (was 9)
- formal: 6 entities, 12 edges (unchanged)
- **Total: 26 entity kinds, 69 edge types** (was 65)

## Phase 7c: Additional Fixes After Re-Score Round 2

Round 2 scoring found: avg 8.2, axes 2 and 17 still at 7.

### 7c.1: Fix 4 phantom edges (backing fields added)
- `type.extends` reference(type) → TypeExtendsType
- `process.sub_processes` reference_list(process) → ProcessComposesProcess
- `refinement.chains_to` reference(refinement) → RefinementChainsToRefinement
- `axiom.assumes` reference_list(condition) → AxiomAssumesCondition

### 7c.2: Rename ConditionAssumedByAxiom → AxiomAssumesCondition
- Reversed direction: axiom→condition (active voice from source)

### 7c.3: Rename EventPayloadType → EventCarriesPayloadType
- Noun compound → verb phrase

### 7c.4: Rename TermSeeAlso → TermReferencesRelatedTerm
- Non-verb-phrase → proper SourceVerbTarget pattern

### 7c.5: Rename decision.decision → decision.statement
- Eliminates entity-field name collision

### 7c.6: Add description to milestone.behaviors
- Was the only non-id field missing a description

## Tests

- All workspace tests pass (0 failures)
- Snapshots auto-updated
