# Extension Inventory

This document catalogs the five official SpecForge extensions after the entity audit and testability extraction. Together they provide 22 entity kinds, 58 edge types, and cross-extension enhancements through the Extension Protocol.

## Dependency Graph

Extensions declare peer dependencies that control load order and enable cross-extension references. The graph reads left-to-right: an arrow means "depends on."

```
                   ┌────────────────────┐
                   │  @specforge/product │  (root -- no dependencies)
                   │  9 kinds, 20 edges  │
                   └─────────┬──────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              v              v              │
   ┌──────────────────┐  (implicit)        │
   │ @specforge/       │                    │
   │ software          │                    │
   │ 5 kinds, 14 edges │                    │
   └────────┬─────────┘                    │
            │                              │
     ┌──────┼──────┐                       │
     │             │                       │
     v             v                       v
┌──────────┐ ┌──────────────┐  ┌────────────────────┐
│ formal   │ │ governance   │  │ software-testing    │
│ 5 kinds  │ │ 3 kinds      │  │ 0 kinds             │
│ 12 edges │ │ 11 edges     │  │ 1 edge              │
└──────────┘ └──────────────┘  │ 13 enhancements     │
                               └────────────────────┘
```

Load order: `product` first (no dependencies), then `software` (depends on product), then `formal`, `governance`, and `software-testing` (all depend on software; software-testing also depends on product).

## @specforge/product

**Root extension with no dependencies.** Provides the product planning and delivery vocabulary. Not every project needs this -- internal tools, scripts, and libraries may skip it entirely.

### Entity Kinds (9)

| Kind | Keyword | Singleton | Description |
|------|---------|-----------|-------------|
| Feature | `feature` | no | User-facing capability composed of behaviors |
| Journey | `journey` | no | UX flow mapping a persona and channel to features |
| Deliverable | `deliverable` | no | Shippable artifact bundling journeys and modules |
| Milestone | `milestone` | no | Planning phase with scheduled features and exit criteria |
| Module | `module` | no | Code package mapping features to ports |
| Term | `term` | no | Structured vocabulary for the project's ubiquitous language |
| Persona | `persona` | no | User archetype that interacts with the system |
| Channel | `channel` | no | Interaction medium through which personas access the system |
| Release | `release` | no | Coordinated shipment of deliverables targeting milestones |

### Edge Types (20)

| Edge Type | Source | Target | Semantics |
|-----------|--------|--------|-----------|
| `FeatureDependsOn` | feature | feature | This feature depends on that feature |
| `FeatureRelatesTo` | feature | feature | This feature is related to that feature |
| `JourneyExercisesFeature` | journey | feature | This UX flow exercises these features |
| `JourneyTargetsPersona` | journey | persona | This journey is performed by this persona |
| `JourneyUsesChannel` | journey | channel | This journey occurs through this channel |
| `DeliverableSupportsJourney` | deliverable | journey | This deliverable ships these journeys |
| `DeliverableContainsModule` | deliverable | module | This deliverable includes these modules |
| `DeliverableTrackedByMilestone` | deliverable | milestone | This deliverable targets these milestones |
| `DeliverableDependsOn` | deliverable | deliverable | This deliverable depends on that deliverable |
| `MilestoneDeliversFeature` | milestone | feature | This phase schedules these features |
| `MilestoneScopesModule` | milestone | module | This milestone includes these modules |
| `MilestoneDependsOn` | milestone | milestone | This milestone depends on that milestone completing first |
| `ModuleContainsFeature` | module | feature | This module implements these features |
| `ModuleDependsOn` | module | module | This module depends on that module |
| `TermReferencesRelatedTerm` | term | term | This term cross-references that term |
| `TermBelongsToModule` | term | module | This term belongs to this module's domain |
| `ReleaseIncludesDeliverable` | release | deliverable | This release ships these deliverables |
| `ReleaseCompletesMilestone` | release | milestone | This release targets these milestones |
| `ReleaseDependsOn` | release | release | This release depends on that release |
| `PersonaPrioritizesFeature` | persona | feature | This persona prioritizes this feature |

