# Phase 2: Edge Naming Standardization

**Status**: COMPLETE (2026-04-12)
**Depends on**: Phase 1 (P0 Manifest Bugs)
**Targets axes**: 4 (Naming Consistency), 17 (Learnability), 18 (Edge Label Quality)
**Created**: 2026-04-12

---

## 1. Analysis: Which Convention Wins

### The three conventions in the wild

| Convention | Pattern | Examples | Count |
|------------|---------|----------|-------|
| A: Verb-only | `{Verb}` | `Produces`, `Consumes`, `Enforces`, `Satisfies` | 9 |
| B: SourceTarget | `{Source}{Target}` | `JourneyFeature`, `MilestoneModule`, `ReleaseDeliverable` | 17 |
| C: SourceVerbTarget | `{Source}{Verb}{Target}` or `{Source}{Verb}` | `FeatureDependsOn`, `ConstraintProtects`, `LibraryDefinesPort` | 28 |

### Decision: Convention C (SourceVerbTarget) wins

**Rationale:**

1. **Self-referencing edges demand disambiguation.** `FeatureDependsOn` (feature->feature), `MilestoneDependsOn` (milestone->milestone), `ProcessComposition` (process->process) -- verb-only labels like `DependsOn` would be ambiguous when multiple entity kinds can depend on themselves. The source prefix resolves this.

2. **Graph query readability.** When an agent or user runs `specforge trace --edges BehaviorProducesEvent`, the label tells them exactly what they are looking at: the source kind, the relationship verb, and the target kind. Compare with `Produces` (what produces what?) or `JourneyFeature` (what does the journey do with the feature?).

3. **Convention C already dominates.** 28 of 54 edges (~52%) already follow SourceVerbTarget or the closely related SourceVerb pattern. Convention B is the second most common (17 edges, ~31%). Renaming toward C requires the fewest total changes.

4. **Learnability.** A single convention means users learn one pattern and can predict any edge label. SourceVerbTarget is the most predictable: if you know the source kind, the relationship, and the target kind, you can guess the label.

### When SourceVerbTarget is NOT needed

Not every edge needs the full `{Source}{Verb}{Target}` triple. The target suffix can be omitted when:

- The verb already implies a unique target kind (e.g., `BehaviorEnforcesInvariant` is clearer than `BehaviorEnforces`, but `TypeExtendsType` is clearer than `TypeExtends` since types can only extend types).
- Self-referencing edges: always include the source prefix but the target is implicit (e.g., `TermSeeAlso` -- term->term -- the source is clear and "SeeAlso" already implies same-kind).

**Convention applied**: Use `{Source}{Verb}{Target}` as the default. Drop the target suffix ONLY when the edge is self-referencing AND the verb is unambiguous, producing `{Source}{Verb}` (e.g., `FeatureDependsOn`, `TermSeeAlso`). Never use verb-only or source-target-without-verb.

---

## 2. Full Edge Inventory and Rename Mapping

### Legend

- **KEEP**: Edge already follows the convention; no rename needed.
- **RENAME**: Edge needs a new label to match the convention.
- **Reason codes**: `A->C` = converting from verb-only; `B->C` = adding a verb; `OK` = already consistent.

### 2.1 @specforge/software (11 edges)

