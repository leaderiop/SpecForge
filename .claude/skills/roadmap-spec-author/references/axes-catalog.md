# Axes Catalog

10 axes along which roadmaps vary. Each axis has options, decision criteria, and a template snippet. Variant defaults are listed at the end.

## Axis 1: Grouping Strategy

**What it decides:** How work items are organized into containers.

| Option | Description | Use When |
|--------|-------------|----------|
| `phase-based` | Sequential numbered phases (PH-1, PH-2, ...) | Work has natural ordering, later phases depend on earlier ones |
| `release-based` | Semantic version releases (REL-0.1.0, REL-0.2.0, ...) | Package with clear version milestones |
| `feature-based` | Named features (FT-1, FT-2, ...) | Library with independent feature areas |
| `flat` | No containers, items listed directly | Fewer than 5 items, no dependencies |

### Template — Phase-Based

```markdown
## Phase 1: <Name>

**Status:** Planned
**Source:** <path/to/research.md>

### Deliverables
| # | Deliverable | Package | Behaviors | Status |
|---|-------------|---------|-----------|--------|
| WI-PH-1-1 | <name> | `@pkg/name` | BEH-XX-NNN–NNN | Planned |
```

### Template — Release-Based

```markdown
## Release 0.1.0

**Status:** Planned

### Items
| # | Item | Behaviors | Status |
|---|------|-----------|--------|
| WI-REL-010-1 | <name> | BEH-XX-NNN–NNN | Planned |
```

### Template — Feature-Based

```markdown
## 1. <Feature Name>

**Status:** Planned

### Scope
<what this feature covers>

### Deliverables
| # | Deliverable | Spec Section | Status |
|---|-------------|-------------|--------|
| WI-FT-1-1 | <name> | §NN | Planned |
```

### Template — Flat

```markdown
## Deliverables

| # | Item | Status | Notes |
|---|------|--------|-------|
| 1 | <name> | Planned | <notes> |
```

---

## Axis 2: Scope Scale

**What it decides:** How large and detailed the roadmap is.

| Option | Description | Use When |
|--------|-------------|----------|
| `large` | 10+ containers, 50+ work items, 100+ behaviors | Multi-package product (e.g., SpecForge: 14 phases, 272 behaviors) |
| `medium` | 3–11 containers, 10–50 work items | Single library with multiple feature areas (e.g., Guard: 11 features) |
| `small` | 1–5 containers, fewer than 10 work items | Single package or minimal scope |

No template — scale affects quantity of entities, not their structure.

---

## Axis 3: Traceability Depth

**What it decides:** How deeply items connect to spec artifacts.

| Option | Description | Use When |
|--------|-------------|----------|
| `full` | BEH-ID ranges + ADR references + research source links | Spec has numbered behaviors, formal decisions, discovery docs |
| `medium` | Spec section references (§NN) + package references | Spec exists but behaviors aren't individually numbered |
| `light` | Deliverable references only, no behavior mapping | No formal spec, or spec is minimal |
| `none` | No traceability links | Minimal roadmap, no spec |

### Template — Full Traceability

```markdown
| # | Deliverable | Package | Behaviors | ADR | Source |
|---|-------------|---------|-----------|-----|--------|
| WI-PH-1-1 | Port system | `@pkg/core` | BEH-SF-001–042 | ADR-001 | research/ports.md |
```

### Template — Medium Traceability

```markdown
| # | Deliverable | Spec Section | Package | Status |
|---|-------------|-------------|---------|--------|
| WI-FT-1-1 | Policy engine | §3–§12 | `@pkg/guard` | Planned |
```

### Template — Light Traceability

```markdown
| # | Deliverable | Status |
|---|-------------|--------|
| WI-REL-010-1 | Add retry support | Planned |
```

---

## Axis 4: Dependency Modeling

**What it decides:** How inter-item dependencies are expressed.

| Option | Description | Use When |
|--------|-------------|----------|
| `ascii-tree` | ASCII art tree showing parent→child relationships | Phase-based roadmap with hierarchical dependencies |
| `blocking-graph` | Arrow notation with constraint descriptions | Release-based roadmap with blocking prerequisites |
| `implicit` | Ordering implies dependency (item N before N+1) | Feature-based roadmap where features are mostly independent |
| `none` | No dependency modeling | Flat roadmap, items are independent |

### Template — ASCII Tree

```markdown
## Dependency Graph

PH-1 ─── Foundation
├── PH-2 ─── Core Types
│   ├── PH-3 ─── Runtime
│   └── PH-4 ─── Adapters
├── PH-5 ─── Integration
│   └── PH-6 ─── Testing
└── PH-7 ─── Documentation
```

### Template — Blocking Graph

```markdown
## Dependency Graph

REL-0.1.0 → REL-0.2.0    (types must stabilize before runtime)
REL-0.2.0 → REL-0.3.0    (runtime before adapters)
REL-0.1.0 → REL-0.3.0    (types also needed by adapters)
```

---

## Axis 5: Product Alignment

**What it decides:** Whether business milestones are tracked alongside technical work.

| Option | Description | Use When |
|--------|-------------|----------|
| `parallel-track` | Explicit PT-N milestones with aligned phases and success metrics | Product-level roadmap with business stakeholders |
| `implicit` | Business goals mentioned in container descriptions but not structured | Library with loose business alignment |
| `none` | No business milestone tracking | Package-level or minimal roadmap |