### Shared Fields

| Field | Type | Description |
|-------|------|-------------|
| `tags` | string_list | Freeform labels applied to all entity kinds |

### Notes

- No testable or verify concepts. Testing was extracted to `@specforge/software-testing`.
- Feature includes a `deprecated` status (terminal state, reachable from `done`).
- Status transition validation (W087-W091, W094) requires an explicit build cache (`specforge-cache.json`).
- Persona and channel are first-class entity kinds, not configuration values.

## @specforge/software

**Depends on: @specforge/product** (peer dependency). Provides the software engineering vocabulary: behavioral contracts, domain events, type definitions, and port interfaces.

### Entity Kinds (5)

| Kind | Keyword | Singleton | Description |
|------|---------|-----------|-------------|
| Behavior | `behavior` | no | Behavioral contract for a single operation |
| Invariant | `invariant` | no | Runtime guarantee the system must never violate |
| Event | `event` | no | Domain or system event emitted by behaviors |
| Type | `type` | no | Data type definition (struct, union, error, command) |
| Port | `port` | no | Interface contract (hexagonal architecture boundary) |

### Edge Types (14)

| Edge Type | Source | Target | Semantics |
|-----------|--------|--------|-----------|
| `References` | any | any | General cross-reference |
| `BehaviorImplementsFeature` | behavior | feature (product) | This behavior implements this feature |
| `BehaviorProducesEvent` | behavior | event | This behavior emits these events |
| `BehaviorConsumesEvent` | behavior | event | This behavior reacts to these events |
| `BehaviorReferencesType` | behavior | type | This behavior uses these type definitions |
| `EventCarriesPayloadType` | event | type | This event carries this payload type |
| `TypeComposesType` | type | type | This type composes or extends that type |
| `BehaviorUsesPort` | behavior | port | This behavior uses these port interfaces |
| `BehaviorEnforcesInvariant` | behavior | invariant | This behavior enforces these invariants |
| `TypeExtendsType` | type | type | This type extends that type (inheritance) |
| `ExternalRef` | any | URI | This entity links to an external reference |
| `MilestoneIncludesBehavior` | milestone (product) | behavior | This milestone delivers these behaviors |
| `ModuleConsumesPort` | module (product) | port | This module consumes these ports |
| `ModuleDefinesPort` | module (product) | port | This module defines these ports |

### Entity Enhancements

| Target Kind | Owner Extension | Added Fields |
|-------------|----------------|-------------|
| `module` | @specforge/product | `ports` (reference_list -> port), `ports_defined` (reference_list -> port) |
| `milestone` | @specforge/product | `behaviors` (reference_list -> behavior) |

### Notes

- No testable or verify concepts. Testing was extracted to `@specforge/software-testing`.
- Cross-extension edges to `feature` (product) use the peer dependency mechanism.
- Enhancement edges (`MilestoneIncludesBehavior`, `ModuleConsumesPort`, `ModuleDefinesPort`) are declared via `#[enhance]` on product entity kinds.

## @specforge/governance

**Depends on: @specforge/software** (peer dependency, which transitively depends on product). Provides architecture governance, quality tracking, and risk assessment.

### Entity Kinds (3)

| Kind | Keyword | Singleton | Description |
|------|---------|-----------|-------------|
| Decision | `decision` | no | Architecture Decision Record with rationale |
| Constraint | `constraint` | no | Non-functional requirement with measurable thresholds |
| FailureMode | `failure_mode` | no | FMEA risk assessment tied to an invariant |

### Edge Types (11)