| # | Current | Proposed | Action | Reason |
|---|---------|----------|--------|--------|
| S1 | `References` | `References` | KEEP | Generic cross-ref edge with no sourceKind/targetKind constraints. Renaming would require a polymorphic prefix. Since this edge is untyped (no source/target kind), it serves as a fallback and the generic name is correct. |
| S2 | `Implements` | `BehaviorImplementsFeature` | RENAME | A->C. Source is always behavior, target is always feature. Without prefix, agents cannot distinguish from a hypothetical future "ModuleImplementsPort". |
| S3 | `Produces` | `BehaviorProducesEvent` | RENAME | A->C. Source is always behavior, target is always event. |
| S4 | `Consumes` | `BehaviorConsumesEvent` | RENAME | A->C. Source is always behavior, target is always event. |
| S5 | `UsesType` | `UsesType` | KEEP | Polymorphic edge: source can be behavior, event, or type. No single source prefix applies. The existing name follows a verb-target pattern which is acceptable for polymorphic edges. |
| S6 | `UsesPort` | `BehaviorUsesPort` | RENAME | A->C. Source is always behavior (the entity_enhancement on module uses separate edges). |
| S7 | `Enforces` | `BehaviorEnforcesInvariant` | RENAME | A->C. Source is always behavior, target is always invariant. |
| S8 | `ExtendsType` | `TypeExtendsType` | RENAME | Partial C (had verb+target, missing source). Source is always type, target is always type. |
| S9 | `TestedBy` | `TestedBy` | KEEP | Polymorphic edge: any testable entity can be tested. No source prefix appropriate. |
| S10 | `ExternalRef` | `ExternalRef` | KEEP | Polymorphic edge: any entity can have external refs. Generic utility edge. |
| S11 | `MilestoneBehavior` | `MilestoneIncludesBehavior` | RENAME | B->C. "MilestoneBehavior" lacks a verb. The milestone *includes* or *delivers* a behavior. "Includes" is the clearest verb. |

**Summary**: 5 renames, 6 kept. Polymorphic/untyped edges (`References`, `UsesType`, `TestedBy`, `ExternalRef`) are exempt from the source prefix rule because they have no fixed source kind.

### 2.2 @specforge/product (26 edges)

| # | Current | Proposed | Action | Reason |
|---|---------|----------|--------|--------|
| P1 | `FeatureDependsOn` | `FeatureDependsOn` | KEEP | OK. Self-referencing, verb is clear, target omitted per convention. |
| P2 | `JourneyFeature` | `JourneyExercisesFeature` | RENAME | B->C. Journey exercises/includes features along its flow. "Exercises" conveys the journey traverses features. |
| P3 | `JourneyPersona` | `JourneyTargetsPersona` | RENAME | B->C. Journey targets a specific persona. |
| P4 | `JourneyChannel` | `JourneyUsesChannel` | RENAME | B->C. Journey uses a communication channel. |
| P5 | `DeliverableJourney` | `DeliverableSupportsJourney` | RENAME | B->C. Deliverable supports/fulfills a journey. |
| P6 | `DeliverableModule` | `DeliverableContainsModule` | RENAME | B->C. Deliverable contains modules. |
| P7 | `DeliverableMilestone` | `DeliverableTrackedByMilestone` | RENAME | B->C. Deliverable is tracked under a milestone. |
| P8 | `DeliverableDependsOn` | `DeliverableDependsOn` | KEEP | OK. Self-referencing, verb is clear. |
| P9 | `MilestoneFeature` | `MilestoneDeliversFeature` | RENAME | B->C. Milestone delivers features as exit criteria. |
| P10 | `MilestoneModule` | `MilestoneScopesModule` | RENAME | B->C. Milestone scopes modules within its boundary. |
| P11 | `MilestoneDependsOn` | `MilestoneDependsOn` | KEEP | OK. Self-referencing, verb is clear. |
| P12 | `ModuleFeature` | `ModuleContainsFeature` | RENAME | B->C. Module contains features. |
| P13 | `ModuleDependsOn` | `ModuleDependsOn` | KEEP | OK. Self-referencing, verb is clear. |
| P14 | `TermSeeAlso` | `TermSeeAlso` | KEEP | OK. Self-referencing, verb is clear ("see also"). |
| P15 | `TermBelongsToModule` | `TermBelongsToModule` | KEEP | OK. Already follows SourceVerbTarget. |
| P16 | `ReleaseDeliverable` | `ReleaseIncludesDeliverable` | RENAME | B->C. Release includes deliverables. |
| P17 | `ReleaseMilestone` | `ReleaseCompletesMilestone` | RENAME | B->C. Release completes milestones. |
| P18 | `ReleaseDependsOn` | `ReleaseDependsOn` | KEEP | OK. Self-referencing, verb is clear. |
| P19 | `CapabilityFeature` | `CapabilityComposesFeature` | RENAME | B->C. Capability is composed of features. |
| P20 | `LibraryFeature` | `LibraryProvidesFeature` | RENAME | B->C. Library provides/supports features. |
| P21 | `LibraryDependsOn` | `LibraryDependsOn` | KEEP | OK. Self-referencing, verb is clear. |
| P22 | `LibraryDefinesPort` | `LibraryDefinesPort` | KEEP | OK. Already follows SourceVerbTarget. |
| P23 | `LibraryConsumesPort` | `LibraryConsumesPort` | KEEP | OK. Already follows SourceVerbTarget. |
| P24 | `RoadmapBehavior` | `RoadmapPlansBehavior` | RENAME | B->C. Roadmap plans/includes behaviors. |
| P25 | `RoadmapFeature` | `RoadmapPlansFeature` | RENAME | B->C. Roadmap plans/includes features. |
| P26 | `RoadmapLibrary` | `RoadmapPlansLibrary` | RENAME | B->C. Roadmap plans/includes libraries. |
| P27 | `RoadmapDependsOn` | `RoadmapDependsOn` | KEEP | OK. Self-referencing, verb is clear. |

