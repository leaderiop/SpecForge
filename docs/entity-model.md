# SpecForge Entity Model

## Overview

SpecForge uses a **zero-entity core** architecture: the compiler is a pure typed-graph engine with zero domain knowledge. Two **structural kinds** (`spec` and `ref`) are parsed by the core grammar. All domain entity kinds (currently 23) come from **extensions** via ManifestV2 declarations.

Four official extensions provide the domain vocabulary:
- **@specforge/software** (5 kinds): behavior, invariant, event, type, port
- **@specforge/product** (9 kinds): journey, deliverable, milestone, module, term, feature, persona, channel, release
- **@specforge/governance** (3 kinds): decision, constraint, failure_mode
- **@specforge/formal** (5 kinds): property, axiom, protocol, refinement, process — plus structured conditions (inline requires/ensures/maintains fields), specification layering, event graph linting, coverage tracking via entity_enhancements on @specforge/software entities

Total: 2 structural + 22 domain = 24 entity kinds (no budget cap).

Every entity has a unique ID, compiler-checked cross-references, and a defined role in the traceability chain. Teams adopt only what they need — start with structural kinds, add extensions as projects grow.

## Architecture: Structural Core + Extensions

```
┌──────────────────────────────────────────────────────────────┐
│                    STRUCTURAL (2 kinds)                       │
│  spec (singleton config) · ref (external references)         │
│  + meta-schema `define` mechanism                            │
│  + zero-entity core: ANY keyword parsed, extensions validate │
├──────────────────────────────┬───────────────────────────────┤
│  @specforge/software (5)     │  @specforge/governance (3)    │
│  behavior · invariant        │  decision · constraint        │
│  event · type · port         │  failure_mode                 │
├──────────────────────────────┤                               │
│  @specforge/product (9)      │                               │
│  journey · deliverable       │                               │
│  milestone · module · term   │                               │
│  feature · persona · channel │                               │
│  release                     │                               │
├──────────────────────────────┘                               │
│  @specforge/formal (5 kinds: property, axiom, protocol,      │
│  refinement, process) · structured conditions (inline) ·     │
│  specification layering · event graph linting · coverage     │
│  tracking · 8 edge types · 4 compiler passes · 3 feature    │
│  flags                                                       │
└──────────────────────────────────────────────────────────────┘
```

### Why This Split

**Structural core** contains two kinds parsed by the core grammar:
- `spec` — singleton project configuration (name, version, extensions, providers)
- `ref` — external resource references with scheme-based routing

The core grammar parses ANY `keyword name { fields }` block generically. Validation of which keywords are legal, what fields are allowed, and what edges exist comes entirely from extensions via ManifestV2. If a new domain requires a compiler change, the architecture has failed.