| Edge Type | Source | Target | Semantics |
|-----------|--------|--------|-----------|
| `DecisionProtectsInvariant` | decision | invariant (software) | This decision protects these invariants |
| `ConstraintEnforcedByBehavior` | constraint | behavior (software) | This constraint applies to these behaviors |
| `DecisionSupersedesDecision` | decision | decision | This decision supersedes that decision |
| `ConstraintConstrainsBehavior` | constraint | behavior (software) | This quality requirement constrains these behaviors |
| `ConstraintProtectsInvariant` | constraint | invariant (software) | This constraint protects these invariants |
| `FailureModeTargetsInvariant` | failure_mode | invariant (software) | This failure mode threatens this invariant |
| `ConstraintGovernsFeature` | constraint | feature (product) | This constraint governs these features |
| `DecisionAffectsFeature` | decision | feature (product) | This decision affects these features |
| `FailureModeThreatensFeature` | failure_mode | feature (product) | This failure mode threatens these features |
| `DecisionImposesConstraint` | decision | constraint | This decision imposes these constraints |
| `FailureModeAffectsBehavior` | failure_mode | behavior (software) | This failure mode affects these behaviors |

### Notes

- All cross-extension edges use soft references. If `@specforge/software` or `@specforge/product` is not installed, references to their entity kinds produce `I004` diagnostics.
- The `failure_mode` kind requires RPN (Risk Priority Number) validation: severity x occurrence x detection = declared rpn (E005).

## @specforge/formal

**Depends on: @specforge/software** (peer dependency). Provides formal analysis constructs: structured conditions, specification layering, event graph linting, and coverage tracking.

### Entity Kinds (5)

| Kind | Keyword | Singleton | Description |
|------|---------|-----------|-------------|
| Property | `property` | no | Temporal or behavioral assertion (safety, liveness, fairness) |
| Axiom | `axiom` | no | Assumed-true foundation (no proof required, no coverage tracking) |
| Protocol | `protocol` | no | Shared synchronization contract across events |
| Refinement | `refinement` | no | Abstract-to-concrete behavior mapping with condition deltas |
| Process | `process` | no | CSP-style communicating process with alphabet and composition |

### Edge Types (12)

| Edge Type | Source | Target | Semantics |
|-----------|--------|--------|-----------|
| `BehaviorRequiresInvariant` | behavior (software) | invariant (software) | This behavior requires this invariant as precondition |
| `BehaviorEnsuresInvariant` | behavior (software) | invariant (software) | This behavior ensures this invariant as postcondition |
| `BehaviorMaintainsInvariant` | behavior (software) | invariant (software) | This behavior maintains this invariant as frame condition |
| `AxiomAssumesInvariant` | axiom | invariant (software) | This axiom assumes this invariant |
| `BehaviorSatisfiesProperty` | behavior (software) | property | This behavior satisfies this property |
| `EventFollowsProtocol` | event (software) | protocol | This event follows this protocol |
| `PropertyDependsOnInvariant` | property | invariant (software) | This property depends on this invariant |
| `RefinementRefinesAbstract` | refinement | behavior (software) | This refinement maps from this abstract behavior |
| `RefinementRefinesConcrete` | refinement | behavior (software) | This refinement maps to this concrete behavior |
| `RefinementChainsToRefinement` | refinement | refinement | This refinement chains to that refinement |
| `EventParticipatesInProcess` | event (software) | process | This event participates in this process |
| `ProcessComposesProcess` | process | process | This process composes that process |

### Entity Enhancements

| Target Kind | Owner Extension | Added Fields |
|-------------|----------------|-------------|
| `behavior` | @specforge/software | `requires` (reference_list -> invariant), `ensures` (reference_list -> invariant), `maintains` (reference_list -> invariant), `satisfies` (reference_list -> property), `sync` (block) |
| `event` | @specforge/software | `follows_protocol` (reference_list -> protocol), `participates_in` (reference_list -> process), `sync` (block) |

### Compiler Passes (4)

| Pass | After | Description |
|------|-------|-------------|
| `condition_check` | resolve | Validate structured condition consistency |
| `layering_verify` | condition_check | Verify specification layering constraints |
| `event_graph_analyze` | resolve | Analyze event graph for synchronization issues |
| `coverage_tracking` | layering_verify | Track coverage of formal verification obligations |

### Notes

