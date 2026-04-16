# PRD-002: Entity Audit & Cleanup

**Status:** Completed
**Author:** Mohammad AL Mechkor
**Date:** 2026-04-12

---

## Problem Statement

SpecForge's four extensions collectively declared 27 entity kinds. A 10-expert panel review (RES-27) and internal audit revealed that several entity kinds were redundant — they had identical field sets and semantics as existing kinds, adding vocabulary without adding expressive power. This violated Principle 2 (zero domain knowledge in core) by inflating the schema with synonyms, and made the graph harder for agents to navigate (more node types = more token cost per query).

Specific problems:

1. **`library`** had the same fields as `module` (family, features, depends_on). Two words for the same concept.

2. **`capability`** had the same fields as `journey` (persona, channels, features, flow). It was a planning-oriented alias for the same user-flow concept.

3. **`roadmap`** was covered by `milestone` + `release` together. Milestones handle planning phases; releases handle versioned shipments. Roadmap added a third concept without adding information.

4. **`condition`** in `@specforge/formal` was a standalone entity kind, but the design evolved to use `invariant` (from `@specforge/software`) as the target for structured condition references (requires/ensures/maintains). Having both `condition` and `invariant` created confusion about which to use for preconditions and postconditions.

5. **Testability fields** (`testable`, `supportsVerify`, `allowedVerifyKinds`, `verifyKinds`) were scattered across all 4 manifests, tightly coupling testing concepts to core domain extensions. The `tests` and `gherkin` fields on behavior, the `TestedBy` edge type, and the W004/W009 validation rules all belonged to a testing concern that not every project needs.

## Solution

Remove the redundant entities, retarget condition references to invariant, and strip all testability fields from the manifests. This reduces the entity count from 27 to 22, simplifies the schema, and prepares the ground for extracting testing into a standalone extension (PRD-003).

### Entity Removals

| Removed | Replacement | Migration Path |
|---------|-------------|----------------|
| `library` | `module` | Rename keyword in spec files. All fields carry over directly. |
| `capability` | `journey` | Rename keyword. Fields (persona, channels, features, flow) are identical. |
| `roadmap` | `milestone` + `release` | Split planning data into milestones, shipping data into releases. |
| `condition` | `invariant` | Retarget all condition edges to invariant. Rename `condition_deltas` to `invariant_deltas`. |

### Testability Field Removal

Removed from all 4 manifests:
- `testable` (boolean on every entity kind)
- `supportsVerify` (boolean on every entity kind)
- `allowedVerifyKinds` (array on entity kinds that had it)
- `verifyKinds` (top-level array on product and software)
- `tests` and `gherkin` fields on behavior (moved to software-testing)
- `TestedBy` edge type (moved to software-testing)
- W004 validation rules (5 variants, one per software entity kind)
- W009 validation rule (verify kind allowlist check)

## User Stories

1. As an AI agent consuming the SpecForge graph, I want each concept to have exactly one entity kind, so that I don't waste tokens disambiguating synonyms like library vs. module.

2. As a spec author, I want to use `module` for code packages without wondering whether `library` is more appropriate, so that I make one decision instead of two.

3. As a spec author, I want to use `journey` for user flows without wondering whether `capability` is more appropriate, so that my vocabulary is unambiguous.

4. As a spec author, I want to use `milestone` for planning phases and `release` for versioned shipments, so that I don't need a third `roadmap` concept that overlaps both.

5. As a formal methods user, I want to reference `invariant` in requires/ensures/maintains blocks, so that preconditions and postconditions point to the same entity kind that `@specforge/software` already defines.

6. As a spec author writing formal refinements, I want the `invariant_deltas` field (not `condition_deltas`) on refinement entities, so that the field name matches the target entity kind.

7. As a project author who doesn't use BDD testing, I want entity kinds without `testable` and `supportsVerify` flags, so that my schema output doesn't include testing concepts I don't use.

8. As a project author, I want `@specforge/software` to have 14 edge types (not 15), with `TestedBy` moved to the testing extension, so that my graph doesn't include test-traceability edges unless I opt in.

9. As an extension author, I want `@specforge/software` to have 12 validation rules (not 18), with W004 and W009 moved to the testing extension, so that test-coverage warnings only fire when the testing extension is installed.

10. As a SpecForge contributor, I want all 2,600+ tests to pass after the entity cleanup, so that I know the removal didn't break any existing functionality.

11. As a formal methods user, I want the 5 retargeted edge types (`BehaviorRequiresInvariant`, `BehaviorEnsuresInvariant`, `BehaviorMaintainsInvariant`, `AxiomAssumesInvariant`, `PropertyDependsOnInvariant`) to work correctly, so that structured conditions reference invariants instead of the removed condition entity.

12. As a product extension user, I want 9 entity kinds (not 10) after roadmap removal, so that the schema is cleaner and `milestone` + `release` cover all planning and shipping needs.

## Implementation Decisions

### Removal Strategy

Entities are not just deleted; all references are retargeted:

