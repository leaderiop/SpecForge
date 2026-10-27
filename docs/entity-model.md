# Entity Model

What you get from this page: how SpecForge represents what you know (things, their links, the rules they obey)
and why none of it is built into the core. If you only remember one thing: **SpecForge's core has no domain.**
Every kind of thing it knows about, a behavior, a persona, an SOP, a campaign, comes from an extension.

## The core knows five keywords

The grammar knows exactly five structural keywords
([`crates/kernel/common/src/structural.rs`](../crates/kernel/common/src/structural.rs)):

| Keyword | What it is |
|---------|------------|
| `spec` | The project's root entity (it names the project; it configures nothing: [spec](entities/spec.md)) |
| `ref` | A link to something outside the graph: `ref gh.issue:42 "..."` ([ref](entities/ref.md)) |
| `use` | An import of another `.spec` file |
| `define` | Reserved; registers nothing (W143) |
| `defaults` | Field values every block of one kind in a file starts with: `defaults behavior { category users }` |

Two of them, `spec` and `ref`, become graph nodes. Everything else you write, `behavior create_user "..." { ... }`,
`persona patient "..." { ... }`, is a block whose keyword an **extension** declares. The parser reads any
`keyword id "title" { ... }` block the same way; the loaded extensions decide which keywords are kinds, which fields
each kind has, which fields are links to which other kinds, and which rules apply. A keyword no loaded extension
declares is E024.

So the question "what entity types does SpecForge have?" has no fixed answer. It has the kinds your project's
extensions declare. Change the extensions and the vocabulary changes, with no change to SpecForge itself.

## The builtin kinds: a starter set

SpecForge ships builtin extensions so a project has a vocabulary on day one. They are examples of what extensions
do, aimed at software and product work, not a base every project needs. The kinds they declare number
22, declared by the four builtin extensions below:

| Extension | Kinds it declares | For |
|-----------|-------------------|-----|
| `@specforge/software` | [behavior](entities/behavior.md), [event](entities/event.md), [type](entities/type.md), [port](entities/port.md) | Software design: contracts, events, data shapes, interfaces |
| `@specforge/product` | [feature](entities/feature.md), [journey](entities/journey.md), [deliverable](entities/deliverable.md), [milestone](entities/milestone.md), [module](entities/module.md), [term](entities/term.md), [persona](entities/persona.md), [channel](entities/channel.md), [release](entities/release.md) | Product planning, in software or anywhere else |
| `@specforge/governance` | [invariant](entities/invariant.md), [decision](entities/decision.md), [constraint](entities/constraint.md), [failure_mode](entities/failure-mode.md) | Rules that must always hold, decisions, measurable limits, risk (FMEA) |
| `@specforge/formal` | `property`, `axiom`, `protocol`, `refinement`, `process` | Formal methods ([guide](guides/formal-verification.md)) |

With the two structural node kinds that makes 2 + 22 = 24 entity kinds in a project that loads all four. Other
builtins declare no kinds: `@specforge/testing` makes kinds testable (`verify` lines), `@specforge/cargo-test` and
`@specforge/vitest` collect test results, `@specforge/rust` and `@specforge/typescript` read source code for
`specforge infer`. The [extension inventory](extension-inventory.md) lists what each one declares.

A pharma team, a marketing team or a team using another spec framework writes its own extension and gets its own
kinds: `sop`, `batch_record`, `campaign`, `audience`. See the [extension model](extension-model.md).

```bash
specforge init                                         # structural core only: spec and ref
specforge init --extensions @specforge/product         # a project with the product kinds
specforge add @specforge/software                      # add a builtin
specforge add ./my-kinds.wasm                          # or your own extension, as a .wasm file
specforge remove @specforge/governance
specforge extensions                                   # what is enabled and loaded
```

## Anatomy of an entity

```spec
behavior create_user "Create User" {
  invariants [email_uniqueness]
  category   users
  contract   "When a valid request arrives, the system MUST create the user with a unique email."

  verify unit "rejects a duplicate email"
}

invariant email_uniqueness "Email Uniqueness" {
  guarantee "No two active users share the same email address."
}
```

- **Keyword** (`behavior`): the kind, declared by an extension.
- **Id** (`create_user`): unique within its scope, whatever the kind (E002, W060): the entity its kind's scope field names, or the project; its path is its scope's path and its name (`v1.0/03/D-01`). 2 to 60 characters (E014),
  made of words of letters, digits and `_`, joined by single `-` or `.` (`create_user`, `AUTH-01`, `v1.0`, `03`). An id may not be a structural keyword or any declared kind (E013).