**Summary**: 15 renames, 12 kept. All self-referencing `*DependsOn` and `*SeeAlso` edges are already compliant. The `TermBelongsToModule`, `LibraryDefinesPort`, and `LibraryConsumesPort` edges already follow SourceVerbTarget.

### 2.3 @specforge/governance (6 edges)

| # | Current | Proposed | Action | Reason |
|---|---------|----------|--------|--------|
| G1 | `DecisionProtects` | `DecisionProtectsInvariant` | RENAME | Partial C (missing target). Adding target for full clarity since decisions could theoretically protect other things. |
| G2 | `ConstraintEnforces` | `ConstraintEnforcedByBehavior` | RENAME | Verb direction mismatch. The field is `enforced_by` on constraint, meaning behaviors enforce the constraint. The edge goes constraint->behavior. Better: `ConstraintEnforcedByBehavior` to match the field semantics. |
| G3 | `Supersedes` | `DecisionSupersedesDecision` | RENAME | A->C. Self-referencing (decision->decision). Source prefix needed for clarity even though only decisions supersede. Target suffix added because the verb alone does not indicate self-reference. |
| G4 | `ConstraintConstrains` | `ConstraintConstrainsBehavior` | RENAME | Partial C (missing target). Adding target for disambiguation with G2. |
| G5 | `ConstraintProtects` | `ConstraintProtectsInvariant` | RENAME | Partial C (missing target). Parallel with G1. |
| G6 | `FailureModeTargets` | `FailureModeTargetsInvariant` | RENAME | Partial C (missing target). Makes the target explicit. |

**Summary**: 6 renames, 0 kept. Governance edges were the least consistent; all need target suffixes.

### 2.4 @specforge/formal (11 edges)

| # | Current | Proposed | Action | Reason |
|---|---------|----------|--------|--------|
| F1 | `RequiresCondition` | `BehaviorRequiresCondition` | RENAME | Missing source prefix. Source is always behavior. |
| F2 | `EnsuresCondition` | `BehaviorEnsuresCondition` | RENAME | Missing source prefix. Source is always behavior. |
| F3 | `MaintainsCondition` | `MaintainsCondition` | KEEP | Polymorphic: source can be behavior OR invariant. No single source prefix. The verb+target pattern is acceptable for polymorphic edges (same rationale as `UsesType`). |
| F4 | `AssumedBy` | `ConditionAssumedByAxiom` | RENAME | A->C. Source is always condition, target is always axiom. Without context, "AssumedBy" is opaque. |
| F5 | `Satisfies` | `BehaviorSatisfiesProperty` | RENAME | A->C. Source is always behavior, target is always property. |
| F6 | `FollowsProtocol` | `EventFollowsProtocol` | RENAME | Missing source prefix. Source is always event. |
| F7 | `PropertyDependsOn` | `PropertyDependsOnCondition` | RENAME | Partial C (missing target). Adding target for full clarity. Distinguishes from all other `*DependsOn` edges. |
| F8 | `RefinesTo` | `RefinementRefinesToBehavior` | RENAME | Missing source prefix. Source is always refinement, target is always behavior. |
| F9 | `RefinementChainLink` | `RefinementChainsToRefinement` | RENAME | B->C. Adding a verb. Self-referencing (refinement->refinement). "ChainsTo" conveys the multi-level linking. |
| F10 | `ParticipatesIn` | `EventParticipatesInProcess` | RENAME | A->C. Source is always event, target is always process. |
| F11 | `ProcessComposition` | `ProcessComposesProcess` | RENAME | B->C. Adding a verb. Self-referencing (process->process). |

