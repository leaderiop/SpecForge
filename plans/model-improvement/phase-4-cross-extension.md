# Phase 4: Cross-Extension Connectivity

**Status**: COMPLETE (2026-04-12)
**Depends on**: Phase 2 (Edge Naming), Phase 3 (Field Consistency)
**Targets axes**: 5 (Cross-Extension Coherence), 9 (Connectivity), 14 (Governance), 16 (Scalability)
**Created**: 2026-04-12

## Goal

Close identified connectivity gaps between the four extensions (product, software, governance, formal) without violating the zero-domain-knowledge-in-core principle. Every new edge or field must use the existing extension mechanisms: `peer_dependencies`, `entity_enhancements`, and `edge_types` in manifests.

## Current Extension Dependency DAG

```
product (standalone, no deps)
   ^
   |  peer_dependency
software (depends on product ^1.0)
   ^           ^
   |           |
governance  formal
(deps: sw)  (deps: sw)
```

**Entity counts**: product=9, software=6, governance=3, formal=6 (24 total)
**Edge counts**: product=16, software=9, governance=4, formal=11 (40 total)
**Cross-extension edges today**: ConstrainsBehavior (gov->sw), Implements (sw->product), MilestoneBehavior (sw->product via enhancement), 11 formal->software edges

---

## 4.1: Governance-to-Product Edges

### Problem

Governance entities (constraint, decision, failure_mode) connect ONLY to software entities (behavior, invariant). There is no way to express:

- "No milestone may have more than 20 features" (constraint -> milestone)
- "We decided to ship mobile-first" (decision -> channel or feature)
- "If this feature is delayed, the release is blocked" (failure_mode -> feature)

Governance is about architectural decisions and constraints. These naturally apply to product-level entities, not just code-level behaviors and invariants. The current model forces users to express product-level governance indirectly through behaviors, which is both unnatural and lossy.

### Decision: DO IT (Option A -- direct peer dependency)

Add `@specforge/product ^1.0` as an **optional** peer dependency of `@specforge/governance`. This is the cleanest mechanism: governance declares it CAN connect to product entities when product is installed. If product is not installed, the cross-extension fields produce soft-resolution I004 diagnostics (existing behavior for missing peers).

