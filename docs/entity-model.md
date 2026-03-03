# SpecForge Entity Model

## Overview

SpecForge defines 16 entity types organized into a **core compiler** (8 entities) and **two official plugins** (8 entities). This architecture follows the Terraform model: a small, stable core grammar with extensible plugins for domain-specific needs. Beyond plugins, SpecForge supports **providers** (external platform integrations for `ref` entities) and **generators** (output format extensions). See [extension-model.md](extension-model.md) for the full three-part extension architecture.

Every entity has a unique ID, compiler-checked cross-references, and a defined role in the traceability chain. Teams adopt only what they need — start with core, add plugins as projects grow.

## Architecture: Core + Plugins

```
┌─────────────────────────────────────────────────────────────────┐
│                        CORE (8 entities)                         │
│  spec · invariant · behavior · feature · event · type · port · ref│
│  + meta-schema `define` mechanism                                │
├──────────────────────────────┬──────────────────────────────────┤
│   @specforge/product (5)     │   @specforge/governance (3)      │
│  capability · deliverable    │  decision · constraint            │
│  roadmap · library · glossary│  failure_mode                    │
└──────────────────────────────┴──────────────────────────────────┘
```

### Why This Split

**Core** contains the structural primitives every specification needs — the traceability chain (`feature → behavior → invariant`), the domain event model (`event`), the code bridge (`type` + `port`), and the external reference bridge (`ref`). These 8 entities are to SpecForge what `resource`, `data`, `variable`, `output` are to Terraform: universal building blocks.

**@specforge/product** adds product planning and delivery entities. Not every project ships a product — internal tools, scripts, and libraries don't need capabilities, deliverables, or roadmaps. This plugin extends the chain upward (`deliverable → capability → feature`) and adds the code bridge (`library → port/type`) and temporal dimension (`roadmap`).

**@specforge/governance** adds architecture governance, quality tracking, and risk assessment. Not every project formalizes ADRs, NFRs, or FMEA — early-stage startups and prototypes rarely do. This plugin adds overlay entities that reference core entities (`decision → invariant`, `constraint → behavior`, `failure_mode → invariant`).

### Plugin CLI

```bash
specforge init                          # core only (8 entities)
specforge add @specforge/product        # + 5 product entities
specforge add @specforge/governance     # + 3 governance entities
specforge remove @specforge/governance  # remove plugin
specforge plugins                       # list installed plugins
```

`specforge init` offers interactive setup:

```
? Which plugins do you want? (space to select)
  ● @specforge/product      — capability, deliverable, roadmap, library, glossary
  ○ @specforge/governance   — decision, constraint, failure_mode
```

## Entity Summary

### Core

| # | Entity | ID Pattern | Question It Answers |
|---|--------|-----------|---------------------|
| 1 | [spec](entities/spec.md) | singleton | What project is this? |
| 2 | [invariant](entities/invariant.md) | `identifier` | What must ALWAYS be true? |
| 3 | [behavior](entities/behavior.md) | `identifier` | What exactly does the system do? |
| 4 | [feature](entities/feature.md) | `identifier` | What value does this deliver? |
| 5 | [event](entities/event.md) | `identifier` | What does the system announce? |
| 6 | [type](entities/type.md) | `identifier` | What shape does the data have? |
| 7 | [port](entities/port.md) | `identifier` | What contracts exist between components? |
| 8 | [ref](entities/ref.md) | `scheme.kind:identifier` | What external resource is this connected to? |

### @specforge/product

| # | Entity | ID Pattern | Question It Answers |
|---|--------|-----------|---------------------|
| 9 | [capability](entities/capability.md) | `identifier` | How does the user experience this? |
| 10 | [deliverable](entities/deliverable.md) | `identifier` | What ships to users? |
| 11 | [roadmap](entities/roadmap.md) | `identifier` | When does this ship? |
| 12 | [library](entities/library.md) | `identifier` | What code package implements this? |
| 13 | [glossary](entities/glossary.md) | singleton | What do our terms mean? |

### @specforge/governance

| # | Entity | ID Pattern | Question It Answers |
|---|--------|-----------|---------------------|
| 14 | [decision](entities/decision.md) | `identifier` | Why was this built this way? |
| 15 | [constraint](entities/constraint.md) | `identifier` | What quality must the system achieve? |
| 16 | [failure_mode](entities/failure-mode.md) | `identifier` | What can go wrong and how bad is it? |

