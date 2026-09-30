# Quick Reference

Single-page lookup for every entity kind, edge type, and validation code (provided by the four builtin extensions). For full details, see the [entity model](entity-model.md) or individual [entity docs](entities/).

---

## Entities

### spec
> Module: core | ID: singleton | "What project is this?"

| Required | Optional |
|----------|----------|
| name, version, extensions | test_dirs, persona, surface, coverage — *(planned: not yet implemented)* providers, gen |

No graph edges. Root configuration declaring which extensions the project uses.

---

### invariant
> Module: core | ID: author-chosen identifier | "What must ALWAYS be true?"

| Required | Optional |
|----------|----------|
| title, guarantee | enforced_by, risk |

Outgoing: behavior (`enforces`)
Incoming: behavior (`references`), decision (`protects`), constraint (`constrains`), failure_mode (`mitigates`)

---

### behavior
> Module: core | ID: author-chosen identifier | "What exactly does the system do?"

| Required | Optional |
|----------|----------|
| title, contract | invariants, adrs, types, ports, verify, tests |

Outgoing: invariant (`references`), event (`produces`), type (`uses_type`), port (`uses_port`), decision (`shaped_by`)
Incoming: feature (`implements`), event (`consumes`), constraint (`constrains`)

---

### feature
> Module: core | ID: author-chosen identifier | "What value does this deliver?"

| Required | Optional |
|----------|----------|
| title, behaviors, problem, solution | roadmap |

Outgoing: behavior (`implements`)
Incoming: capability (`traces_to`), library (`provides`), roadmap (`schedules`)

---

### event
> Module: core | ID: author-chosen identifier | "What does the system announce?"

| Required | Optional |
|----------|----------|
| title, trigger | payload, channel, consumers |

Outgoing: behavior (`consumes`)
Incoming: behavior (`produces`)

---

### type
> Module: core | ID: identifier | "What shape does the data have?"

| Required | Optional |
|----------|----------|
| name, fields or variants | — |

No outgoing edges.
Incoming: behavior (`uses_type`), event (payload reference), port (method signatures)

---

### port
> Module: core | ID: identifier | "What contracts exist between components?"

| Required | Optional |
|----------|----------|
| name, direction, methods | category |

No outgoing edges.
Incoming: behavior (`uses_port`), library (`defines_port`), invariant (`enforces`)

---

### ref
> Module: core | ID: `scheme.kind:identifier` | "What external resource is this connected to?"

| Required | Optional |
|----------|----------|
| scheme, identifier | title, provider-specific fields |

No outgoing edges. Leaf node.
Incoming: any entity (`links_to`)

---

### journey
> Module: @specforge/product | ID: `identifier` | "How does the user experience this?"

| Required | Optional |
|----------|----------|
| title, persona, features, flow | surface |

Outgoing: feature (`traces_to`)
Incoming: deliverable (`bundles`)

---

### deliverable
> Module: @specforge/product | ID: author-chosen identifier | "What ships to users?"

| Required | Optional |
|----------|----------|
| title, journeys | modules, milestone, personas, type |

Outgoing: journey (`bundles`), module (`built_from`)
Incoming: milestone (`schedules`)

---

### milestone
> Module: @specforge/product | ID: `identifier` | "When does this ship?"

| Required | Optional |
|----------|----------|
| title, status | features, criteria |

Outgoing: feature (`schedules`), deliverable (`schedules`)
Incoming: feature (`milestone` field), deliverable (`milestone` field)

---

### module
> Module: @specforge/product | ID: `identifier` | "What component delivers this?"

| Required | Optional |
|----------|----------|
| title, features | depends_on, description, family |

Outgoing: feature (`provides`), module (`depends_on`)
Incoming: deliverable (`built_from`), module (`depends_on`)

---

### term
> Module: @specforge/product | ID: `identifier` | "What does this term mean?"

| Required | Optional |
|----------|----------|
| definition | title, aliases, context, see |

No graph edges. The `see` field is informational only — it does not create compiler-tracked edges.

---

### decision
> Module: @specforge/governance | ID: `ADR-{n}` | "Why was this built this way?"

| Required | Optional |
|----------|----------|
| title, status, context, decision | date, consequences, invariants |

Outgoing: invariant (`protects`)
Incoming: behavior (`shaped_by`)

---

### constraint
> Module: @specforge/governance | ID: author-chosen identifier | "What quality must the system achieve?"

| Required | Optional |
|----------|----------|
| title, category, priority, description/metric | behaviors/affects, invariants, verify |

Outgoing: behavior (`constrains`), invariant (`constrains`)
No incoming edges.

---

### failure_mode
> Module: @specforge/governance | ID: author-chosen identifier | "What can go wrong and how bad is it?"

| Required | Optional |
|----------|----------|
| title, invariant, severity, occurrence, detection | rpn, cause, effect, mitigation, post_mitigation |

Outgoing: invariant (`mitigates`)
No incoming edges.

---

## Edge Types

### Core (9 edges)

| Edge | From | To | Meaning |
|------|------|----|---------|
| `references` | behavior | invariant | Behavior depends on invariants |
| `implements` | feature | behavior | Feature is composed of behaviors |
| `produces` | behavior | event | Behavior emits events |
| `consumes` | event | behavior | Event triggers behaviors |
| `uses_type` | behavior | type | Behavior uses type definitions |
| `uses_port` | behavior | port | Behavior uses port interfaces |
| `enforces` | invariant | behavior | Invariant enforced by behaviors |
| `imports` | file | file | File uses symbols from another file |
| `links_to` | any entity | ref | Entity links to external reference |

### @specforge/product (7 edges)

| Edge | From | To | Meaning |
|------|------|----|---------|
| `traces_to` | journey | feature | UX flow maps to features |
| `bundles` | deliverable | journey | Deliverable ships journeys |
| `built_from` | deliverable | module | Deliverable uses modules |
| `depends_on` | module | module | Module depends on another module |
| `provides` | module | feature | Module implements features |
| `schedules` | milestone | feature/deliverable | Phase schedules features or deliverables |
| `FeatureDependsOn` | feature | feature | Feature depends on another feature |

### @specforge/governance (4 edges)

| Edge | From | To | Meaning |
|------|------|----|---------|
| `protects` | decision | invariant | Decision protects invariants |
| `constrains` | constraint | behavior/invariant | Quality requirement applies to entities |
| `mitigates` | failure_mode | invariant | Failure mode threatens invariant |
| `shaped_by` | behavior | decision | Behavior shaped by decisions (soft ref) |

---

## Validation Codes

Every diagnostic code, with its meaning, owner and fix, is listed in [Diagnostic Codes](diagnostics.md), generated from the catalog that `specforge explain <CODE>` prints.