- Requires `warning_level=strict` in `specforge.json` for formal warnings to fire.
- Structured conditions are inline blocks (requires/ensures/maintains) within behavior bodies. Conditions are not standalone entities; shared constraints are modeled as invariant entities.
- Cycle detection on refinement chains (E041) and process composition (E042) prevents infinite layering.

## @specforge/software-testing

**Depends on: @specforge/software, @specforge/product.** Enhancement-only extension that adds the `gherkin` field and test result collectors to entity kinds across multiple extensions.

### Entity Kinds (0)

This extension declares no entity kinds of its own. It exists solely to enhance other extensions' entity kinds with testing capabilities.

### Edge Types (1)

| Edge Type | Source | Target | Semantics |
|-----------|--------|--------|-----------|
| `TestedBy` | any enhanced kind | test file | This entity is tested by these files |

### Entity Enhancements (13)

The `gherkin` field (`string_list`, `file_reference = true`) is added to 13 entity kinds across four extensions:

**Direct enhancements** (peer dependencies installed):

| Target Kind | Owner Extension |
|-------------|----------------|
| `behavior` | @specforge/software |
| `invariant` | @specforge/software |
| `event` | @specforge/software |
| `type` | @specforge/software |
| `port` | @specforge/software |
| `feature` | @specforge/product |
| `deliverable` | @specforge/product |
| `milestone` | @specforge/product |

**Soft reference enhancements** (enhancement applies if extension installed, `I004` otherwise):

| Target Kind | Owner Extension |
|-------------|----------------|
| `constraint` | @specforge/governance |
| `property` | @specforge/formal |
| `protocol` | @specforge/formal |
| `process` | @specforge/formal |
| `refinement` | @specforge/formal |

### Validation Rules

| Code | Severity | Rule |
|------|----------|------|
| W004 | warning | Entity has `gherkin` field but no files referenced |

### Collectors

| Collector | Formats | Auto-detect |
|-----------|---------|-------------|
| Gherkin/Cucumber | junit-xml, json | `**/cucumber-report.json`, `**/cucumber-report.xml` |

### Notes

- Gherkin is the sole testing mechanism. There are no `verify` blocks in the DSL.
- Testing was extracted from `@specforge/software` to follow Principle 7 (extensions over built-ins). Projects that do not use BDD testing can skip this extension entirely.
- The `gherkin` field's `file_reference = true` enables the host to validate that referenced `.feature` files exist on disk.

## What Was Removed (Entity Audit)

The entity audit identified four redundant entity kinds and one DSL construct that were removed or retargeted:

| Removed | Reason | Replacement |
|---------|--------|-------------|
| `library` | Identical fields to module (family, features, depends_on) | Use `module` |
| `capability` | Identical fields to journey (persona, channels, features, flow) | Use `journey` |
| `roadmap` | Covered by milestone + release (planning via milestones, shipping via releases) | Use `milestone` and `release` |
| `condition` | Retargeted to invariant; edges renamed `*Condition` to `*Invariant` | Use `invariant` with requires/ensures/maintains references |
| `verify` blocks | Removed from DSL entirely | Use `gherkin` files via `@specforge/software-testing` |

## Entity Count Summary

| Extension | Entity Kinds | Edge Types | Enhancements | Dependencies |
|-----------|-------------|------------|--------------|-------------|
| @specforge/product | 9 | 20 | -- | none |
| @specforge/software | 5 | 14 | module, milestone | product |
| @specforge/governance | 3 | 11 | -- | software |
| @specforge/formal | 5 | 12 | behavior, event | software |
| @specforge/software-testing | 0 | 1 | 13 entity kinds | software, product |
| **Total** | **22** | **58** | -- | -- |

## Related Documentation

- [Extension Protocol](extension-protocol.md) -- the Wasm protocol extensions implement
- [Extension SDK](extension-sdk.md) -- the SDK for building extensions
- [Extension Model](extension-model.md) -- the broader extension architecture
- [Entity Model](entity-model.md) -- entity kinds, edge types, and validation rules
