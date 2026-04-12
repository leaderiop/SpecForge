# Model Improvement Master Plan

**Goal**: Raise all 20 expert scores from avg 6.6/10 to 9+/10
**Created**: 2026-04-12
**Status**: COMPLETE — avg 8.6/10, all axes >= 8 (target: 9.0 avg, no axis < 8)

## Final Scorecard

| # | Axis | Before | After | Delta |
|---|------|--------|-------|-------|
| 1 | Domain Completeness | 6 | **8** | +2 |
| 2 | Relationship Correctness | 6 | **9** | +3 |
| 3 | Field Completeness | 6 | **8** | +2 |
| 4 | Naming Consistency | 6 | **9** | +3 |
| 5 | Cross-Extension Coherence | 7 | **9** | +2 |
| 6 | AI Agent Usability | 7 | **9** | +2 |
| 7 | Normalization | 7 | **8** | +1 |
| 8 | Redundancy | 7 | **8** | +1 |
| 9 | Connectivity | 6 | **9** | +3 |
| 10 | Cardinality | 7 | **9** | +2 |
| 11 | DDD Alignment | 5 | **8** | +3 |
| 12 | Product Management | 8 | **9** | +1 |
| 13 | Software Engineering | 8 | **8** | 0 |
| 14 | Governance | 6 | **9** | +3 |
| 15 | Formal Methods | 8 | **8** | 0 |
| 16 | Scalability | 6 | **9** | +3 |
| 17 | Learnability | 5 | **8** | +3 |
| 18 | Edge Label Quality | 6 | **9** | +3 |
| 19 | Description Quality | 7 | **9** | +2 |
| 20 | Extension Boundary | 8 | **9** | +1 |
| | **Average** | **6.6** | **8.6** | **+2.0** |

**Result**: 12 axes at 9, 8 axes at 8, 0 axes below 8. The 8 axes at 8 have structural remaining issues (aggregate roots, stakeholder entity, module/library overlap, string-typed enums) that are v1.1 scope.

## Phases

| Phase | Name | Checkboxes | Changes | Status | Plan |
|-------|------|-----------|---------|--------|------|
| 1 | P0 Manifest Bugs | 21 items | 4 bugs fixed, +3 edge types, -2 redundant fields | COMPLETE | [phase-1](phase-1-manifest-bugs.md) |
| 2 | Edge Naming Standardization | 28 items | 36 edge renames across 4 manifests, 55 test updates | COMPLETE | [phase-2](phase-2-edge-naming.md) |
| 3 | Field Type Consistency | 18 items | 2 field promotions, +2 edge types, +12 validation rules | COMPLETE | [phase-3](phase-3-field-consistency.md) |
| 4 | Cross-Extension Connectivity | 16 items | +4 new edge types, +1 peer dependency | COMPLETE | [phase-4](phase-4-cross-extension.md) |
| 5 | Model Builder Accuracy | 19 items | Fix edge ownership accounting, count accuracy | COMPLETE | [phase-5](phase-5-model-accuracy.md) |
| 6 | Test & Snapshot Updates | 37 items | All test assertions + snapshots updated | COMPLETE | [phase-6](phase-6-tests.md) |
| 7 | Re-Score Verification | 39 items | 20-expert re-evaluation, 3 scoring rounds | COMPLETE | [phase-7](phase-7-rescore.md) |
| 7b | Targeted Fixes (Round 1) | 4 items | Split UsesType, rename MaintainsCondition, +2 governance edges | COMPLETE | [phase-7b](phase-7b-targeted-fixes.md) |
| 7c | Targeted Fixes (Round 2) | 6 items | Fix 4 phantom edges, rename 3 edges, fix descriptions | COMPLETE | [phase-7b](phase-7b-targeted-fixes.md) |

**Total tracking items**: 188 checkboxes across 8 phases

## Dependency Graph

```
Phase 1 (P0 Bugs: 4 manifest fixes)
  |
  v
Phase 2 (Edge Naming: 28 renames) ----+
  |                                    |
  v                                    v
Phase 3 (Fields: type promotions) -> Phase 4 (Cross-Ext: +4 edges)
  |                                    |
  +----------------+-------------------+
                   |
                   v
             Phase 5 (Model Builder: accuracy fixes)
                   |
                   v
             Phase 6 (Tests: 37 updates + snapshots)
                   |
                   v
             Phase 7 (Re-Score: 20 experts)
```

## Summary of All Changes

### Phase 1 -- P0 Manifest Bugs (16KB plan)
- **1.1**: Split `UsesPort` on module into `ModuleConsumesPort` + `ModuleDefinesPort`
- **1.2**: Add `FeatureRelatesTo` edge to `feature.features` field
- **1.3**: Split `RefinesTo` into `RefinesAbstract` + `RefinesConcrete`
- **1.4**: Remove redundant `decision.supersedes` field (keep `superseded_by`)