**@specforge/software** is a recommended-by-default extension (like Terraform's built-in providers). It adds the software engineering domain: behavioral contracts (`behavior → invariant`), the domain event model (`event`), and the code bridge (`type` + `port`). All 5 entity kinds are testable with verify support.

**@specforge/product** adds product planning and delivery entities. Not every project ships a product — internal tools, scripts, and libraries don't need journeys, deliverables, or milestones. This extension extends the chain upward (`deliverable -> journey -> feature`) and adds the structural bridge (`module -> feature`), temporal dimension (`milestone`), domain-neutral feature grouping (`feature`), user modeling (`persona`), interaction medium modeling (`channel`), and glossary (`term`).

**@specforge/governance** adds architecture governance, quality tracking, and risk assessment. Not every project formalizes ADRs, NFRs, or FMEA — early-stage startups and prototypes rarely do. This extension adds overlay entities that reference software entities (`decision → invariant`, `constraint → behavior`, `failure_mode → invariant`).

### Extension CLI

```bash
specforge init                          # structural core only (2 kinds: spec, ref)
specforge add @specforge/software       # + 5 software entities (recommended)
specforge add @specforge/product        # + 8 product entities
specforge add @specforge/governance     # + 3 governance entities
specforge add @specforge/formal         # + 5 formal entities (property, axiom, protocol, refinement, process)
specforge remove @specforge/governance  # remove extension
specforge plugins                       # list installed extensions
```

`specforge init` offers interactive setup with @specforge/software pre-selected:

```
? Which extensions do you want? (space to select)
  ● @specforge/software    — behavior, invariant, event, type, port (recommended)
  ○ @specforge/product     — journey, deliverable, milestone, module, term, feature, persona, channel, release
  ○ @specforge/governance  — decision, constraint, failure_mode
  ○ @specforge/formal      — property, axiom, protocol, refinement, process + structured conditions (inline)
```

## Entity Summary

### Structural (Core)

| # | Entity | ID Pattern | Question It Answers |
|---|--------|-----------|---------------------|
| 1 | [spec](entities/spec.md) | singleton | What project is this? |
| 2 | [ref](entities/ref.md) | `scheme.kind:identifier` | What external resource is this connected to? |

### @specforge/software

| # | Entity | ID Pattern | Question It Answers |
|---|--------|-----------|---------------------|
| 3 | [behavior](entities/behavior.md) | `identifier` | What exactly does the system do? |
| 4 | [invariant](entities/invariant.md) | `identifier` | What must ALWAYS be true? |
| 5 | [event](entities/event.md) | `identifier` | What does the system announce? |
| 6 | [type](entities/type.md) | `identifier` | What shape does the data have? |
| 7 | [port](entities/port.md) | `identifier` | What contracts exist between components? |

### @specforge/product

| # | Entity | ID Pattern | Question It Answers |
|---|--------|-----------|---------------------|
| 8 | [feature](entities/feature.md) | `identifier` | What value does this deliver? |
| 9 | [journey](entities/journey.md) | `identifier` | How does the user experience this? |
| 10 | [deliverable](entities/deliverable.md) | `identifier` | What ships to users? |
| 11 | [milestone](entities/milestone.md) | `identifier` | When does this ship? |
| 12 | [module](entities/module.md) | `identifier` | What component delivers this? |
| 13 | [term](entities/term.md) | `identifier` | What does this term mean? |
| 14 | [persona](entities/persona.md) | `identifier` | Who uses the system? |
| 15 | [channel](entities/channel.md) | `identifier` | Through which medium? |
| 16 | [release](entities/release.md) | `identifier` | What coordinated shipment is this? |

### @specforge/governance

| # | Entity | ID Pattern | Question It Answers |
|---|--------|-----------|---------------------|
| 17 | [decision](entities/decision.md) | `identifier` | Why was this built this way? |
| 18 | [constraint](entities/constraint.md) | `identifier` | What quality must the system achieve? |
| 19 | [failure_mode](entities/failure-mode.md) | `identifier` | What can go wrong and how bad is it? |

### @specforge/formal

| # | Entity | ID Pattern | Question It Answers |
|---|--------|-----------|---------------------|
| 20 | property | `identifier` | What temporal assertion must hold over time? |
| 21 | axiom | `identifier` | What is assumed true without proof? |
| 22 | protocol | `identifier` | What synchronization contract do events share? |
| 23 | refinement | `identifier` | What abstract-to-concrete mapping exists? |
| 24 | process | `identifier` | What communicating process do events participate in? |

#### @specforge/formal Entity & Edge Diagram

```
┌──────────────────────────────────────────────────────────────────────┐
│                    @specforge/formal (5 kinds, 8 edges)              │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─────────────────── Structured Conditions ──────────────────────┐  │
│  │                                                                │  │
│  │   ┌───────────┐                                               │  │
│  │   │ behavior  │  requires/ensures/maintains (inline fields)   │  │
│  │   │(enhanced) │  → ConditionEntry AST nodes                   │  │
│  │   │           │                                               │  │
│  │   │           │  Satisfies           ┌───────────┐            │  │
│  │   │           │─────────────────────▶│ property  │            │  │
│  │   └───────────┘                      │           │            │  │
│  │                                      └─────┬─────┘            │  │
│  │   ┌───────────┐  AssumedBy          PropertyDependsOn         │  │
│  │   │ invariant │────────────────┐           │                  │  │
│  │   │(enhanced) │                ▼           ▼                  │  │
│  │   └───────────┘           ┌───────┐  ┌───────────┐            │  │
│  │                           │ axiom │  │ invariant │            │  │
│  │                           └───────┘  └───────────┘            │  │
│  └────────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  ┌─────────────────── Specification Layering ─────────────────────┐  │
│  │                                                                │  │
│  │   ┌────────────┐  RefinesTo          ┌───────────┐            │  │
│  │   │ refinement │────────────────────▶│ behavior  │            │  │
│  │   │            │                     │(abstract) │            │  │
│  │   │            │                     └───────────┘            │  │
│  │   │            │  RefinementChainLink                         │  │
│  │   │            │────────────────────▶┌────────────┐           │  │
│  │   └────────────┘                     │ refinement │           │  │
│  │                                      └────────────┘           │  │
│  └────────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  ┌─────────────────── Event Graph Linting (CSP) ──────────────────┐  │
│  │                                                                │  │
│  │   ┌───────────┐  FollowsProtocol    ┌──────────┐             │  │
│  │   │   event   │────────────────────▶│ protocol │             │  │
│  │   │(enhanced) │                     └──────────┘             │  │
│  │   │           │  ParticipatesIn     ┌──────────┐             │  │
│  │   │           │────────────────────▶│ process  │             │  │
│  │   └───────────┘                     │          │             │  │
│  │                                     │          │             │  │
│  │                ProcessComposition   │          │             │  │
│  │                  ┌──────────┐──────▶│          │             │  │
│  │                  │ process  │       └──────────┘             │  │
│  │                  └──────────┘                                │  │
│  └────────────────────────────────────────────────────────────────┘  │
│                                                                      │
│  Entity kinds:  property · axiom · protocol · refinement · process   │
│  All: testable=false, supports_verify=false                          │
│  Conditions: inline fields (requires/ensures/maintains), not entities│
│  Errors:   E030-E035, E041 (refinement cycle), E042 (process cycle)  │
│  Warnings: W028-W040, W058, W061-W074                                │
│  Passes:   condition_check → layering_verify → event_graph_analyze   │
│            → coverage_tracking                                       │
└──────────────────────────────────────────────────────────────────────┘
```

## Cross-Extension References

When an entity references another entity from a different extension, the compiler uses **soft references** — a progressive enhancement model where spec files are valid with or without extensions installed.

### Resolution Rules

| Scenario | From | To | Behavior |
|----------|------|----|----------|
| **Same Extension** | extension entity | same extension entity | Always validated. |
| **Extension → Structural** | extension entity | spec/ref | Always validated. |
| **Cross-Extension** | extension entity | other extension entity | **Soft reference.** If target extension installed → validated (`E001` on miss). If not installed → `I004` info. |

### Diagnostic: I004 (Unknown Entity in Reference Field)

When a reference uses an identifier not found in any installed extension's entity registry:

```
info[I004]: Unknown entity 'use_postgresql' in field 'adrs'
  ┌─ behaviors/auth.spec:3:16
  │
3 │   adrs [use_postgresql]
  │         ^^^^^^^^^^^^^^ not found in installed extensions
  │
  = help: Install @specforge/governance to enable decision validation
```

### Example

```spec
// Structural core + @specforge/software only (no product/governance)
behavior create_user {
  invariants [data_persistence]    // ✅ Same extension: validated
  features   [user_management]     // ℹ️ I004: "Install @specforge/product"

  contract "..."
}
```

After `specforge add @specforge/product`:

```spec
// Same file — now product is installed
behavior create_user {
  invariants [data_persistence]    // ✅ Same extension: validated
  features   [user_management]     // ✅ Cross-extension: validated (E001 if not found)

  contract "..."
}
```

### Field-to-EntityKind Registry

The compiler uses the field name in which a reference appears to determine the expected target entity type. This replaces the prefix-based routing of the old ID system.

| Field Name | Target EntityKind | Extension |
|------------|-------------------|-----------|
| `invariants` | Invariant | @specforge/software |
| `enforces` | Invariant | @specforge/software |
| `types` | Type | @specforge/software |
| `ports` | Port | @specforge/software |
| `produces` | Event | @specforge/software |
| `trigger` | Behavior | @specforge/software |
| `features` | Feature | @specforge/product |
| `tests` | (file reference) | @specforge/software |
| `fieldType` | Type | @specforge/software |
| `refs` | Ref | core (structural) |
| `journeys` | Journey | @specforge/product |
| `modules` | Module | @specforge/product |
| `depends_on` | Module | @specforge/product |
| `adrs` | Decision | @specforge/governance |
| `invariant` (singular) | Invariant | @specforge/software |
| `affects` | Behavior | @specforge/software |

When the target kind's extension is not installed, the compiler emits `I004` instead of `E001`.

## Naming Conventions

Any valid identifier is accepted for all entity kinds. There is no enforced case convention.

```ebnf
identifier = letter , { letter | digit | "_" } ;   (* 2-60 chars *)
```

| Convention | Used By | Examples |
|------------|---------|----------|
| Free-form identifier | all named entities | `data_persistence`, `UserRepository`, `camelCase`, `SCREAMING_SNAKE` |
| Scheme-based | ref | `gh.issue:42`, `jira.epic:PROJ-123` |
| Singleton | spec | one per project, no ID |

### Flat Namespace

No two entities of ANY type can share the same name. `invariant data_check` and `behavior data_check` in the same project is an `E002` error. This prevents ambiguity in cross-references.

### Title Derivation

The title string after the identifier is optional. If omitted, the compiler auto-derives a title from the identifier:

- `auth_login` → "Auth Login"
- `data_persistence` → "Data Persistence"
- `UserRepository` → "User Repository"

Explicit titles override: `behavior auth_login "Login with Credentials" { ... }`

### Reserved Words

All entity keywords from installed extensions are reserved identifiers. Using a reserved word as an entity name produces `E013`. The structural keywords `spec` and `ref` are always reserved. Extension-declared keywords (`behavior`, `invariant`, etc.) are reserved when the extension is installed.

### Unicode

Identifiers allow Unicode letters (NFC-normalized). Bidirectional characters are forbidden. The `--ascii-only` lint restricts identifiers to ASCII.

### Backtick Escaping

For edge cases, backtick-escaped identifiers allow characters normally forbidden: `` `complex-name` ``.

## Traceability Chain

The entities form a directed acyclic graph. The @specforge/software chain is self-contained and useful on its own. Other extensions extend it:

### @specforge/software Chain

```
behavior ──enforces──→ invariant
    │
    │produces
    ▼
  event

type ←──extends_type── type (inheritance)
type / port (code bridge, with UsesType edges)

any entity ──tested_by──→ test file
any entity ──external_ref──→ URI
```

### Extended by @specforge/product

```
release ──ships──→ deliverable ──bundles──→ journey ──traces_to──→ feature
    │                  │            │
    │targets           │built_from  │persona / channels
    ▼                  ▼            ▼
  milestone          module     persona / channel
                     │
                     │provides
                     ▼
                   feature

milestone ──schedules──→ feature / module
milestone ──depends_on──→ milestone
deliverable ──targets──→ milestone
deliverable ──depends_on──→ deliverable
```

### Extended by @specforge/governance

```
decision ──protects──→ invariant (software)

constraint ──constrains──→ behavior (software) / invariant (software)

failure_mode ──mitigates──→ invariant (software)
```

**Full chain:** `release -> deliverable -> journey -> feature -> behavior -> invariant`
**Code bridge:** `deliverable -> module -provides-> feature`
**Temporal:** `milestone -> feature / module / milestone`
**Release coordination:** `release -> deliverable / milestone`
**Governance overlay:** `decision ─protects→ invariant`, `constraint → behavior`, `failure_mode → invariant`

## Edge Types

### @specforge/software Edges (11)

| Edge Type | From | To | Semantics |
|-----------|------|----|-----------|
| `References` | any | any | General cross-reference |
| `Implements` | behavior | feature (product) | "This behavior implements this feature" |
| `Produces` | behavior | event | "This behavior emits these events" |
| `Consumes` | behavior | event | "This behavior reacts to these events" |
| `UsesType` | behavior/port/type | type | "This entity uses these type definitions" |
| `UsesPort` | behavior | port | "This behavior uses these port interfaces" |
| `Enforces` | behavior | invariant | "This behavior enforces these invariants" |
| `ExtendsType` | type | type | "This type extends/composes that type" |
| `TestedBy` | any testable | test file | "This entity is tested by these files" |
| `ExternalRef` | any | URI | "This entity links to this external reference" |
| `MilestoneBehavior` | milestone | behavior | Cross-extension enhancement edge |

### @specforge/product Edges

| Edge Type | From | To | Semantics |
|-----------|------|----|-----------|
| `JourneyFeature` | journey | feature | "This UX flow maps to these features" |
| `DeliverableJourney` | deliverable | journey | "This deliverable ships these journeys" |
| `DeliverableModule` | deliverable | module | "This deliverable uses these modules" |
| `ModuleDependsOn` | module | module | "This module depends on that module" |
| `ModuleFeature` | module | feature | "This module implements these features" |
| `MilestoneFeature` | milestone | feature | "This phase schedules these features" |
| `FeatureDependsOn` | feature | feature | "This feature depends on that feature" |
| `JourneyPersona` | journey | persona | "This journey is performed by this persona" |
| `JourneyChannel` | journey | channel | "This journey occurs through this channel" |
| `MilestoneModule` | milestone | module | "This milestone includes these modules" |
| `MilestoneDependsOn` | milestone | milestone | "This milestone depends on that milestone completing first" |
| `TermSeeAlso` | term | term | "This term cross-references that term" |
| `DeliverableDependsOn` | deliverable | deliverable | "This deliverable depends on that deliverable" |
| `DeliverableMilestone` | deliverable | milestone | "This deliverable targets these milestones" |
| `ReleaseDeliverable` | release | deliverable | "This release ships these deliverables" |
| `ReleaseMilestone` | release | milestone | "This release targets these milestones" |

### @specforge/governance Edges

| Edge Type | From | To | Semantics |
|-----------|------|----|-----------|
| `protects` | decision | invariant | "This decision protects these invariants" |
| `constrains` | constraint | behavior/invariant | "This quality requirement applies to these entities" |
| `mitigates` | failure_mode | invariant | "This failure mode threatens this invariant" |

### Cross-Extension Edges (Soft References)

| Edge Type | From | To | Semantics |
|-----------|------|----|-----------|
| `Implements` | behavior (software) | feature (product) | "This behavior implements this feature" — via peer_dependency |
| `MilestoneBehavior` | milestone (product) | behavior (software) | "This milestone delivers these behaviors" — via entity_enhancement |

## Validation Rules

The compiler enforces structural invariants. Each rule belongs to the extension that owns the entities it validates. **Extension rules only fire when the extension is installed.** Cross-extension rules include `requires` guards for extension availability.

### Core Errors

| Code | Rule |
|------|------|
| E001 | **No dangling references** — every ID in a reference list must resolve to a declared entity (soft for cross-extension references) |
| E002 | **No duplicate IDs** — each entity ID is globally unique across all `.spec` files |
| E003 | **No import cycles** — the `use` statements form a DAG |
| E011 | **Invalid ref target format** — provider validates identifier doesn't match expected pattern |
| E012 | **Unknown provider kind** — ref uses kind not registered by its provider |
| E013 | **Reserved word used as identifier** — entity name is a reserved keyword |
| E014 | **Invalid identifier characters** — identifier contains forbidden characters |

### @specforge/software Errors

| Code | Rule |
|------|------|
| E004 | **Invalid port methods** — port operation type references unknown entity |
| E006 | **Event trigger invalid** — event's trigger must reference existing behaviors |
| E010 | **Invalid behavior range** — milestone behaviors range is malformed (requires @specforge/product) |

### @specforge/software Warnings

| Code | Rule |
|------|------|
| W001 | **Orphan behavior** — not implementing any feature (requires @specforge/product) |
| W002 | **Orphan type** — no incoming UsesType or ExtendsType edges |
| W003 | **Unused invariant** — no incoming Enforces edges from behaviors |
| W004 | **Unverified testable** — testable entity with no verify or test reference |
| W005 | **Orphan port** — no incoming UsesPort edges |
| W006 | **Missing behavior category** — agent task routing requires category |
| W007 | **Orphan event** — no incoming Produces edges (see also: W029) |
| W008 | **Feature without behaviors** — no incoming Implements edges (requires @specforge/product) |
| W009 | **Invalid verify kind** — verify kind not in allowed set for entity kind |
| W010 | **Unknown field annotation** — unknown annotation on type field |

### @specforge/formal Errors (requires warning_level=strict)

| Code | Rule |
|------|------|
| E030 | **Contradictory precondition** — structurally contradictory precondition (X/not_X, tautological false) |
| E031 | **Layering condition mismatch** — named-condition set violation in layering |
| E032 | **Layering cycle** — cycle in specification layering DAG |
| E034 | **Unmitigated cycle** — circular dependency without sync.timeout, @idempotent, or circuit_breaker |
| E035 | **Payload type mismatch** — producer/consumer disagree on event payload type |
| E041 | **Refinement chain cycle** — cycle in RefinementChainLink DAG |
| E042 | **Process composition cycle** — cycle in ProcessComposition DAG |

### @specforge/formal Warnings (requires warning_level=strict)

| Code | Rule |
|------|------|
| W028 | **Conditions without verify** — requires/ensures without contract/property verify |
| W029 | **Unmatched producers** — event with producers but no consumers |
| W030 | **Incomplete layering** — abstract behavior with no concrete refinement |
| W031 | **Deep layering chain** — chain depth > 4 |
| W032 | **Unmitigated retry cycle** — event cycle without timeout in sync block |
| W033 | **Asymmetric connectivity** — port with structurally unbalanced access pattern |
| W034 | **Unbounded channel** — event with no sync timeout or buffer limit |
| W035 | **Undischarged coverage item** — coverage tracking item not covered by test (aggregated summary) |
| W036 | **Port-behavior incompatibility** — port condition stricter/weaker than behavior |
| W037 | **Unverifiable condition** — condition references external state |
| W038 | **Unreachable postcondition** — postcondition contradicts precondition |
| W039 | **Redundant precondition** — precondition implied by sibling |
| W040 | **Invariant without property** — prose guarantee without maintains block |
| W058 | **Feature coverage mismatch** — behavior may not satisfy feature requirements (structural check only, downgraded from E033) |
| W059 | ~~REMOVED~~ — condition entity kind removed |
| W060 | ~~REMOVED~~ — condition entity kind removed |
| W061 | **Orphan property** — no incoming Satisfies edges from behaviors |
| W062 | **Empty property description** — property has blank description |
| W063 | **Property without kind** — missing safety/liveness/fairness classification |
| W064 | **Orphan axiom** — no incoming AssumedBy edges from invariants |
| W065 | **Empty axiom description** — axiom has blank description |
| W066 | **Orphan protocol** — no incoming FollowsProtocol edges from events |
| W067 | **Empty protocol description** — protocol has blank description |
| W068 | **Protocol ordering conflict** — ordering references events not in graph |
| W069 | **Orphan refinement** — no incoming RefinesTo/RefinementChainLink edges |
| W070 | **Empty refinement description** — refinement has blank description |
| W071 | **Refinement without condition delta** — no conditions field |
| W072 | **Orphan process** — no incoming ParticipatesIn edges |
| W073 | **Empty process description** — process has blank description |
| W074 | **Process without alphabet** — empty/absent alphabet field |

### Core Info

| Code | Rule |
|------|------|
| I003 | **Newer format features available** — project version < compiler version |
| I004 | **Unknown entity in reference field** — reference uses an identifier not found in any installed extension's entity registry |
| I005 | **Unknown provider scheme** — ref uses a scheme not registered by any installed provider |

### @specforge/product Errors

| Code | Rule |
|------|------|
| E007 | **Circular module dependency** — `depends_on` edges between modules form a cycle |
| E008 | **Persona not defined** — journey's `persona` doesn't match any persona defined in the project |
| E009 | **Channel not defined** — journey's `channel` doesn't match any channel defined in the project |
| E015 | **Circular milestone dependency** — `depends_on` edges between milestones form a cycle |

### @specforge/product Warnings

| Code | Rule |
|------|------|
| W041 | **Orphan feature** — not referenced by any journey |
| W042 | **Orphan journey** — journey not referenced by any deliverable's `journeys` field |
| W043 | **Deliverable with no journeys** — deliverable has an empty `journeys` list |
| W044 | **Orphan module** — module not referenced by any deliverable's `modules` field |
| W045 | **Feature dependency cycle** — `depends_on` edges between features form a cycle |

### @specforge/product Info

| Code | Rule |
|------|------|
| I010 | **Orphan term** — term not referenced by any entity's see_also |
| I046 | **Orphan persona** — persona not referenced by any journey |
| I047 | **Orphan channel** — channel not referenced by any journey |

### @specforge/governance Errors

| Code | Rule |
|------|------|
| E005 | **RPN mismatch** — severity × occurrence × detection ≠ declared rpn |

### @specforge/governance Warnings

| Code | Rule |
|------|------|
| W005 | **Unmitigated high-risk invariant** — `risk: high` with no `failure_mode` |
| W006 | **Unconstrained behavior** — behavior with no constraint coverage for common categories |

### @specforge/governance Info

| Code | Rule |
|------|------|
| I001 | **Stale proposal** — decision with `status: proposed` older than 30 days |

## DSL Scope Boundaries

### What belongs in the DSL (23 entity types across 4 extensions + structural core)

The 23 entity types above are the complete set of compiled block types. They were selected because they have high cross-reference density, benefit from compiler validation, and complete the traceability chain.

### What stays as markdown

These concepts gain nothing from compilation — they are prose documents with minimal cross-references:

| Concept | Reason |
|---------|--------|
| **research** | Exploratory narratives. Only `related_adr` and `outcome` are structural. |
| **product** | Pure prose: pitch, positioning, go-to-market strategy. Zero cross-references. |
| **process** | Governance docs: definition of done, test strategy, change control. |
| **references** | External links and tool references. Now partially handled by the `ref` entity for compiler-tracked external references; unstructured references remain as markdown. |
| **type-system** | Meta-documentation about type patterns. The `type` blocks handle actual types. |

### What is generated output

These are never source — they are produced by the compiler:

| Concept | Generated by |
|---------|-------------|
| **traceability** | `specforge trace` — auto-generated from graph traversal |
| **overview** | Compiler-generated from the graph |

### Meta-schema extensibility

For domain-specific entity types beyond the 23 shipped types, the `define` mechanism in the `spec` root block allows user-defined types with attribute validation, reference resolution, orphan detection, and LSP support. See [spec entity docs](entities/spec.md) for syntax.

## Progressive Adoption

SpecForge supports progressive adoption via its extension architecture. Teams start with structural core and add extensions as projects grow.

### Level 1: Structural Only

Just the compiler with zero domain knowledge. Parses any `keyword name { fields }` block generically. Useful for exploring the DSL or using only `define` for custom entity types.

```bash
specforge init --no-extensions
# → 2 structural kinds: spec, ref
# → Core grammar parses any keyword, but no validation beyond structure
```

### Level 2: + Software Engineering (recommended starting point)

For any software project. Add behavioral contracts, domain events, type definitions, and port interfaces.

```bash
specforge init
# → @specforge/software pre-selected (recommended)
# → +5 entities: behavior, invariant, event, type, port
# → All 5 are testable with verify support
```

### Level 3: + Product Planning

For teams building products, add journeys, deliverables, milestones, modules, terms, features, personas, and channels.

```bash
specforge add @specforge/product
# → +9 entities: feature, journey, deliverable, milestone, module, term, persona, channel, release
```

### Level 3.5: + Formal Analysis

For teams using structured conditions, specification layering, or event graph linting. Requires @specforge/software.

```bash
specforge add @specforge/formal
# → +5 entity kinds: property, axiom, protocol, refinement, process
# → +4 compiler passes: condition_check, layering_verify, event_graph_analyze, coverage_tracking
# → +8 edge types (AssumedBy, Satisfies, FollowsProtocol, PropertyDependsOn, RefinesTo, RefinementChainLink, ParticipatesIn, ProcessComposition)
# → Inline condition fields (requires/ensures/maintains) enhanced on behavior entities
# → Requires warning_level=strict in specforge.json for formal warnings
```

### Level 4: + Governance

For teams that need architecture rationale, quality tracking, and risk management.

```bash
specforge add @specforge/governance
# → +3 entities: decision, constraint, failure_mode
```

### Level 5: Domain-Specific

For regulated industries, complex domains, or custom workflows. Use the meta-schema `define` mechanism or the community extension ecosystem.

```bash
# Meta-schema: define custom entity types in specforge.spec
# Community: specforge add @specforge/compliance
#            specforge add @specforge/visual
```

A team using only @specforge/software gets full value from `specforge check` + `specforge trace` without ever touching product planning, governance, or compliance.

## Design Principles

1. **Zero domain knowledge in core** — the compiler is a pure typed-graph engine; ALL domain vocabulary comes from extensions; if a new domain requires a compiler change, the architecture has failed
2. **Every entity earns its place** — each answers a distinct question no other entity answers
3. **Compiler-checked references** — entity names are typed, resolved, and validated at compile time; cross-extension references degrade gracefully via soft references
4. **Traceability by construction** — the graph structure enforces traceability; orphan detection catches missing links
5. **Progressive adoption** — start with structural core, add @specforge/software (5), @specforge/product (8), @specforge/governance (3), @specforge/formal (5 kinds, 4 passes) as needed
6. **Language-agnostic** — the entity model works for any software project regardless of implementation language
7. **Bounded complexity** — the DSL balances expressiveness with readability (currently 24 entity kinds across 4 extensions); beyond official extensions, use `define` or community extensions
8. **Extensions don't break specs** — a spec file is always valid with structural core alone; extensions add validation, they don't remove it
