# DataForge — Roadmap

**Variant:** product
**Status:** Active
**Last Updated:** 2025-03-01

## Goal

Deliver a type-safe data transformation pipeline framework with compile-time schema validation, runtime streaming, and a plugin ecosystem — across `@dataforge/core`, `@dataforge/plugins`, and `@dataforge/cli` packages.

---

## Phase 1: Foundation

**Status:** Delivered
**Source:** spec/dataforge/research/foundation-patterns.md

### Scope

- Core type system for schema definitions
- Port and adapter interfaces
- Error result types

### Deliverables

| # | Deliverable | Package | Behaviors | ADR | Status |
|---|-------------|---------|-----------|-----|--------|
| WI-PH-1-1 | Schema type system | `@dataforge/core` | BEH-DF-001–018 | ADR-001 | Delivered |
| WI-PH-1-2 | Port factory | `@dataforge/core` | BEH-DF-019–032 | ADR-002 | Delivered |
| WI-PH-1-3 | Result/Error types | `@dataforge/core` | BEH-DF-033–041 | — | Delivered |

### Exit Criteria

- [x] EC-PH-1-1: All 41 foundation behaviors pass unit tests
- [x] EC-PH-1-2: TypeScript strict mode compiles with zero errors
- [x] EC-PH-1-3: API surface documented with TSDoc

### Risk

None — foundation patterns well-established from prior art.

---

## Phase 2: Transform Engine

**Status:** In Progress
**Source:** spec/dataforge/research/transform-patterns.md

### Scope

- Pipeline builder API
- Transform composition (map, filter, flatMap, reduce)
- Schema validation at transform boundaries

### Deliverables

| # | Deliverable | Package | Behaviors | ADR | Status |
|---|-------------|---------|-----------|-----|--------|
| WI-PH-2-1 | Pipeline builder | `@dataforge/core` | BEH-DF-042–067 | ADR-003 | Delivered |
| WI-PH-2-2 | Transform operators | `@dataforge/core` | BEH-DF-068–089 | — | In Progress |
| WI-PH-2-3 | Boundary validation | `@dataforge/core` | BEH-DF-090–102 | ADR-004 | Planned |

### Exit Criteria

- [x] EC-PH-2-1: Pipeline builder supports 4+ transform types
- [ ] EC-PH-2-2: Schema validation catches type mismatches at boundary
- [ ] EC-PH-2-3: Transform composition handles 10+ chained operators without stack overflow

### Risk

- Type inference performance may degrade with deeply chained transforms — mitigate with benchmark gate at 50+ operators
- Schema validation error messages must be human-readable — needs UX review

---

## Phase 3: Plugin Ecosystem

**Status:** Planned
**Source:** spec/dataforge/research/plugin-architecture.md

### Scope

- Plugin registration and discovery
- Built-in plugins (CSV, JSON, Parquet)
- Plugin testing utilities

### Deliverables

| # | Deliverable | Package | Behaviors | ADR | Status |
|---|-------------|---------|-----------|-----|--------|
| WI-PH-3-1 | Plugin registry | `@dataforge/plugins` | BEH-DF-103–118 | ADR-005 | Planned |
| WI-PH-3-2 | CSV plugin | `@dataforge/plugins` | BEH-DF-119–130 | — | Planned |
| WI-PH-3-3 | JSON plugin | `@dataforge/plugins` | BEH-DF-131–142 | — | Planned |
| WI-PH-3-4 | Plugin test harness | `@dataforge/plugins` | BEH-DF-143–155 | ADR-006 | Planned |

### Exit Criteria

- [ ] EC-PH-3-1: Plugin registry discovers and loads plugins without runtime reflection
- [ ] EC-PH-3-2: CSV and JSON plugins handle files up to 1GB via streaming
- [ ] EC-PH-3-3: Plugin test harness validates any plugin in under 5s

### Risk

- Streaming large files requires backpressure handling — needs prototype before committing to API
- Plugin discovery without reflection limits dynamic loading patterns

---

## Dependency Graph

```
PH-1 ─── Foundation
├── PH-2 ─── Transform Engine
│   └── PH-3 ─── Plugin Ecosystem
└── PH-3 ─── Plugin Ecosystem
```

---

## External Dependencies

| Dependency | Required By | Blocking Phase | Notes |
|------------|------------|----------------|-------|
| `@hex-di/core` v2.0 | PH-1 | — | Port factory API used for all adapters |
| `@hex-di/graph` v1.5 | PH-3 | PH-2 must deliver first | Plugin graph composition |
| `papaparse` v5.4 | PH-3 | PH-2 must deliver first | CSV parsing engine |

---

## Product Track

| # | Milestone | Aligned Phases | Success Metric |
|---|-----------|---------------|----------------|
| PT-1 | Core API Usable | PH-1, PH-2 | Pipeline builder demo transforms JSON end-to-end |
| PT-2 | Plugin MVP | PH-2, PH-3 | 3 built-in plugins pass integration tests |
| PT-3 | GA Release | PH-1, PH-2, PH-3 | 100% behavior coverage, all OQ pass |

---

## Status Summary

| Phase | Name | Status | Items | Delivered | Remaining |
|-------|------|--------|-------|-----------|-----------|
| PH-1 | Foundation | Delivered | 3 | 3 | 0 |
| PH-2 | Transform Engine | In Progress | 3 | 1 | 2 |
| PH-3 | Plugin Ecosystem | Planned | 4 | 0 | 4 |