- **library removal**: All `library` references in tests, manifests, and docs replaced with `module`.
- **capability removal**: All `capability` references replaced with `journey`.
- **roadmap removal**: Entity kind removed from `@specforge/product`. Two edge types removed (`RoadmapPlansFeature`, `RoadmapDependsOn`). Two validation rules removed (`W097` roadmap cycle, `W098` roadmap status). Entity enhancement on roadmap removed from `@specforge/software` (including `RoadmapPlansBehavior` edge).
- **condition retargeting**: Entity kind removed from `@specforge/formal`. Five edge types retargeted (`*Condition` renamed to `*Invariant`). Three enhancement fields on behavior retargeted to invariant. Axiom's `assumes` field retargeted to invariant. `condition_deltas` field renamed to `invariant_deltas`. W058 validation rule removed.

### Testability Field Stripping

All `testable`, `supportsVerify`, `allowedVerifyKinds` removed from every entity kind across all 4 manifests. `verifyKinds` removed from product and software extension level. The Rust types (`ManifestEntityKind`, `ManifestV2`) retain these fields with `#[serde(default)]`, so deserialization produces `false`/empty rather than failing. This allows forward compatibility if a future testing extension re-introduces them.

### Software Manifest Changes

- Edge types: 15 to 14 (removed `TestedBy`)
- Validation rules: 18 to 12 (removed 5x W004, 1x W009)
- Behavior fields: removed `tests` and `gherkin`
- Extension-level `verifyKinds` removed

### Product Manifest Changes

- Entity kinds: 9 (removed roadmap)
- Edge types: 20 (removed `RoadmapPlansFeature`, `RoadmapDependsOn`)
- Extension-level `verifyKinds` removed
- All entity kinds: testability fields stripped

### Formal Manifest Changes

- Entity kinds: 5 (removed condition)
- Edge types: 12 (5 retargeted from `*Condition` to `*Invariant`)
- Refinement field: `condition_deltas` renamed to `invariant_deltas`
- All entity kinds: testability fields stripped

### Governance Manifest Changes

- All 3 entity kinds: testability fields stripped (no other changes)

### Final Entity Count

| Extension | Before | After |
|-----------|--------|-------|
| @specforge/product | 10 | 9 |
| @specforge/software | 5 | 5 |
| @specforge/governance | 3 | 3 |
| @specforge/formal | 6 | 5 |
| **Total** | **24** | **22** |

## Testing Decisions

### What Makes a Good Test

Tests verify that manifest deserialization, registry population, and validation rule parsing produce correct results after the entity changes. Tests should assert on observable output (entity counts, edge labels, field presence) not on internal manifest JSON structure.

### Modules Tested

- **specforge-registry manifest tests** (`software_manifest.rs`, `product_manifest.rs`): Updated assertions for entity counts, edge counts, testability flags (now false/empty), verify kinds (now empty), removed field assertions, removed edge assertions, validation rule counts.

- **specforge-emitter outline tests** (`outline.rs`): Updated entity kind and edge type count assertions for product extension.

- **Full workspace regression**: `cargo test --workspace` with zero failures confirms no downstream breakage.

### Prior Art

The existing manifest test files (`crates/specforge-registry/tests/software_manifest.rs`, `product_manifest.rs`) served as the verification gate. Every assertion was updated to match the new manifest state: edge counts, entity counts, testability booleans, verify kind arrays, field presence, and validation rule counts.

## Out of Scope

- **Building @specforge/software-testing**: The testing extension itself is covered by PRD-003. This PRD only strips the testability fields from existing manifests.
- **Removing testability from Rust source types**: The `ManifestEntityKind` struct still has `testable`, `supports_verify`, `allowed_verify_kinds` fields with `#[serde(default)]`. Removing them from the Rust types is deferred to when the Extension Protocol (PRD-001) replaces manifest.json entirely.
- **Updating stale spec/doc files**: Several spec files reference removed entities (`spec/extensions/product/validation-rules.spec`, `invariants.spec`, `failure-modes.spec`, `features.spec`; `docs/quick-reference.md`; `docs/entities/feature.md`; `spec/extensions/formal/manifest.spec`). These are tracked but not addressed in this PRD.

## Further Notes

### Completion Status

This PRD documents work that has been completed. All manifest changes are applied, all tests pass, and the workspace builds cleanly. The changes span:

- 4 manifest JSON files (product, software, formal, governance)
- 2 test files (software_manifest.rs, product_manifest.rs)
- 1 outline test file (outline.rs)

### Validation Source

The entity audit was informed by:
- **RES-27**: 10-expert panel validation of the software entity set
- **Conceptual analysis**: Field-by-field comparison showing library=module, capability=journey
- **Design session**: 22-question iterative interview establishing the extension protocol architecture, which surfaced the redundancies

### Impact on Extension Protocol (PRD-001)

The entity cleanup directly feeds PRD-001. The Extension Protocol's inventory document (`docs/extension-inventory.md`) reflects the post-audit state: 22 entity kinds across 5 extensions (including the future `@specforge/software-testing`). The protocol design was built on the cleaned-up entity set, not the pre-audit 27-entity set.
