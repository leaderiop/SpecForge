# Extension Inventory

What you get from this page: the catalogue of the extensions that ship with SpecForge, and exactly what each one
declares. They are a **starter set**: examples of what extensions do, aimed at software and product work. None of
them is part of the core, a project loads only the ones it lists in `specforge.json`, and your own extensions use
the same SDK ([extension model](extension-model.md)).

Together the four vocabulary builtins declare 22 entity kinds and 56 edge types. Five more declare no kinds:
`@specforge/testing` and the runner extensions `@specforge/cargo-test` and `@specforge/vitest` handle test
obligations and results ([ADR 0002](adr/0002-test-runner-extensions.md)); `@specforge/rust` and
`@specforge/typescript` read source code for `specforge infer`.

The counts and lists on this page are read from the builtins' pinned declarations
(`crates/runtime/component/tests/declarations/`); tests hold the kind count and the edge tables to them.

## At a glance

| Extension | Kinds | Edge types | Adds to other kinds | Peers |
|-----------|-------|------------|---------------------|-------|
| `@specforge/product` | 9 | 20 | -- | none |
| `@specforge/software` | 4 | 15 | `module`, `milestone` | product, governance (both optional) |
| `@specforge/governance` | 4 | 11 | -- | software, product (both optional) |
| `@specforge/formal` | 5 | 10 | `behavior`, `event`, `invariant` | software, governance (both optional) |
| `@specforge/testing` | 0 | 0 | `verify` kinds on 7 kinds | software, governance (both optional) |
| `@specforge/cargo-test` | 0 | 0 | -- | testing (required) |
| `@specforge/vitest` | 0 | 0 | -- | testing (required) |
| `@specforge/rust` | 0 | 0 | -- | none |
| `@specforge/typescript` | 0 | 0 | -- | none |

Extensions load after the peers they declare; extensions with no peer between them keep the order
`specforge.json` lists them in (ADR 0041). A required peer must be loaded (E027); an optional one, when loaded,
must be in the declared version range.

## In progress and not started

| Family | State |
|--------|-------|
| `@gsd/*` (the GSD spec framework modelled as extensions) | Being designed; nothing built yet |
| BMAD, OpenSpec, SpecKit | Not started |

## @specforge/product

Product planning and delivery: what is built, for whom, through which channels, when it ships. Nothing in it is
software-only; a clinic, a course or a campaign has features, personas, journeys and milestones too.

### Entity kinds (9)

| Kind | Description |
|------|-------------|
| [`feature`](entities/feature.md) | A unit of value, with problem and solution framing |
| [`journey`](entities/journey.md) | One persona's path through some features, on some channels |
| [`deliverable`](entities/deliverable.md) | Something that ships, bundling journeys and modules |
| [`milestone`](entities/milestone.md) | A planning phase with scheduled features and exit criteria |
| [`module`](entities/module.md) | A structural unit (package, subsystem, workstream) that builds features |
| [`term`](entities/term.md) | A glossary entry |
| [`persona`](entities/persona.md) | A kind of person who uses what you build |
| [`channel`](entities/channel.md) | The medium people reach it through |
| [`release`](entities/release.md) | Deliverables and milestones that ship together |

### Edge types (20)

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

### Also declares

- A shared field, `tags` (string list), on all nine kinds.
- 47 rule declarations (unreferenced entities, dependency cycles, missing owners, missing reasons, non-SemVer
  versions, ...), each listed on its kind's page.
- Two passes: `lifecycle` (every compile: a completed milestone delivering a feature that is not done, W154, and
  similar contradictions across entities) and `delivery_evidence` (`specforge analyze`: a `done` feature the
  recorded tests do not prove, I071).
- 40 commands, `specforge product <command>` on the CLI and the same names as MCP tools: the list commands
  (`features`, `journeys`, ...) and graph queries such as `milestone-completion`, `critical-path`,
  `deliverable-traceability`, `owner-workload`, `coverage-matrix` and `health`. `specforge product --help` lists
  them all.

## @specforge/software

Software design: behavioral contracts, domain events, data shapes and interfaces. Its product links
(`features`, and the fields it adds to `module` and `milestone`) need `@specforge/product`; without it they are
I004 advice and the additions are skipped.

### Entity kinds (4)