## Cross-Plugin References

When an entity references another entity from a different module (core ↔ plugin, or plugin ↔ plugin), the compiler uses **soft references** — a progressive enhancement model where spec files are valid with or without plugins installed.

### Resolution Rules

| Scenario | From | To | Behavior |
|----------|------|----|----------|
| **Core → Core** | core entity | core entity | Always validated. Dangling reference = `E001` error. |
| **Plugin → Core** | plugin entity | core entity | Always validated. Plugin entities can always reference core entities. |
| **Core → Plugin** | core entity | plugin entity | **Soft reference.** If plugin installed → validated (`E001` on miss). If not installed → `I004` info. |
| **Plugin → Plugin** | plugin entity | other plugin entity | **Soft reference.** Same as Core → Plugin. |
| **Same Plugin** | plugin entity | same plugin entity | Always validated. |

### Diagnostic: I004 (Unknown Entity in Reference Field)

When a reference uses an identifier not found in any installed module's entity registry:

```
info[I004]: Unknown entity 'use_postgresql' in field 'adrs'
  ┌─ behaviors/auth.spec:3:16
  │
3 │   adrs [use_postgresql]
  │         ^^^^^^^^^^^^^^ not found in installed modules
  │
  = help: Install @specforge/governance to enable decision validation
```

### Example

```spec
// Core-only project (no plugins installed)
behavior create_user {
  invariants [data_persistence]    // ✅ Core → Core: validated
  adrs       [use_postgresql]      // ℹ️ I004: "Install @specforge/governance"

  contract "..."
}
```

After `specforge add @specforge/governance`:

```spec
// Same file — now governance is installed
behavior create_user {
  invariants [data_persistence]    // ✅ Core → Core: validated
  adrs       [use_postgresql]      // ✅ Core → Governance: validated (E001 if not found)

  contract "..."
}
```

### Field-to-EntityKind Registry

The compiler uses the field name in which a reference appears to determine the expected target entity type. This replaces the prefix-based routing of the old ID system.

| Field Name | Target EntityKind | Module |
|------------|-------------------|--------|
| `invariants` | Invariant | core |
| `behaviors` | Behavior | core |
| `features` | Feature | core |
| `types` | Type | core |
| `ports` | Port | core |
| `consumers` | Behavior | core |
| `trigger` | Behavior | core |
| `enforced_by` | Behavior | core |
| `refs` | Ref | core |
| `capabilities` | Capability | @specforge/product |
| `libraries` | Library | @specforge/product |
| `depends_on` | Library | @specforge/product |
| `ports_defined` | Port | core |
| `roadmap` | Roadmap | @specforge/product |
| `adrs` | Decision | @specforge/governance |
| `invariant` (singular) | Invariant | core |
| `affects` | Behavior | core |

When the target kind's module is not installed, the compiler emits `I004` instead of `E001`.

## Naming Conventions

Any valid identifier is accepted for all entity kinds. There is no enforced case convention.

```ebnf
identifier = letter , { letter | digit | "_" } ;   (* 2-60 chars *)
```

| Convention | Used By | Examples |
|------------|---------|----------|
| Free-form identifier | all named entities | `data_persistence`, `UserRepository`, `camelCase`, `SCREAMING_SNAKE` |
| Scheme-based | ref | `gh.issue:42`, `jira.epic:PROJ-123` |
| Singleton | spec, glossary | one per project, no ID |

### Flat Namespace

No two entities of ANY type can share the same name. `invariant data_check` and `behavior data_check` in the same project is an `E002` error. This prevents ambiguity in cross-references.

### Title Derivation

The title string after the identifier is optional. If omitted, the compiler auto-derives a title from the identifier:

- `auth_login` → "Auth Login"
- `data_persistence` → "Data Persistence"
- `UserRepository` → "User Repository"

Explicit titles override: `behavior auth_login "Login with Credentials" { ... }`

### Reserved Words

All 16 entity keywords are reserved identifiers: `spec`, `invariant`, `behavior`, `feature`, `event`, `type`, `port`, `ref`, `capability`, `deliverable`, `roadmap`, `library`, `glossary`, `decision`, `constraint`, `failure_mode`. Using a reserved word as an entity name produces `E013`.

