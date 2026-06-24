# Quick Reference

Single-page lookup for all 16 SpecForge entities, 20 edge types, and 27 validation codes. For full details, see the [entity model](entity-model.md) or individual [entity docs](entities/).

---

## Entities

### spec
> Module: core | ID: singleton | "What project is this?"

| Required | Optional |
|----------|----------|
| name, infix, version | plugins, providers, test_dirs, persona, surface, define, coverage, gen |

No graph edges. Root configuration that scopes all entity IDs via the `infix` field.

---

### invariant
> Module: core | ID: `INV-{infix}-{n}` | "What must ALWAYS be true?"

| Required | Optional |
|----------|----------|
| title, guarantee | enforced_by, risk |

Outgoing: behavior (`enforces`)
Incoming: behavior (`references`), decision (`protects`), constraint (`constrains`), failure_mode (`mitigates`)

---

### behavior
> Module: core | ID: `BEH-{infix}-{n}` | "What exactly does the system do?"

| Required | Optional |
|----------|----------|
| title, contract | invariants, adrs, types, ports, verify, tests |

Outgoing: invariant (`references`), event (`produces`), type (`uses_type`), port (`uses_port`), decision (`shaped_by`)
Incoming: feature (`implements`), event (`consumes`), constraint (`constrains`)

---

### feature
> Module: core | ID: `FEAT-{infix}-{n}` | "What value does this deliver?"

| Required | Optional |
|----------|----------|
| title, behaviors, problem, solution | roadmap |

Outgoing: behavior (`implements`)
Incoming: capability (`traces_to`), library (`provides`), roadmap (`schedules`)

---

### event
> Module: core | ID: `EVT-{infix}-{n}` | "What does the system announce?"

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
> Module: @specforge/product | ID: `DLV-{infix}-{n}` | "What ships to users?"

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
> Module: @specforge/governance | ID: `CON-{infix}-{n}` | "What quality must the system achieve?"

| Required | Optional |
|----------|----------|
| title, category, priority, description/metric | behaviors/affects, invariants, verify |

Outgoing: behavior (`constrains`), invariant (`constrains`)
No incoming edges.

---

### failure_mode
> Module: @specforge/governance | ID: `FM-{infix}-{n}` | "What can go wrong and how bad is it?"

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

### Errors (11 codes)

| Code | Module | Rule |
|------|--------|------|
| E001 | core | Parse error — the `.spec` file has invalid syntax |
| E002 | core | No duplicate IDs — each entity ID is globally unique |
| E003 | core | No dangling references — every ID in a reference list must resolve to a declared entity |
| E005 | governance | RPN mismatch — severity x occurrence x detection must equal declared rpn |
| E006 | core | Event trigger invalid — trigger must reference an existing behavior |
| E011 | core | Invalid ref target format — provider validates identifier doesn't match expected pattern |
| E012 | core | Unknown provider kind — ref uses kind not registered by its provider |
| E007 | product | Circular module dependency — `depends_on` must form a DAG |
| E008 | product | Persona not defined — journey persona must match spec root definition |
| E009 | product | Surface not defined — journey surface must match spec root definition |
| E010 | product | Behavior range invalid — range start > end or expanded IDs don't exist |

### Warnings (12 codes)

| Code | Module | Rule |
|------|--------|------|
| W001 | core | Orphan behavior — not referenced by any feature |
| W002 | product | Orphan feature — not referenced by any journey |
| W003 | core | Unused invariant — not referenced by any behavior |
| W004 | core | Unverified behavior — no `verify` statement |
| W005 | governance | Unmitigated high-risk invariant — `risk: high` with no failure_mode |
| W006 | governance | Unconstrained behavior — no constraint coverage |
| W007 | core | Orphan event — event with no consumers |
| W012 | core | Orphan ref — declared but never referenced by any entity |
| W008 | product | Uncovered journey — deliverable journey not reachable via modules |
| W009 | product | Orphan module — not referenced by any deliverable |
| W010 | product | Deprecated feature — using a deprecated format feature |
| W011 | product | Orphan journey — not referenced by any deliverable |

### Info (4 codes)

| Code | Module | Rule |
|------|--------|------|
| I001 | governance | Stale proposal — decision with `status: proposed` older than 30 days |
| I003 | core | Newer format features available — project version < compiler version |
| I004 | core | Unknown entity prefix — ID prefix not registered by any installed module |
| I005 | core | Unknown provider scheme — ref uses scheme not registered by any installed provider |