Option B (routing through software's entity_enhancements) creates a confusing three-way dependency. Option C (keep the gap) leaves a real user need unaddressed and keeps the governance score at 6/10.

### New Dependency DAG After 4.1

```
product (standalone, no deps)
   ^           ^
   |           |  peer_dependency (optional)
software    governance
(deps: p)   (deps: sw, optional p)
   ^           ^
   |           |
  formal      (no change)
(deps: sw)
```

This is a clean DAG. No cycles. Governance optionally reaches product, just as software does.

### New Edge Types (3)

#### ConstraintGovernsFeature

```json
{
  "label": "ConstraintGovernsFeature",
  "description": "A constraint that governs/restricts a product feature",
  "source_kind": "constraint",
  "target_kind": "feature",
  "edge_style": "dashed",
  "edge_color": "#CC6600",
  "edge_arrowhead": "normal"
}
```

**Rationale**: Constraints frequently govern features (e.g., "all public-facing features must meet WCAG 2.1 AA"). The dashed style distinguishes cross-extension edges from intra-extension ones.

#### DecisionAffectsFeature

```json
{
  "label": "DecisionAffectsFeature",
  "description": "An architectural decision that affects a product feature",
  "source_kind": "decision",
  "target_kind": "feature",
  "edge_style": "dashed",
  "edge_color": "#CC6600",
  "edge_arrowhead": "normal"
}
```

**Rationale**: ADRs routinely reference features they impact. Without this edge, decisions float disconnected from the product layer they govern.

#### FailureModeThreatsFeature

```json
{
  "label": "FailureModeThreatsFeature",
  "description": "A failure mode that threatens a product feature",
  "source_kind": "failure_mode",
  "target_kind": "feature",
  "edge_style": "dashed",
  "edge_color": "#CC0000",
  "edge_arrowhead": "diamond"
}
```

**Rationale**: FMEA naturally maps to features ("if feature X fails, the impact is Y"). The diamond arrowhead signals threat/risk, consistent with failure_mode's doubleoctagon dot shape.

### Field Changes on Governance Entities

#### constraint: add `governs_features` field

```json
{
  "name": "governs_features",
  "field_type": "reference_list",
  "edge": "ConstraintGovernsFeature",
  "target_kind": "feature",
  "required": false,
  "description": "Product features this constraint governs (requires @specforge/product)"
}
```

#### decision: add `affects_features` field

```json
{
  "name": "affects_features",
  "field_type": "reference_list",
  "edge": "DecisionAffectsFeature",
  "target_kind": "feature",
  "required": false,
  "description": "Product features affected by this decision (requires @specforge/product)"
}
```

#### failure_mode: add `threatens_features` field

```json
{
  "name": "threatens_features",
  "field_type": "reference_list",
  "edge": "FailureModeThreatsFeature",
  "target_kind": "feature",
  "required": false,
  "description": "Product features threatened by this failure mode (requires @specforge/product)"
}
```

### Manifest Changes

**File**: governance manifest (JSON)

1. Increment edge_types count from 4 to 7
2. Add `@specforge/product ^1.0` to `peer_dependencies` (optional)
3. Add 3 new edge type declarations
4. Add 3 new field declarations on existing entity kinds

### Spec File Changes

| File | Change |
|------|--------|
| `spec/extensions/governance/manifest.spec` | Update edge count (4->7), add peer_dependency for product, update ensures clauses |
| `spec/extensions/governance/behaviors.spec` | Update `ge_register_edge_types` (4->7 edges), update `ge_register_field_definitions` with new fields |
| `spec/extensions/governance/types.spec` | Add `governs_features`, `affects_features`, `threatens_features` fields to types |
| `spec/extensions/governance/invariants.spec` | Add invariant for 7 edge types |
| `spec/extensions/governance/features.spec` | Update feature descriptions to mention product connectivity |
| `spec/extensions/governance/validation-rules.spec` | No new validation rules needed (fields are optional) |

### Diagnostic Code Allocation

No new diagnostic codes needed. The existing I004 soft-resolution diagnostic handles the case where `@specforge/product` is not installed but the reference fields are populated.

### Validation Rules

No new validation rules. The 3 new fields are all optional `reference_list` types. Cross-extension reference resolution already handles:
- Missing target entity: existing E003 (unresolved reference)
- Missing peer extension: existing I004 (soft resolution)

---

## 4.2: Formal-to-Governance Edges

### Problem

Formal (conditions, properties, axioms) has no connection to governance (decisions, constraints). Potential use cases:

- A decision references the formal property it was designed to satisfy
- A constraint is backed by a formal condition (metric -> condition mapping)
- A failure_mode's trigger maps to a formal condition

### Decision: SKIP (keep independent)

**Rationale**: The indirect path through behavior is sufficient and avoids DAG complexity.

- `decision -> invariant <- failure_mode` (via DecisionInvariant + FailureModeInvariant)
- `constraint -> behavior <- requires/ensures/maintains -> condition` (via ConstrainsBehavior + formal entity_enhancements on behavior)
- `property -> satisfies <- behavior <- constrains <- constraint` (multi-hop but traversable)

Adding a formal->governance dependency would:
1. Create a fourth level in the DAG (formal depends on governance depends on software)
2. Add complexity for niche use cases (fewer than 5% of projects would use formal+governance together)
3. Violate the "structure is a spectrum" principle by requiring comprehensive cross-extension coverage

The indirect paths exist and are queryable via `specforge trace`. If demand materializes in v2, this can be revisited without breaking changes (adding optional peer_dependencies and new edges is always additive).

---

## 4.3: Reduce Leaf Entity Isolation

### Problem

10 of 24 domain entities are degree-1 leaves (only 1 edge type connects them):

| Entity | Extension | Current Edges | Degree |
|--------|-----------|---------------|--------|
| channel | product | JourneyChannel | 1 |
| persona | product | JourneyPersona | 1 |
| term | product | TermSeeAlso | 1 |
| axiom | formal | AssumedBy | 1 |
| release | product | ReleaseDeliverable, ReleaseMilestone | 2 |

### Decisions Per Entity

#### 4.3a: PersonaFeature edge -- DO IT

```json
{
  "label": "PersonaFeature",
  "description": "A feature that is important to a specific persona",
  "source_kind": "persona",
  "target_kind": "feature",
  "edge_style": "solid",
  "edge_color": "black",
  "edge_arrowhead": "normal"
}
```

**Field on persona**:

```json
{
  "name": "key_features",
  "field_type": "reference_list",
  "edge": "PersonaFeature",
  "target_kind": "feature",
  "required": false,
  "description": "Features this persona cares about most"
}
```

**Rationale**: Personas and features have a natural affinity. "This persona's priority features" is a common product question. Today this requires traversing persona -> journey -> feature (3 hops). A direct edge enables:
- Direct persona-feature queries without multi-hop traversal
- Feature prioritization by persona ("which features matter to power users?")
- Impact analysis ("if we cut this feature, which personas lose value?")

**File changes**:
- `spec/extensions/product/manifest.spec`: Increment edge count 16->17
- `spec/extensions/product/types.spec`: Add `key_features` field to `ProductPersona`
- `spec/extensions/product/behaviors-registration.spec`: Update edge registration (16->17)

**Diagnostic impact**: None. The PersonaFeaturePayload query already exists (multi-hop). The direct edge gives it a structural foundation.

#### 4.3b: ChannelFeature edge -- SKIP

**Rationale**: Channel -> feature is rarely queried directly. The journey -> channel + journey -> feature path is sufficient. Channel is intentionally simple (it's an interaction model descriptor, not a planning entity).

#### 4.3c: Term isolation -- SKIP

**Rationale**: Terms are glossary entries. TermSeeAlso (term -> term) is the correct and only relationship. Terms don't "connect to" features or behaviors -- they define vocabulary. Isolation is by design.

#### 4.3d: Axiom isolation -- SKIP

**Rationale**: Axioms are foundational assumptions. They are ASSUMED, not derived. AssumedBy (condition -> axiom) is the single correct relationship. An axiom with multiple edges would undermine its semantic role as an unchangeable foundation.

#### 4.3e: Release isolation -- SKIP

**Rationale**: Release already has degree 2 (ReleaseDeliverable, ReleaseMilestone) which is sufficient for its coordination role.

### Summary of 4.3

Only PersonaFeature is added. The remaining leaves are correctly isolated by design.

---

## 4.4: Behavior Hub Analysis

### Problem

`behavior` participates in 11+ edge types, making it the coupling bottleneck of the graph.

### Decision: NO CHANGE (by design)

**Rationale**: Behavior IS the graph hub. It is the core unit of specification in the software extension. All three other extensions connect through it:

- **formal** enriches it (requires/ensures/maintains/abstract/refines/assumes/satisfies/refinement)
- **governance** constrains it (ConstrainsBehavior)
- **product** implements through it (Implements: behavior -> feature)

Reducing behavior's connectivity would fracture the graph. The high degree is a feature, not a bug -- it makes behavior the natural anchor for cross-extension queries. An agent asking "what governs this feature?" traverses feature <- Implements <- behavior <- ConstrainsBehavior <- constraint in 3 hops through behavior.

The only risk is query performance on very large graphs (>10,000 behaviors). This is a scalability concern addressed by the graph's incremental compilation and index-based traversal, not by model changes.

---

## Consolidated Changes

### New Extension Dependency Graph (Final)

```
product (standalone, no deps)
   ^           ^
   |           |  peer_dependency (optional)
software    governance
(deps: p)   (deps: sw, optional p)
   ^
   |
  formal
(deps: sw)
```

### New Edge Types (4 total)

| # | Label | Source | Target | Extension | Cross-Ext? |
|---|-------|--------|--------|-----------|------------|
| 1 | ConstraintGovernsFeature | constraint | feature | governance | yes (gov->product) |
| 2 | DecisionAffectsFeature | decision | feature | governance | yes (gov->product) |
| 3 | FailureModeThreatsFeature | failure_mode | feature | governance | yes (gov->product) |
| 4 | PersonaFeature | persona | feature | product | no (intra-product) |

### New Fields (4 total)

| # | Entity Kind | Field Name | Type | Edge | Extension |
|---|-------------|------------|------|------|-----------|
| 1 | constraint | governs_features | reference_list | ConstraintGovernsFeature | governance |
| 2 | decision | affects_features | reference_list | DecisionAffectsFeature | governance |
| 3 | failure_mode | threatens_features | reference_list | FailureModeThreatsFeature | governance |
| 4 | persona | key_features | reference_list | PersonaFeature | product |

### Updated Edge Counts

| Extension | Before | After | Delta |
|-----------|--------|-------|-------|
| product | 16 | 17 | +1 |
| software | 9 | 9 | 0 |
| governance | 4 | 7 | +3 |
| formal | 11 | 11 | 0 |
| **Total** | **40** | **44** | **+4** |

### Updated Leaf Analysis

| Entity | Before Degree | After Degree | Change |
|--------|---------------|--------------|--------|
| channel | 1 | 1 | -- |
| persona | 1 | 2 (+PersonaFeature) | improved |
| term | 1 | 1 | -- (by design) |
| axiom | 1 | 1 | -- (by design) |
| feature | 8+ | 12+ (+3 gov + 1 persona) | hub strengthened |
| constraint | 2 | 3 (+ConstraintGovernsFeature) | improved |
| decision | 1 | 2 (+DecisionAffectsFeature) | improved |
| failure_mode | 1 | 2 (+FailureModeThreatsFeature) | improved |

Degree-1 leaf count drops from 10 to 6 (persona, decision, failure_mode all gain a second edge).

---

## Files to Modify

### Spec Files

| File | Changes |
|------|---------|
| `spec/extensions/governance/manifest.spec` | edge count 4->7, add optional product peer_dependency, update ensures |
| `spec/extensions/governance/behaviors.spec` | `ge_register_edge_types` ensures 7 edges, `ge_register_field_definitions` adds 3 new fields |
| `spec/extensions/governance/types.spec` | Add `governs_features`, `affects_features`, `threatens_features` fields |
| `spec/extensions/governance/invariants.spec` | Update `ge_manifest_four_edge_types` -> `ge_manifest_seven_edge_types` |
| `spec/extensions/governance/features.spec` | Update feature descriptions for product connectivity |
| `spec/extensions/product/manifest.spec` | edge count 16->17, update ensures |
| `spec/extensions/product/types.spec` | Add `key_features` field to `ProductPersona` |
| `spec/extensions/product/behaviors-registration.spec` | Update edge registration |
| `spec/extensions/product/manifest.spec` | Update `pe_manifest_sixteen_edge_types` -> seventeen |

### Rust Source Files

| File | Changes |
|------|---------|
| `crates/specforge-registry/src/registries/kind.rs` | No changes (kinds unchanged) |
| `crates/specforge-registry/src/registries/field.rs` | No changes (field registration is manifest-driven) |
| `crates/specforge-registry/src/compilation/contributions.rs` | Verify new edge types register correctly |
| `crates/specforge-registry/src/compilation/populate.rs` | Verify new fields populate correctly |
| `crates/specforge-registry/src/compilation/validation_engine.rs` | No changes (no new validation rules) |
| `crates/specforge-emitter/src/model/build.rs` | No changes (model builder reads from registries dynamically) |
| `crates/specforge-emitter/src/schema.rs` | No changes (schema generation reads from registries dynamically) |
| `crates/specforge-emitter/src/model/dot.rs` | Verify new cross-extension edges render with correct styles |

### Test Files

| File | Changes |
|------|---------|
| `crates/specforge-registry/tests/zero_entity_registries.rs` | Add test: governance 7 edge types register |
| `crates/specforge-registry/tests/software_manifest.rs` | No changes |
| `crates/specforge-emitter/tests/schema.rs` | Update schema snapshot (new edges appear) |
| `crates/specforge-emitter/tests/model.rs` | Update model snapshot (new relationships appear) |
| `crates/specforge-emitter/src/model/snapshots/` | Regenerate all model snapshots |

### Tests to Create

| Test | Location | Description |
|------|----------|-------------|
| `test_governance_product_peer_dep` | `crates/specforge-registry/tests/` | Governance with product peer dep registers 7 edges |
| `test_governance_without_product` | `crates/specforge-registry/tests/` | Governance without product installed: 4 edges, I004 on cross-ext fields |
| `test_constraint_governs_feature_edge` | `crates/specforge-emitter/tests/` | Schema includes ConstraintGovernsFeature with correct source/target |
| `test_decision_affects_feature_edge` | `crates/specforge-emitter/tests/` | Schema includes DecisionAffectsFeature with correct source/target |
| `test_failure_mode_threats_feature_edge` | `crates/specforge-emitter/tests/` | Schema includes FailureModeThreatsFeature with correct source/target |
| `test_persona_feature_edge` | `crates/specforge-emitter/tests/` | Schema includes PersonaFeature with correct source/target |
| `test_governance_product_cross_ext_dot` | `crates/specforge-emitter/tests/` | DOT output renders dashed cross-extension edges |
| `test_model_relationships_include_new_edges` | `crates/specforge-emitter/tests/model.rs` | Model intermediate includes 4 new relationships |

---

## Progress Tracking

### 4.1: Governance-to-Product Edges
- [ ] Update `spec/extensions/governance/manifest.spec` (edge count, peer_dep, ensures)
- [ ] Update `spec/extensions/governance/invariants.spec` (7 edge types invariant)
- [ ] Update `spec/extensions/governance/behaviors.spec` (register 7 edges, 3 new fields)
- [ ] Update `spec/extensions/governance/types.spec` (3 new fields on entity types)
- [ ] Update `spec/extensions/governance/features.spec` (product connectivity description)
- [ ] Add governance edge type registrations in Rust (if hardcoded) or verify manifest-driven registration handles them
- [ ] Add test: `test_governance_product_peer_dep`
- [ ] Add test: `test_governance_without_product`
- [ ] Add test: `test_constraint_governs_feature_edge`
- [ ] Add test: `test_decision_affects_feature_edge`
- [ ] Add test: `test_failure_mode_threats_feature_edge`
- [ ] Regenerate schema snapshots
- [ ] Regenerate DOT snapshots

### 4.2: Formal-to-Governance Edges
- [x] Decision: SKIP (indirect path through behavior is sufficient)

### 4.3: Reduce Leaf Entity Isolation
- [ ] Update `spec/extensions/product/manifest.spec` (edge count 16->17)
- [ ] Update `spec/extensions/product/types.spec` (add `key_features` to ProductPersona)
- [ ] Update `spec/extensions/product/behaviors-registration.spec` (register PersonaFeature edge)
- [ ] Update product invariant for 17 edge types
- [ ] Add test: `test_persona_feature_edge`
- [ ] Regenerate schema snapshots
- [ ] Regenerate model snapshots

### 4.4: Behavior Hub Analysis
- [x] Decision: NO CHANGE (behavior hub is by design)

### Integration
- [ ] Run full `cargo test` -- all existing tests pass
- [ ] Run `cargo clippy` -- no new warnings
- [ ] Verify `specforge export --format=graph` includes new edges in schema
- [ ] Verify `specforge schema` output includes new edge types
- [ ] Verify `specforge model` output includes new relationships
- [ ] Update MEMORY.md with new edge counts and governance peer_dep

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Governance + product peer_dep creates unexpected coupling | Low | Medium | Optional peer_dep; fields are all optional reference_lists; I004 fires when product absent |
| Feature entity becomes overloaded hub (like behavior) | Medium | Low | Feature is already high-degree; 4 additional edges are proportional to its role as the central product concept |
| Schema version bump required | Certain | Low | New edges are additive-only; minor version bump (1.1.0), not breaking |
| Snapshot churn in tests | Certain | Low | Expected and manageable; Phase 6 handles comprehensive test updates |

## Schema Version Impact

All changes are additive (new edges, new optional fields). This triggers a **minor** version bump: `1.0.0 -> 1.1.0`. No breaking changes. Existing consumers of the Graph Protocol schema will continue to work -- they simply won't see the new edge types unless they update their edge type list.