| Kind | Description |
|------|-------------|
| [`behavior`](entities/behavior.md) | One operation's contract |
| [`event`](entities/event.md) | Something the system announces |
| [`type`](entities/type.md) | A data shape (struct or union) |
| [`port`](entities/port.md) | An interface contract with typed methods |

### Edge types (15)

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
| `PortReferencesType` | port | type | This port names this type in a method parameter or return type |
| `ExternalRef` | any | URI | This entity links to an external reference |
| `MilestoneIncludesBehavior` | milestone (product) | behavior | This milestone delivers these behaviors |
| `ModuleConsumesPort` | module (product) | port | This module consumes these ports |
| `ModuleDefinesPort` | module (product) | port | This module defines these ports |

### Adds to other kinds

| Kind | Owner | Added fields |
|------|-------|--------------|
| `module` | `@specforge/product` | `ports` (reference list → port), `ports_defined` (reference list → port) |
| `milestone` | `@specforge/product` | `behaviors` (reference list → behavior) |

### Also declares

Eight rules (W001, W002, W005–W008, W010, E010). Two are custom validators written in the extension's
code: milestone behavior references (E010) and type member annotations (W010). The host reports a type name that
names no type (E004). A starter `.spec` file `specforge init` writes.

## @specforge/governance

Rules that must always hold, decisions, measurable limits and risk. Its links into software and product kinds are optional: without those
extensions they are I004 advice.

### Entity kinds (4)

| Kind | Description |
|------|-------------|
| [`invariant`](entities/invariant.md) | A rule that must always hold |
| [`decision`](entities/decision.md) | A recorded choice with context, alternatives and consequences |
| [`constraint`](entities/constraint.md) | A measurable quality or limit, with metric and threshold |
| [`failure_mode`](entities/failure-mode.md) | An FMEA entry: cause, effect, ratings, mitigation |

### Edge types (11)

| Edge Type | Source | Target | Semantics |
|-----------|--------|--------|-----------|
| `DecisionProtectsInvariant` | decision | invariant | This decision protects these invariants |
| `ConstraintEnforcedByBehavior` | constraint | behavior (software) | This constraint applies to these behaviors |
| `DecisionSupersedesDecision` | decision | decision | This decision supersedes that decision |
| `ConstraintConstrainsBehavior` | constraint | behavior (software) | This quality requirement constrains these behaviors |
| `ConstraintProtectsInvariant` | constraint | invariant | This constraint protects these invariants |
| `FailureModeTargetsInvariant` | failure_mode | invariant | This failure mode threatens this invariant |
| `ConstraintGovernsFeature` | constraint | feature (product) | This constraint governs these features |
| `DecisionAffectsFeature` | decision | feature (product) | This decision affects these features |
| `FailureModeThreatensFeature` | failure_mode | feature (product) | This failure mode threatens these features |
| `DecisionImposesConstraint` | decision | constraint | This decision imposes these constraints |
| `FailureModeAffectsBehavior` | failure_mode | behavior (software) | This failure mode affects these behaviors |

`failure_mode` declares `severity`, `occurrence` and `detection` (and their `post_` forms) as word vocabularies
(E061 otherwise); a written `rpn` is not checked against them.

One rule (W003): an invariant nothing references.

## @specforge/formal

Formal methods: temporal properties, assumptions, protocols, refinement between specification layers, and
communicating processes. Requires `@specforge/software`. See [Formal verification](guides/formal-verification.md).

### Entity kinds (5)

| Kind | Description |
|------|-------------|
| `property` | A temporal or behavioral assertion (safety, liveness, ...) |
| `axiom` | Something assumed true without proof |
| `protocol` | A synchronization contract events share |
| `refinement` | A mapping from an abstract behavior to a concrete one |
| `process` | A CSP-style communicating process with an alphabet |

### Edge types (10)

| Edge Type | Source | Target | Semantics |
|-----------|--------|--------|-----------|
| `AxiomAssumesInvariant` | axiom | invariant | This axiom assumes this invariant |
| `BehaviorSatisfiesProperty` | behavior (software) | property | This behavior satisfies this property |
| `BehaviorRefinesBehavior` | behavior (software) | behavior (software) | This concrete behavior refines this abstract behavior |
| `EventFollowsProtocol` | event (software) | protocol | This event follows this protocol |
| `PropertyDependsOnInvariant` | property | invariant | This property depends on this invariant |
| `RefinementRefinesAbstract` | refinement | behavior (software) | This refinement maps from this abstract behavior |
| `RefinementRefinesConcrete` | refinement | behavior (software) | This refinement maps to this concrete behavior |
| `RefinementChainsToRefinement` | refinement | refinement | This refinement chains to that refinement |
| `EventParticipatesInProcess` | event (software) | process | This event participates in this process |
| `ProcessComposesProcess` | process | process | This process composes that process |