### Unicode

Identifiers allow Unicode letters (NFC-normalized). Bidirectional characters are forbidden. The `--ascii-only` lint restricts identifiers to ASCII.

### Backtick Escaping

For edge cases, backtick-escaped identifiers allow characters normally forbidden: `` `complex-name` ``.

## Traceability Chain

The entities form a directed acyclic graph. The core chain is self-contained and useful on its own. Plugins extend it:

### Core Chain

```
feature ──implements──→ behavior ──references──→ invariant
                                       │
                                       │produces
                                       ▼
                                     event

                       type / port (code bridge)

          any entity ──links_to──→ ref (external reference bridge)
                                   scheme.kind:id
```

### Extended by @specforge/product

```
deliverable ──bundles──→ capability ──traces_to──→ feature (core)
                │
                │built_from
                ▼
              library ──provides──→ feature (core)
                         │
                         │defines_port
                         ▼
                       port (core)

roadmap ──schedules──→ feature (core) / deliverable
```

### Extended by @specforge/governance

```
decision ──protects──→ invariant (core)

constraint ──constrains──→ behavior (core) / invariant (core)

failure_mode ──mitigates──→ invariant (core)
```

**Full chain:** `deliverable → capability → feature → behavior → invariant`
**Code bridge:** `deliverable → library ─provides→ feature` and `library → port / type`
**Temporal:** `roadmap → feature / deliverable`
**Governance overlay:** `decision ─protects→ invariant`, `constraint → behavior`, `failure_mode → invariant`

## Edge Types

### Core Edges

| Edge Type | From | To | Semantics |
|-----------|------|----|-----------|
| `references` | behavior | invariant | "This behavior depends on these invariants" |
| `implements` | feature | behavior | "This feature is composed of these behaviors" |
| `produces` | behavior | event | "This behavior emits these events" |
| `consumes` | event | behavior | "This event triggers these behaviors" |
| `uses_type` | behavior | type | "This behavior uses these type definitions" |
| `uses_port` | behavior | port | "This behavior uses these port interfaces" |
| `enforces` | invariant | behavior | "This invariant is enforced by these behaviors" |
| `imports` | file | file | "This file uses symbols from that file" |
| `links_to` | any entity | ref | "This entity links to this external reference" |

### @specforge/product Edges

| Edge Type | From | To | Semantics |
|-----------|------|----|-----------|
| `traces_to` | capability | feature | "This UX flow maps to these features" |
| `bundles` | deliverable | capability | "This deliverable ships these capabilities" |
| `built_from` | deliverable | library | "This deliverable uses these libraries" |
| `depends_on` | library | library | "This library depends on that library" |
| `provides` | library | feature | "This library provides the code for these features" |
| `defines_port` | library | port | "This library defines this port interface" |
| `schedules` | roadmap | feature/deliverable | "This phase schedules these features or deliverables" |

### @specforge/governance Edges

| Edge Type | From | To | Semantics |
|-----------|------|----|-----------|
| `protects` | decision | invariant | "This decision protects these invariants" |
| `constrains` | constraint | behavior/invariant | "This quality requirement applies to these entities" |
| `mitigates` | failure_mode | invariant | "This failure mode threatens this invariant" |

### Cross-Module Edge (Soft Reference)

| Edge Type | From | To | Semantics |
|-----------|------|----|-----------|
| `shaped_by` | behavior (core) | decision (governance) | "This behavior was shaped by these decisions" — soft reference |

## Validation Rules

The compiler enforces structural invariants. Each rule belongs to the module that owns the entities it validates. **Plugin rules only fire when the plugin is installed.**

### Core Errors

| Code | Rule |
|------|------|
| E001 | **No dangling references** — every ID in a reference list must resolve to a declared entity (soft for cross-plugin references) |
| E002 | **No duplicate IDs** — each entity ID is globally unique across all `.spec` files |
| E003 | **No import cycles** — the `imports` edges form a DAG |
| E006 | **Event trigger invalid** — event's trigger must reference an existing behavior |
| E011 | **Invalid ref target format** — provider validates identifier doesn't match expected pattern |
| E012 | **Unknown provider kind** — ref uses kind not registered by its provider |
| E013 | **Reserved word used as identifier** — entity name is a reserved keyword |
| E014 | **Invalid identifier characters** — identifier contains forbidden characters |