- **Title** (`"Create User"`): required (E076). It is the one label every surface shows. `specforge migrate` writes
  missing titles.
- **Fields**: `name value` lines. The kind's declaration says which names exist (W020 otherwise), each one's type,
  and which are required (E006).
- **Statements**: `verify [kind] "..."` lines (test obligations; `@specforge/testing` says which kinds a kind
  accepts) and, for kinds that take them, `method name(...) -> Type` lines (`port`).

### Field types

Extensions declare each field with one of twelve types (`specforge_protocol_types::FieldType`):

| Type | Value | Checked |
|------|-------|---------|
| `string` | quoted text, or `"""` text over several lines | |
| `integer` | a whole number | |
| `bool` | `true` / `false` | |
| `date` | one calendar day, `2026-06-30`, written bare | |
| `enum` | one of the values the field lists | E061 otherwise |
| `enum_list` | `[low, high]`, words the field lists | E061 for another word or a repeat |
| `identifier` | one bare word from an open set | |
| `string_list` | `["a", "b"]` | |
| `reference` | one entity id | must resolve (E003), to the right kind (E022) |
| `reference_list` | `[id, id]` | each must resolve (E003), to the right kind (E022) |
| `block` | a nested `{ ... }` block; with declared members, each is read by its own type | read by the extension's passes; E061 and E006 per member |
| `block_list` | `[ { a 1 } { a 2 } ]`, a list of blocks with the same declared members | E061 and E006 per member of each item |

A reference may name any of several kinds (`target_kinds`), an integer may be bounded (`minimum`, `maximum`), and a
`file_reference` field reads its paths from the spec root or, declaring `file_base project`, the project root.
A field can also declare an `edge` (its references become typed graph edges), `required`, `file_reference` (each
value is a path that must exist), `normative` (it states what the entity promises; the review document quotes it
whole) and `headline` (it is the entity's one-line statement, as a behavior's `contract` is). The core knows none of
these fields by name: it reads the flags. `specforge model` renders the declared vocabulary of a project
([model docs](model/README.md)); the [quick reference](quick-reference.md) lists every builtin field and relation.

### A relation is written on one side

Each relation has an owning field: a behavior lists the invariants it enforces (`invariants`); the invariant does
not list its behaviors. The other side is derived: `specforge trace` and the queries walk it. Writing the other
side's name is E079, which names the owning field to use instead.

### References across extensions

A reference field may point at a kind another extension declares: `behavior.features` points at
`@specforge/product`'s `feature`. When that extension is not loaded, the reference is I004 advice (the kind is
missing), not an error; load it and the reference is checked like any other.

## Edge types

The builtin extensions declare these edge types. Each is written by the field the [entity pages](entities/) and
the quick reference name.

### @specforge/software Edges (15)

| Edge Type | From | To | Semantics |
|-----------|------|----|-----------|
| `ExternalRef` | any | any | Entity references an external URI or resource |
| `References` | any | any | General cross-reference between entities |
| `BehaviorConsumesEvent` | behavior | event | This behavior consumes these events |
| `BehaviorEnforcesInvariant` | behavior | invariant | This behavior enforces these invariants |
| `BehaviorImplementsFeature` | behavior | feature | This behavior implements these features |
| `BehaviorProducesEvent` | behavior | event | This behavior emits these events |
| `BehaviorReferencesType` | behavior | type | This behavior uses these types |
| `BehaviorUsesPort` | behavior | port | This behavior uses these ports |
| `EventCarriesPayloadType` | event | type | This event has as its payload these types |
| `MilestoneIncludesBehavior` | milestone | behavior | This milestone delivers these behaviors |
| `ModuleConsumesPort` | module | port | This module consumes these ports |
| `ModuleDefinesPort` | module | port | This module defines these ports |
| `PortReferencesType` | port | type | This port references these types |
| `TypeComposesType` | type | type | This type composes these types |
| `TypeExtendsType` | type | type | This type extends these types |

### @specforge/product Edges (20)

