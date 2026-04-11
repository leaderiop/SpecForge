# SpecForge Documentation

This directory contains the entity model reference and design documentation for SpecForge. For a project overview, see the [root README](../README.md). For the full architecture — core + plugins, edge types, validation rules, and design principles — see [entity-model.md](entity-model.md).

## Entity Reference

### Core (8 entities)

| Entity | ID Pattern | Purpose |
|--------|-----------|---------|
| [spec](entities/spec.md) | singleton | Project root configuration — name, infix, version, codegen settings |
| [invariant](entities/invariant.md) | `INV-{infix}-{n}` | Runtime guarantee the system must never violate |
| [behavior](entities/behavior.md) | `BEH-{infix}-{n}` | Behavioral contract for a single operation with RFC 2119 keywords |
| [feature](entities/feature.md) | `FEAT-{infix}-{n}` | User-facing capability composed of behaviors |
| [event](entities/event.md) | `EVT-{infix}-{n}` | Domain event emitted by a behavior, consumed by others |
| [type](entities/type.md) | identifier | Data type definition — structs, unions, errors, commands |
| [port](entities/port.md) | identifier | Interface contract — hexagonal architecture boundary |
| [ref](entities/ref.md) | `scheme.kind:identifier` | External reference — typed link to issues, tickets, designs |

### @specforge/product (5 entities)

| Entity | ID Pattern | Purpose |
|--------|-----------|---------|
| [journey](entities/journey.md) | `identifier` | UX flow mapping a persona + surface to features |
| [deliverable](entities/deliverable.md) | `identifier` | Shippable artifact bundling journeys and modules |
| [milestone](entities/milestone.md) | `identifier` | Planning phase with scheduled features and exit criteria |
| [module](entities/module.md) | `identifier` | Code package mapping features to ports |
| [term](entities/term.md) | `identifier` | Structured vocabulary defining the project's ubiquitous language |

### @specforge/governance (3 entities)

| Entity | ID Pattern | Purpose |
|--------|-----------|---------|
| [decision](entities/decision.md) | `ADR-{n}` | Architecture Decision Record — rationale for technical choices |
| [constraint](entities/constraint.md) | `CON-{infix}-{n}` | Non-functional requirement with measurable thresholds |
| [failure_mode](entities/failure-mode.md) | `FM-{infix}-{n}` | FMEA risk assessment tied to an invariant |

### @specforge/formal (6 entity kinds + enhances @specforge/software)

| Entity | ID Pattern | Purpose |
|--------|-----------|---------|
| condition | `identifier` | Named, reusable precondition/postcondition/frame invariant |
| property | `identifier` | Temporal/behavioral assertion (safety/liveness/fairness) |
| axiom | `identifier` | Assumed-true foundation (no proof required) |
| protocol | `identifier` | Shared synchronization contract across events |
| refinement | `identifier` | Abstract-to-concrete behavior mapping with condition deltas |
| process | `identifier` | CSP-style communicating process with alphabet and composition |

Provides structured conditions (dual-mode: inline blocks + condition entity references), specification layering, event graph linting, and coverage tracking. Contributes 11 edge types (RequiresCondition, EnsuresCondition, MaintainsCondition, AssumedBy, Satisfies, FollowsProtocol, PropertyDependsOn, RefinesTo, RefinementChainLink, ParticipatesIn, ProcessComposition), 4 compiler passes (condition_check, layering_verify, event_graph_analyze, coverage_tracking), and formal analysis diagnostics (E030-E035, E041-E042, W028-W040, W058-W074). Requires `warning_level=strict` in specforge.json.

## Traceability Chain

### Core

```
feature ──implements──→ behavior ──references──→ invariant
  FEAT-XX-N              BEH-XX-N     │           INV-XX-N
                                      │produces
                                      ▼
                                    event
                                    EVT-XX-N

                         type / port (code bridge)

          any entity ──links_to──→ ref (external reference bridge)
                                   scheme.kind:id
```