### Core Warnings

| Code | Rule |
|------|------|
| W001 | **Orphan behavior** — not referenced by any feature |
| W003 | **Unused invariant** — not referenced by any behavior |
| W004 | **Unverified behavior** — no `verify` statement |
| W007 | **Orphan event** — event with no consumers |
| W012 | **Orphan ref** — declared but never referenced by any entity |
| W013 | **Vague name** — identifier is too generic (e.g., `data`, `thing`) |

### Core Info

| Code | Rule |
|------|------|
| I003 | **Newer format features available** — project version < compiler version |
| I004 | **Unknown entity in reference field** — reference uses an identifier not found in any installed module's entity registry |
| I005 | **Unknown provider scheme** — ref uses a scheme not registered by any installed provider |

### @specforge/product Errors

| Code | Rule |
|------|------|
| E007 | **Circular library dependency** — `depends_on` edges between libraries form a cycle |
| E008 | **Persona not defined** — capability's `persona` doesn't match any persona defined in `spec` root (only fires when spec root defines personas) |
| E009 | **Surface not defined** — capability's `surface` doesn't match any surface defined in `spec` root (only fires when spec root defines surfaces) |

### @specforge/product Warnings

| Code | Rule |
|------|------|
| W002 | **Orphan feature** — not referenced by any capability |
| W008 | **Uncovered capability** — deliverable references a capability not reachable via its libraries |
| W009 | **Orphan library** — library not referenced by any deliverable's `libraries` field |
| W010 | **Deprecated feature** — using a feature deprecated in the current format version |
| W011 | **Orphan capability** — capability not referenced by any deliverable's `capabilities` field |

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

### What belongs in the DSL (16 entity types across 3 modules)

The 16 entity types above are the complete set of compiled block types. They were selected because they have high cross-reference density, benefit from compiler validation, and complete the traceability chain.

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

For domain-specific entity types beyond the 15 shipped types, the `define` mechanism in the `spec` root block allows user-defined types with attribute validation, reference resolution, orphan detection, and LSP support. See [spec entity docs](entities/spec.md) for syntax.

**Complexity budget:** ~15-20 max hardcoded node types. The current 16 are within this budget. Beyond ~20 types, use the meta-schema `define` mechanism or create a community plugin.

## Progressive Adoption

SpecForge supports progressive adoption via its plugin architecture. Teams start with core and add plugins as projects grow.

### Level 1: Core Only

Adoptable in an afternoon. Start with invariants, behaviors, features, events, types, ports, and refs. `specforge check` + `specforge trace` provide full value immediately.

```bash
specforge init
# → 8 entities: spec, invariant, behavior, feature, event, type, port, ref
```

### Level 2: + Product Planning

For teams building products, add capabilities, deliverables, roadmaps, libraries, and glossary.

```bash
specforge add @specforge/product
# → +5 entities: capability, deliverable, roadmap, library, glossary
```

### Level 3: + Governance

For teams that need architecture rationale, quality tracking, and risk management.

```bash
specforge add @specforge/governance
# → +3 entities: decision, constraint, failure_mode
```

### Level 4: Domain-Specific

For regulated industries, complex domains, or custom workflows. Use the meta-schema `define` mechanism or the community plugin ecosystem.

```bash
# Meta-schema: define custom entity types in specforge.spec
# Community: specforge add @specforge/compliance
#            specforge add @specforge/visual
```

A team using only core gets full value from `specforge check` + `specforge trace` without ever touching product planning, governance, or compliance.

## Design Principles

1. **Small stable core** — 8 entities cover the universal specification chain; everything else is a plugin
2. **Every entity earns its place** — each answers a distinct question no other entity answers
3. **Compiler-checked references** — entity names are typed, resolved, and validated at compile time; cross-plugin references degrade gracefully via soft references
4. **Traceability by construction** — the graph structure enforces traceability; orphan detection catches missing links
5. **Progressive adoption** — start with core (8 entities), add @specforge/product (5) and @specforge/governance (3) as needed
6. **Language-agnostic** — the entity model works for any software project regardless of implementation language
7. **Bounded complexity** — the DSL has a hard budget of ~15-20 hardcoded types (currently 16); beyond that, use `define` or community plugins
8. **Plugins don't break specs** — a spec file is always valid with core alone; plugins add validation, they don't remove it