### Template — Parallel Track

```markdown
## Product Track

| # | Milestone | Aligned Phases | Success Metric |
|---|-----------|---------------|----------------|
| PT-1 | MVP Launch | PH-1, PH-2, PH-3 | Core API usable end-to-end |
| PT-2 | GA Release | PH-4, PH-5, PH-6 | 90% spec coverage, all OQ pass |
```

---

## Axis 6: Document Control

**What it decides:** How the roadmap document itself is versioned.

| Option | Description | Use When |
|--------|-------------|----------|
| `explicit-block` | Document ID, revision number, last updated, CCR reference | Library with formal change control |
| `git-derived` | Version derived from git history, no explicit block | Package using git as single source of truth |
| `none` | No document control metadata | Minimal or internal-only roadmap |

### Template — Explicit Block

```markdown
## Document Control

| Field | Value |
|-------|-------|
| Document ID | RM-<name> |
| Revision | 1 |
| Last Updated | 2025-01-15 |
| Change Control | CCR-042 |
```

### Template — Git-Derived

```markdown
## Document Control

Version derived from git. See `git log -- roadmap.md` for history.
```

---

## Axis 7: External Dependencies

**What it decides:** How cross-package dependencies are tracked.

| Option | Description | Use When |
|--------|-------------|----------|
| `explicit-table` | Full table with dependency, required-by, blocking phase, notes | Multi-package product with cross-package blockers |
| `advancement-criteria` | Criteria listed per container that reference external packages | Single package that depends on external releases |
| `none` | No external dependency tracking | Self-contained package or minimal roadmap |

### Template — Explicit Table

```markdown
## External Dependencies

| Dependency | Required By | Blocking Phase | Notes |
|------------|------------|----------------|-------|
| `@hex-di/core` v2.0 | PH-3 | PH-2 must deliver first | Port types needed |
| `@hex-di/graph` v1.5 | PH-5 | PH-4 must deliver first | Builder API required |
```

### Template — Advancement Criteria

```markdown
### Advancement Criteria

- [ ] `@hex-di/core` must publish port factory API
- [ ] Upstream `Effect` v3.0 released
```

---

## Axis 8: Risk Treatment

**What it decides:** How risks are documented per item.

| Option | Description | Use When |
|--------|-------------|----------|
| `per-phase-exit` | Risks noted alongside exit criteria per phase | Product roadmap where risks gate phase completion |
| `per-item-notes` | Risk field on individual work items or containers | Library/package roadmap needing item-level risk awareness |
| `aggregated-fmea` | Reference to external FMEA document | GxP-regulated or safety-critical roadmap |
| `none` | No risk documentation | Minimal or low-risk roadmap |

### Template — Per-Phase Exit

```markdown
### Risk

- Type inference performance may degrade with 50+ adapters — mitigate with benchmark gate
- Breaking change to port API requires coordinated migration across 3 packages
```

### Template — Per-Item Notes

```markdown
| # | Deliverable | Status | Risk |
|---|-------------|--------|------|
| WI-FT-1-1 | Async evaluator | Planned | Performance with 100+ policies unknown |
```

---

## Axis 9: Status Lifecycle

**What it decides:** The state machine for grouping containers and work items.

All variants use the same 5-state lifecycle:

```
         ┌──────────────────────────────────────┐
         │                                      ▼
Planned ───► Specified ───► In Progress ───► Delivered
   │
   └──────────────► Deferred
```

| Status | Meaning |
|--------|---------|
| `Planned` | Identified but not yet specified in detail |
| `Specified` | Spec complete, ready for implementation |
| `In Progress` | Implementation actively underway |
| `Delivered` | All exit criteria met, work complete |
| `Deferred` | Intentionally postponed to a later cycle |

Valid transitions:

| From | To |
|------|-----|
| Planned | Specified, Deferred |
| Specified | In Progress, Deferred |
| In Progress | Delivered, Deferred |
| Deferred | Planned |

---

## Axis 10: Research Traceability

**What it decides:** Whether items link back to research/discovery documents.

| Option | Description | Use When |
|--------|-------------|----------|
| `explicit-source` | `Source:` field on each container pointing to research doc | Research docs exist and informed the roadmap |
| `implicit` | Research mentioned in descriptions but not structured | Informal discovery process |
| `none` | No research links | Roadmap authored without formal research |

### Template — Explicit Source

```markdown
## Phase 1: Foundation

**Status:** Planned
**Source:** spec/research/foundation-discovery.md
```

---

## Variant Defaults

| Axis | Product | Library | Package | Minimal |
|------|---------|---------|---------|---------|
| 1. Grouping | phase-based | feature-based | release-based | flat |
| 2. Scale | large | medium | small | small |
| 3. Traceability | full | medium | medium | light |
| 4. Dependencies | ascii-tree | implicit | blocking-graph | none |
| 5. Product Alignment | parallel-track | none | none | none |
| 6. Document Control | none | explicit-block | git-derived | none |
| 7. External Deps | explicit-table | advancement-criteria | advancement-criteria | none |
| 8. Risk | per-phase-exit | per-item-notes | per-item-notes | none |
| 9. Status | 5-state | 5-state | 5-state | 5-state |
| 10. Research | explicit-source | none | none | none |