**Summary**: 10 renames, 1 kept. Only `MaintainsCondition` is polymorphic and exempt.

---

## 3. Complete Rename Summary

| Extension | Total Edges | Renames | Kept | Rename Rate |
|-----------|-------------|---------|------|-------------|
| @specforge/software | 11 | 5 | 6 | 45% |
| @specforge/product | 27 | 15 | 12 | 56% |
| @specforge/governance | 6 | 6 | 0 | 100% |
| @specforge/formal | 11 | 10 | 1 | 91% |
| **Total** | **55** | **36** | **19** | **65%** |

### Exempt edges (polymorphic/untyped -- no fixed source kind)

These 7 edges have no single source kind and are exempt from the source-prefix rule:

| Edge | Reason |
|------|--------|
| `References` | Untyped: no source/target constraint |
| `UsesType` | Polymorphic: behavior, event, or type as source |
| `TestedBy` | Polymorphic: any testable entity |
| `ExternalRef` | Polymorphic: any entity |
| `MaintainsCondition` | Polymorphic: behavior or invariant as source |
| `TermSeeAlso` | Self-referencing, already has source + verb |
| `TermBelongsToModule` | Already SourceVerbTarget |

---

## 4. Implementation Steps

### Step 4.1: Manifest JSON files (4 files)

Update edge labels in `edgeTypes[].label` and all `fields[].edge` references within entity kind declarations and entity enhancements.

**Files:**

| File | Changes |
|------|---------|
| `extensions/software/manifest.json` | 5 edge label renames in `edgeTypes`, ~8 field `edge` references |
| `extensions/product/manifest.json` | 15 edge label renames in `edgeTypes`, ~15 field `edge` references |
| `extensions/governance/manifest.json` | 6 edge label renames in `edgeTypes`, ~6 field `edge` references |
| `extensions/formal/manifest.json` | 10 edge label renames in `edgeTypes`, ~8 field `edge` references + entity enhancement `edge` references |

**Procedure for each manifest:**

1. Rename `label` in every affected `edgeTypes[]` entry.
2. Search all `entityKinds[].fields[]` for `"edge": "OldLabel"` and update to new label.
3. Search all `entityEnhancements[].fields[]` for `"edge": "OldLabel"` and update to new label.
4. Update `validationRules[].edgeType` references where they match renamed edges.

### Step 4.2: Validation rule messageTemplate strings

Validation rules reference edge types in their `edgeType` field (used for `no_incoming_edges`, `no_outgoing_edges`, `cycle_detection` checks). These must be updated.

**Files:**

| File | Rules to update |
|------|-----------------|
| `extensions/software/manifest.json` | W001 (`Implements`), W008 (`Implements`) |
| `extensions/product/manifest.json` | E007 (`ModuleDependsOn`), E015 (`MilestoneDependsOn`), E016 (`DeliverableDependsOn`), W045 (`FeatureDependsOn`), W092 (`ReleaseDependsOn`), W096 (`LibraryDependsOn`), W097 (`RoadmapDependsOn`) |

Note: The product extension validation rules that reference `*DependsOn` edges are NOT being renamed (they are already convention-compliant), so no change needed for E007, E015, E016, W045, W092, W096, W097. Only W001 and W008 in the software manifest need updating (`Implements` -> `BehaviorImplementsFeature`).

### Step 4.3: Spec files referencing edge labels

Update spec files that mention edge labels in contracts, ensures, invariant guarantees, or behavior descriptions.

**Files:**

| File | References |
|------|------------|
| `spec/extensions/software/manifest.spec` | Edge list in contracts and ensures (References, Implements, Produces, ...) |
| `spec/extensions/product/manifest.spec` | Edge list in contracts and ensures (JourneyFeature, DeliverableJourney, ...) |
| `spec/extensions/governance/manifest.spec` | Edge list in contracts and ensures (DecisionInvariant, ConstrainsBehavior, ...) |
| `spec/extensions/formal/manifest.spec` | Edge list in contracts and ensures (RequiresCondition, EnsuresCondition, ...) |