### Adds to other kinds

| Kind | Owner | Added fields |
|------|-------|--------------|
| `behavior` | `@specforge/software` | `requires`, `ensures`, `maintains`, `sync` (blocks), `satisfies` (reference list → property), `abstract` (bool), `refines` (reference → behavior) |
| `event` | `@specforge/software` | `follows_protocol` (reference list → protocol), `participates_in` (reference list → process), `sync` (block) |
| `invariant` | `@specforge/governance` | `expression` (string) |

### Passes (5)

| Pass | After | When |
|------|-------|------|
| `analysis_available` | resolve | Every compile |
| `condition_check` | resolve | `specforge analyze` |
| `layering_verify` | condition_check | `specforge analyze` |
| `event_graph_analyze` | layering_verify | `specforge analyze` |
| `coverage_tracking` | event_graph_analyze | `specforge analyze` |

Refinement chains (E041) and process composition (E042) must have no cycle.

## @specforge/testing and the runner extensions

`@specforge/testing` gives `verify [kind] "..."` lines their meaning ([ADR 0002](adr/0002-test-runner-extensions.md)):
the core grammar parses them, and this extension says which kinds owe test obligations and which verify kinds each
accepts. It declares no kinds and no edges.

### Verify kinds it adds (7 kinds)

An addition to a kind whose extension is not loaded is skipped silently.

| Kind | Owner | Verify kinds | W004 when it has none |
|------|-------|--------------|-----------------------|
| `behavior` | `@specforge/software` | unit, contract, integration, property, performance | yes |
| `invariant` | `@specforge/governance` | unit, integration, property, performance, mutation | yes |
| `event` | `@specforge/software` | integration, unit, deadlock_free, liveness | yes |
| `type` | `@specforge/software` | unit, property | yes |
| `port` | `@specforge/software` | integration, unit | yes |
| `constraint` | `@specforge/governance` | unit, integration, property, load, contract | no |
| `failure_mode` | `@specforge/governance` | unit, integration, property | no |

W009 is a verify kind outside a kind's list.

### The coverage pass

`coverage` (`specforge analyze coverage`) scores intent (A001/A002), counts enforcement, and reads proof from the
results `specforge collect` records: failing tests (A014), obligations no passing test names (A015), tests naming
obligations the spec does not declare (A016). `--min` gates CI on proof coverage.

### Runner extensions

Each test runner has its own extension that requires `@specforge/testing` and declares one collector: the files
that select it, the command `specforge collect` runs (after you approve it, or with `--yes`), where the report
lands, and a pure mapping from the report to entities.

| Extension | Detected by | How a test names its entity |
|-----------|-------------|-----------------------------|
| `@specforge/cargo-test` | `Cargo.toml` | `#[specforge_test(behavior = "...", verify = "...")]`, or tests named `entity_id__obligation_slug` or inside `mod entity_id` ([guide](guides/rust-test-tracing.md)) |
| `@specforge/vitest` | `vitest.config.*`, `vitest.workspace.*` | `meta: { specforge: { behavior: "...", verify: "..." } }` ([guide](guides/vitest-test-tracing.md)) |

Spec files carry no test paths: the link lives in the tests. `specforge init` enables `@specforge/testing` with
`@specforge/software`, and a runner extension when it finds that runner in the project.

Proof today is test-centred. A general notion of evidence for non-software work (a signed record, an audit
finding) is future work.

## @specforge/rust and @specforge/typescript

Each declares one analyzer: a scanner that finds the public items (Rust) or exported symbols (TypeScript and
JavaScript) of source files, which `specforge infer` uses to guide an agent writing specs for existing code.

## Related

- [Extension model](extension-model.md): what an extension can declare and how it runs
- [Entity model](entity-model.md): the structural core and the builtin kinds
- [Extension SDK](extension-sdk.md) and [Extension protocol](extension-protocol.md): building one