### Phase 2 -- Edge Naming (27KB plan)
- Hybrid 3-tier convention: Tier 1 (self-evident verbs, keep), Tier 2 (SourceVerbTarget), Tier 3 (self-referential DependsOn)
- 28 renames: 16 product, 4 software, 3 governance, 5 formal
- HIGH risk -- ripples to tests, validation rules, snapshots

### Phase 3 -- Field Type Consistency (22KB plan)
- **3.1**: `capability.persona` string -> reference + `CapabilityPersona` edge
- **3.2**: `capability.surface` string_list -> reference_list `channels` + `CapabilityUsesChannel` edge
- **3.3-3.8**: Add enum validation rules for condition.kind, property.property_type, type.kind, decision.status, failure_mode severity/occurrence/detection, roadmap.status

### Phase 4 -- Cross-Extension Connectivity (21KB plan)
- **4.1 (DO)**: Governance->product: 3 new edges (`DecisionAffectsFeature`, `ConstraintGovernsFeature`, `FailureModeThreatsFeature`)
- **4.2 (SKIP)**: Formal->governance: indirect path through behavior is sufficient
- **4.3 (DO)**: Add `PersonaFeature` edge (persona->feature) to reduce leaf isolation
- **4.4 (SKIP)**: Behavior hub is by design, do not reduce

### Phase 5 -- Model Builder Accuracy (24KB plan)
- **5.1**: Fix edge ownership accounting (enhancement edges counted against wrong extension)
- **5.2**: Fix total edge count (deduplication + footer calculation)
- **5.3**: Verify entity counts match manifests
- **5.4**: Verify enhancement field attribution
- **5.5**: Recompute extension stats after filtering

### Phase 6 -- Tests (21KB plan)
- Update registry tests: edge counts, label assertions
- Update model tests: snapshot regeneration
- Create formal_manifest.rs and governance_manifest.rs test files (gap found)
- Full regression: `cargo test --workspace` + `cargo clippy --workspace`

### Phase 7 -- Re-Score (19KB plan)
- Pre-verification checklist (12 gates)
- Model regeneration in all 5 formats
- 20 expert agents with scoring rubric
- 3 scoring rounds (initial → 7b fixes → 7c fixes)
- Final: avg 8.6/10, no axis below 8

### Phase 7b -- Targeted Fixes Round 1
- Split `UsesType` into `BehaviorReferencesType` + `EventPayloadType` + `TypeComposesType`
- Rename `fieldType` field to `composed_types` (fix camelCase)
- Rename `MaintainsCondition` to `BehaviorMaintainsCondition`
- Add `DecisionImposesConstraint` + `FailureModeAffectsBehavior` edges

### Phase 7c -- Targeted Fixes Round 2
- Fix 4 phantom edges: add `type.extends`, `process.sub_processes`, `refinement.chains_to`, `axiom.assumes`
- Reverse `ConditionAssumedByAxiom` → `AxiomAssumesCondition` (active voice)
- Rename `EventPayloadType` → `EventCarriesPayloadType` (verb phrase)
- Rename `TermSeeAlso` → `TermReferencesRelatedTerm` (SourceVerbTarget)
- Rename `decision.decision` → `decision.statement` (fix name collision)
- Add description to `milestone.behaviors` field

## Files Modified (Complete)

### Manifests (4)
- `extensions/software/manifest.json` -- P1 (module ports), P2 (4 renames), P7b (split UsesType, rename fieldType), P7c (add type.extends, milestone.behaviors desc, rename EventCarriesPayloadType)
- `extensions/product/manifest.json` -- P1 (FeatureRelatesTo), P2 (16 renames), P3 (capability fields), P4 (PersonaFeature), P7c (rename TermReferencesRelatedTerm)
- `extensions/governance/manifest.json` -- P1 (remove supersedes), P2 (3 renames), P4 (3 new edges + peer dep), P7b (+2 edges), P7c (rename decision.statement)
- `extensions/formal/manifest.json` -- P1 (RefinesTo split), P2 (5 renames), P7b (rename BehaviorMaintainsCondition), P7c (add axiom.assumes, process.sub_processes, refinement.chains_to, reverse AxiomAssumesCondition)

### Rust Source (3-5)
- `crates/specforge-emitter/src/model/build.rs` -- Phase 5
- `crates/specforge-emitter/src/model/mod.rs` -- Phase 5
- `crates/specforge-emitter/src/model/filter.rs` -- Phase 5
- `crates/specforge-registry/src/compilation/populate.rs` -- Phase 5 (possible)
- `crates/specforge-emitter/src/schema.rs` -- Phase 5 (possible)

### Tests (8+)
- `crates/specforge-registry/tests/product_manifest.rs`
- `crates/specforge-registry/tests/software_manifest.rs`
- `crates/specforge-registry/tests/zero_entity_registries.rs`
- `crates/specforge-emitter/tests/model.rs`
- `crates/specforge-emitter/tests/schema.rs`
- `crates/specforge-registry/tests/formal_manifest.rs` (NEW)
- `crates/specforge-registry/tests/governance_manifest.rs` (NEW)
- All snapshot files in `crates/specforge-emitter/src/model/snapshots/`