Note: The governance spec already uses different names (`DecisionInvariant`, `ConstrainsBehavior`, `ProtectsInvariant`, `FailureModeInvariant`) than the actual manifest JSON (`DecisionProtects`, `ConstraintEnforces`, `Supersedes`, etc.). This discrepancy is a Phase 1 bug. Phase 2 should fix the spec files to match the new standardized names.

### Step 4.4: Rust test files

Tests that assert on edge label strings must be updated.

**Files and approximate change counts:**

| File | Edge labels referenced | Changes |
|------|----------------------|---------|
| `crates/specforge-registry/tests/software_manifest.rs` | `References`, `Implements`, `Produces`, `Consumes`, `UsesType`, `UsesPort`, `Enforces`, `ExtendsType`, `TestedBy`, `ExternalRef`, `MilestoneBehavior` | ~20 string literal updates |
| `crates/specforge-registry/tests/product_manifest.rs` | `JourneyFeature`, `JourneyPersona`, `JourneyChannel`, `DeliverableJourney`, `DeliverableModule`, `MilestoneFeature`, `MilestoneModule`, `ModuleFeature`, `TermSeeAlso`, `TermBelongsToModule` | ~15 string literal updates |
| `crates/specforge-graph/tests/graph.rs` | `Implements`, `Produces`, `Consumes`, `Enforces` | ~8 string literal updates |
| `crates/specforge-emitter/tests/model.rs` | `Implements`, `TermBelongsToModule`, `JourneyFeature` | ~12 string literal updates |
| `crates/specforge-lsp/tests/hover.rs` | `Implements` | ~2 string literal updates |
| `crates/specforge-lsp/src/hover.rs` | `Implements` (if hardcoded) | ~1 string literal update |
| `crates/specforge-cli/tests/e2e_fixtures.rs` | `Implements`, `Produces`, `Consumes`, `Enforces` | ~4 string literal updates |
| `crates/specforge-lsp/tests/e2e_support/hover.rs` | `Implements` | ~1 string literal update |
| `crates/specforge-lsp/tests/e2e_support/integration.rs` | `Implements` | ~1 string literal update |

### Step 4.5: Snapshot files

All snapshot files containing edge labels must be regenerated. These are auto-updated by `cargo insta review` after the source changes.

**Files (auto-updated):**

| File |
|------|
| `crates/specforge-emitter/tests/snapshots/tests__model__render_json_keys.snap` |
| `crates/specforge-emitter/tests/snapshots/tests__model__render_mermaid_keys_grouped.snap` |
| `crates/specforge-emitter/tests/snapshots/tests__model__render_mermaid_none_fields.snap` |
| `crates/specforge-emitter/tests/snapshots/tests__model__render_markdown_keys_grouped.snap` |
| `crates/specforge-emitter/tests/snapshots/tests__model__render_markdown_flat.snap` |
| `crates/specforge-emitter/tests/snapshots/tests__model__render_markdown_none_fields.snap` |
| `crates/specforge-emitter/tests/snapshots/tests__model__render_dot_keys_grouped.snap` |
| `crates/specforge-emitter/tests/snapshots/tests__model__render_dot_none_fields.snap` |
| `crates/specforge-emitter/tests/snapshots/tests__model__render_dbml_keys_grouped.snap` |

### Step 4.6: Schema type references

The `SchemaEdgeType` in `crates/specforge-emitter/src/schema.rs` uses `label: String` -- this is data-driven from manifests, so no code change needed. The edge registry in `crates/specforge-registry/src/registries/edge.rs` is also string-keyed. No Rust enum or constant needs renaming -- all edge labels are string literals flowing from JSON manifests.

### Step 4.7: Client-facing registry config

Check `crates/specforge-registry/src/client/registry_config.rs` for any hardcoded edge labels used in default configurations.

---

## 5. Implementation Order

Execute in this sequence to minimize broken intermediate states:

```
Step 1: Update manifest JSON files (4.1 + 4.2)
  |  All edge label renames happen here. The manifests are the source of truth.
  v
Step 2: Update spec files (4.3)
  |  Spec contracts must match the new manifest labels.
  v
Step 3: Update Rust test string literals (4.4)
  |  Tests must match the new manifest labels.
  v
Step 4: Regenerate snapshots (4.5)
  |  Run: cargo insta test --accept
  v
Step 5: Run full test suite
  |  cargo test --workspace
  v
Step 6: Verify LSP and MCP still work
  |  Run e2e tests: cargo test -p specforge-cli --test e2e_mcp
  v
Step 7: Update MEMORY.md edge type references
```

---

## 6. Risk Assessment

### 6.1 Breaking change severity: MEDIUM

Edge labels are part of the **Graph Protocol schema** -- the public API that agents and tools consume. Renaming edges is a breaking change for:

- **Agent prompts** that reference specific edge labels (e.g., "follow the `Implements` edge").
- **Exported graphs** (JSON) that use edge labels as keys.
- **User queries** (e.g., `specforge trace --edges Implements`).
- **MCP resources** that expose edge-specific data.

### 6.2 Mitigation strategies

| Risk | Mitigation |
|------|------------|
| Agent prompts break | Document the rename mapping. Agents re-read schema on each invocation, so they will pick up new names automatically if they query the schema. |
| Exported graphs change | This is a schema version bump (minor). Old exports remain valid for their version. New exports use new labels. |
| User muscle memory | The `specforge migrate` infrastructure can emit a one-time warning listing renamed edges. |
| Extension authors with custom edges | Phase 2 only renames first-party extension edges. Third-party extensions are unaffected. |
| Partially applied rename | Execute all manifest renames atomically in a single commit. Never leave manifests and tests out of sync. |

### 6.3 Rollback plan

All changes are in string literals (manifest JSON + Rust test strings + snapshot files). A `git revert` of the Phase 2 commit cleanly undoes everything.

### 6.4 Schema version impact

This rename constitutes a **minor version bump** of the Graph Protocol schema (e.g., 1.0.0 -> 1.1.0) because:
- No new edge types are added (additive-only for minor).
- Edge types are renamed, which is a breaking change at the edge-label level but not at the structural level.
- However, per the schema evolution strategy (additive-only for minor versions), renames are technically breaking. Consider bumping to 2.0.0 if strict semver is enforced, or providing an alias/migration layer.

**Recommendation**: Bump to schema version 1.1.0 with a `deprecated_edge_labels` map in the schema output that maps old names to new names for one release cycle. Remove the map in 1.2.0.

---

## 7. Deprecation Alias Layer (Optional Enhancement)

To smooth the transition, add a `deprecated_edge_labels` field to `ManifestEdgeType`:

```rust
pub struct ManifestEdgeType {
    pub label: String,
    pub deprecated_aliases: Vec<String>,  // Old names that still resolve
    // ... existing fields
}
```

The edge registry would index both `label` and all `deprecated_aliases`, emitting an `I-level` diagnostic when a deprecated alias is used:

```
I098: edge label 'Implements' is deprecated — use 'BehaviorImplementsFeature'
```

This allows existing spec files and agent prompts to keep working during the transition period. The aliases would be removed in the next major version.

**Estimated effort**: ~2 hours for the registry change + ~1 hour for diagnostic formatting. This is OPTIONAL and can be deferred if the breaking change is acceptable.

---

## 8. Quick-Reference: Old Name to New Name

Sorted alphabetically by old name for easy lookup:

| Old Name | New Name | Extension |
|----------|----------|-----------|
| `AssumedBy` | `ConditionAssumedByAxiom` | formal |
| `BehaviorRequiresCondition` | *(new from RequiresCondition)* | formal |
| `BehaviorEnsuresCondition` | *(new from EnsuresCondition)* | formal |
| `CapabilityFeature` | `CapabilityComposesFeature` | product |
| `ConstraintConstrains` | `ConstraintConstrainsBehavior` | governance |
| `ConstraintEnforces` | `ConstraintEnforcedByBehavior` | governance |
| `ConstraintProtects` | `ConstraintProtectsInvariant` | governance |
| `Consumes` | `BehaviorConsumesEvent` | software |
| `DecisionProtects` | `DecisionProtectsInvariant` | governance |
| `DeliverableJourney` | `DeliverableSupportsJourney` | product |
| `DeliverableMilestone` | `DeliverableTrackedByMilestone` | product |
| `DeliverableModule` | `DeliverableContainsModule` | product |
| `Enforces` | `BehaviorEnforcesInvariant` | software |
| `EnsuresCondition` | `BehaviorEnsuresCondition` | formal |
| `ExtendsType` | `TypeExtendsType` | software |
| `FailureModeTargets` | `FailureModeTargetsInvariant` | governance |
| `FollowsProtocol` | `EventFollowsProtocol` | formal |
| `Implements` | `BehaviorImplementsFeature` | software |
| `JourneyChannel` | `JourneyUsesChannel` | product |
| `JourneyFeature` | `JourneyExercisesFeature` | product |
| `JourneyPersona` | `JourneyTargetsPersona` | product |
| `MilestoneBehavior` | `MilestoneIncludesBehavior` | software |
| `MilestoneFeature` | `MilestoneDeliversFeature` | product |
| `MilestoneModule` | `MilestoneScopesModule` | product |
| `ModuleFeature` | `ModuleContainsFeature` | product |
| `ParticipatesIn` | `EventParticipatesInProcess` | formal |
| `ProcessComposition` | `ProcessComposesProcess` | formal |
| `Produces` | `BehaviorProducesEvent` | software |
| `PropertyDependsOn` | `PropertyDependsOnCondition` | formal |
| `RefinementChainLink` | `RefinementChainsToRefinement` | formal |
| `RefinesTo` | `RefinementRefinesToBehavior` | formal |
| `ReleaseDeliverable` | `ReleaseIncludesDeliverable` | product |
| `ReleaseMilestone` | `ReleaseCompletesMilestone` | product |
| `RoadmapBehavior` | `RoadmapPlansBehavior` | product |
| `RoadmapFeature` | `RoadmapPlansFeature` | product |
| `RoadmapLibrary` | `RoadmapPlansLibrary` | product |
| `Satisfies` | `BehaviorSatisfiesProperty` | formal |
| `Supersedes` | `DecisionSupersedesDecision` | governance |
| `UsesPort` | `BehaviorUsesPort` | software |

---

## 9. Progress Tracking

### Manifests
- [ ] `extensions/software/manifest.json` -- 5 edge renames + field refs + validation rule refs
- [ ] `extensions/product/manifest.json` -- 15 edge renames + field refs
- [ ] `extensions/governance/manifest.json` -- 6 edge renames + field refs
- [ ] `extensions/formal/manifest.json` -- 10 edge renames + field refs + enhancement refs

### Spec files
- [ ] `spec/extensions/software/manifest.spec` -- update edge lists in contracts/ensures/invariants
- [ ] `spec/extensions/product/manifest.spec` -- update edge lists in contracts/ensures/invariants
- [ ] `spec/extensions/governance/manifest.spec` -- update edge lists in contracts/ensures/invariants
- [ ] `spec/extensions/formal/manifest.spec` -- update edge lists in contracts/ensures/invariants

### Rust tests
- [ ] `crates/specforge-registry/tests/software_manifest.rs`
- [ ] `crates/specforge-registry/tests/product_manifest.rs`
- [ ] `crates/specforge-graph/tests/graph.rs`
- [ ] `crates/specforge-emitter/tests/model.rs`
- [ ] `crates/specforge-lsp/tests/hover.rs`
- [ ] `crates/specforge-lsp/src/hover.rs`
- [ ] `crates/specforge-cli/tests/e2e_fixtures.rs`
- [ ] `crates/specforge-lsp/tests/e2e_support/hover.rs`
- [ ] `crates/specforge-lsp/tests/e2e_support/integration.rs`

### Snapshots
- [ ] Run `cargo insta test --accept` to regenerate all snapshots
- [ ] Review snapshot diffs for correctness

### Verification
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace` clean
- [ ] `cargo test -p specforge-cli --test e2e_mcp` passes
- [ ] Schema version bumped appropriately
- [ ] MEMORY.md edge references updated

### Optional
- [ ] Implement `deprecated_aliases` in `ManifestEdgeType`
- [ ] Add I098 diagnostic for deprecated edge label usage
- [ ] Add migration warning output for renamed edges