### Extended by @specforge/product

```
deliverable ──bundles──→ journey ──traces_to──→ feature (core)
                │
                │built_from
                ▼
              module ──provides──→ feature (core)
                        │
                        │defines_port
                        ▼
                      port (core)

milestone ──schedules──→ feature (core) / deliverable
```

### Extended by @specforge/governance

```
decision ──protects──→ invariant (core)
  ADR-N

constraint ──constrains──→ behavior (core) / invariant (core)
  CON-XX-N

failure_mode ──mitigates──→ invariant (core)
  FM-XX-N
```

**Full chain:** `deliverable -> journey -> feature -> behavior -> invariant`

## Quick Reference

See **[quick-reference.md](quick-reference.md)** for a single-page cheat sheet covering all 16 entities, 20 edge types, and 27 validation codes.

## AI Agent Token Economics

AI coding agents spend 60-80% of their tokens on *discovery* — not *building*. SpecForge provides structured, machine-readable context that eliminates this waste:

- **90-95% fewer tokens** for context gathering (spec graph query vs. 20-50 file reads)
- **75-86% total token reduction** per agent task
- **70% fewer rework cycles** (first-shot accuracy from precise contracts)
- **Developer time savings exceed token savings by 10x** (~4,000 hours/year for a 100-dev org)

The industry has independently converged on structured context files (CLAUDE.md, .cursor/rules, copilot-instructions.md). SpecForge is the **compiled, cross-referenced, validated** version of what these tools approximate.

> Full analysis with citations: **[RES-18: AI Agent Token Economics](../spec/research/RES-18-ai-agent-token-economics.md)**

## Research

| ID | Title |
|----|-------|
| [RES-11a](../spec/research/RES-11a-spec-dsl-core-compiler.md) | Core compiler architecture |
| [RES-11b](../spec/research/RES-11b-spec-dsl-codegen-plugins.md) | Code generation & test plugins |
| [RES-12](../spec/research/RES-12-dsl-concept-expansion.md) | DSL concept expansion analysis |
| [RES-13](../spec/research/RES-13-README.md) | Market landscape 2026 |
| [RES-18](../spec/research/RES-18-ai-agent-token-economics.md) | **AI agent token economics — cost reduction analysis** |
| [RES-19](../spec/research/RES-19-market-position-success-estimation.md) | **Market position & success estimation** |
| [Extension Model](extension-model.md) | Plugins, providers, and generators architecture |

## Business Plan

A comprehensive business plan covering 10 areas (executive summary, financials, go-to-market, technical roadmap, pricing, product strategy, community, investment thesis, operations, strategic analysis) is available at **[business/README.md](../business/README.md)**.

## Validation Codes

### Errors

| Code | Module | Rule |
|------|--------|------|
| E001 | core | No dangling references — every ID must resolve to a declared entity |
| E002 | core | No duplicate IDs — each entity ID is globally unique |
| E003 | core | No import cycles — `imports` edges form a DAG |
| E005 | governance | RPN mismatch — severity x occurrence x detection must equal declared rpn |
| E006 | core | Event trigger invalid — trigger must reference an existing behavior |
| E011 | core | Invalid ref target format — provider validates identifier doesn't match expected pattern |
| E012 | core | Unknown provider kind — ref uses kind not registered by its provider |
| E007 | product | Circular module dependency — `depends_on` edges must form a DAG |
| E008 | product | Persona not defined — journey persona must match spec root definition |
| E009 | product | Surface not defined — journey surface must match spec root definition |
| E010 | product | Behavior range invalid — range start > end or expanded IDs don't exist |

### Warnings

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

### Info

| Code | Module | Rule |
|------|--------|------|
| I001 | governance | Stale proposal — decision with `status: proposed` older than 30 days |
| I003 | core | Newer format features available — project version < compiler version |
| I004 | core | Unknown entity prefix — ID prefix not registered by any installed module |
| I005 | core | Unknown provider scheme — ref uses scheme not registered by any installed provider |