| Edge Type | From | To | Semantics |
|-----------|------|----|-----------|
| `DeliverableContainsModule` | deliverable | module | This deliverable contains these modules |
| `DeliverableDependsOn` | deliverable | deliverable | This deliverable depends on these deliverables |
| `DeliverableSupportsJourney` | deliverable | journey | This deliverable ships these journeys |
| `DeliverableTrackedByMilestone` | deliverable | milestone | This deliverable is tracked by these milestones |
| `FeatureDependsOn` | feature | feature | This feature depends on these features |
| `FeatureRelatesTo` | feature | feature | This feature relates to these features |
| `JourneyExercisesFeature` | journey | feature | This journey exercises these features |
| `JourneyTargetsPersona` | journey | persona | This journey is performed by these personas |
| `JourneyUsesChannel` | journey | channel | This journey uses these channels |
| `MilestoneDeliversFeature` | milestone | feature | This milestone schedules these features |
| `MilestoneDependsOn` | milestone | milestone | This milestone depends on these milestones |
| `MilestoneScopesModule` | milestone | module | This milestone includes these modules |
| `ModuleContainsFeature` | module | feature | This module implements these features |
| `ModuleDependsOn` | module | module | This module depends on these modules |
| `PersonaPrioritizesFeature` | persona | feature | This persona prioritizes these features |
| `ReleaseCompletesMilestone` | release | milestone | This release targets these milestones |
| `ReleaseDependsOn` | release | release | This release depends on these releases |
| `ReleaseIncludesDeliverable` | release | deliverable | This release ships these deliverables |
| `TermBelongsToModule` | term | module | This term belongs to these modules |
| `TermReferencesRelatedTerm` | term | term | This term cross-references these terms |

### @specforge/governance Edges (11)

| Edge Type | From | To | Semantics |
|-----------|------|----|-----------|
| `ConstraintConstrainsBehavior` | constraint | behavior | This constraint constrains these behaviors |
| `ConstraintEnforcedByBehavior` | constraint | behavior | This constraint is enforced by these behaviors |
| `ConstraintGovernsFeature` | constraint | feature | This constraint governs these features |
| `ConstraintProtectsInvariant` | constraint | invariant | This constraint protects these invariants |
| `DecisionAffectsFeature` | decision | feature | This decision affects these features |
| `DecisionImposesConstraint` | decision | constraint | This decision imposes these constraints |
| `DecisionProtectsInvariant` | decision | invariant | This decision protects these invariants |
| `DecisionSupersedesDecision` | decision | decision | This decision supersedes these decisions |
| `FailureModeAffectsBehavior` | failure_mode | behavior | This failure mode affects these behaviors |
| `FailureModeTargetsInvariant` | failure_mode | invariant | This failure mode targets these invariants |
| `FailureModeThreatensFeature` | failure_mode | feature | This failure mode threatens these features |

### @specforge/formal Edges (10)

| Edge Type | From | To | Semantics |
|-----------|------|----|-----------|
| `AxiomAssumesInvariant` | axiom | invariant | This axiom assumes these invariants |
| `BehaviorRefinesBehavior` | behavior | behavior | This behavior refines these behaviors |
| `BehaviorSatisfiesProperty` | behavior | property | This behavior satisfies these properties |
| `EventFollowsProtocol` | event | protocol | This event follows these protocols |
| `EventParticipatesInProcess` | event | process | This event participates in these processes |
| `ProcessComposesProcess` | process | process | This process composes these processes |
| `PropertyDependsOnInvariant` | property | invariant | This property depends on these invariants |
| `RefinementChainsToRefinement` | refinement | refinement | This refinement chains to these refinements |
| `RefinementRefinesAbstract` | refinement | behavior | This refinement maps from these behaviors |
| `RefinementRefinesConcrete` | refinement | behavior | This refinement maps to these behaviors |

## Validation

Each rule belongs to the extension that declares it and runs only when that extension is loaded; core adds the
structural checks every kind gets (references resolve, ids are unique, required fields are present, values have
their declared types, blocks have titles). Every code, its meaning and its owner (`core` or the emitting
extension) is in [diagnostics.md](diagnostics.md), generated from the `specforge explain` catalog. Codes
`E900`–`E998`, `W900`–`W998` and `I900`–`I998` are reserved for third-party extensions.

## What belongs in the graph

Put something in the graph when it has links worth checking: when other things depend on it, point at it, or
must agree with it. Long narratives with no links (a pitch, a research note) gain nothing from being entities.
Some views are never written by hand: `specforge trace`, `specforge export` and `specforge model` derive them from
the graph.

## Design principles

1. **No domain in the core.** All vocabulary comes from extensions. If a new domain needed a compiler change, the
   design would have failed.
2. **Every kind earns its place.** Each answers a question no other kind answers.
3. **References are checked.** Ids are typed and resolved on every compile; a reference to a kind whose extension
   is not loaded is advice (I004), not a failure.
4. **One source for each relation.** It is written on its owning side; the other side is derived.
5. **Start small.** Begin with the structural core or one extension; add extensions as the project grows.
