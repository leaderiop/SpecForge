# Variant Profiles

4 pre-configured axis combinations for common roadmap types. Each profile includes when to use, axis configuration, full template, and promotion criteria.

## Profile Comparison

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

---

## Product Profile

**When to use:** Multi-package product with 10+ phases, full behavior traceability, business stakeholders who need milestone tracking. Inspired by the SpecForge roadmap pattern.

**Example reference:** [examples/product/roadmap.md](../examples/product/roadmap.md)

### Axis Configuration

- Grouping: `phase-based` — Phases numbered PH-1 through PH-N
- Scale: `large` — 10+ phases, 50+ work items, 100+ behaviors
- Traceability: `full` — BEH-ID ranges, ADR references, research source links
- Dependencies: `ascii-tree` — Hierarchical ASCII tree
- Product Alignment: `parallel-track` — PT-N milestones with aligned phases
- Document Control: `none` — Git-managed
- External Deps: `explicit-table` — Full dependency table
- Risk: `per-phase-exit` — Risk notes alongside exit criteria
- Status: 5-state lifecycle
- Research: `explicit-source` — Source field on each phase

### Full Template

```markdown
# <Product Name> — Roadmap

**Variant:** product
**Status:** Draft | Active | Frozen
**Last Updated:** YYYY-MM-DD

## Goal

<1–3 sentence product vision that the roadmap achieves>

---

## Phase 1: <Phase Name>

**Status:** Planned | Specified | In Progress | Delivered | Deferred
**Source:** <path/to/research-doc.md>

### Scope

- <scope item 1>
- <scope item 2>

### Deliverables

| # | Deliverable | Package | Behaviors | ADR | Status |
|---|-------------|---------|-----------|-----|--------|
| WI-PH-1-1 | <name> | `@pkg/name` | BEH-XX-NNN–NNN | ADR-NNN | Planned |
| WI-PH-1-2 | <name> | `@pkg/name` | BEH-XX-NNN–NNN | — | Planned |

### Exit Criteria

- [ ] EC-PH-1-1: <measurable criterion>
- [ ] EC-PH-1-2: <measurable criterion>

### Risk

<risk description or "None identified">

---

<!-- Repeat Phase sections for PH-2 through PH-N -->

---

## Dependency Graph

PH-1 ─── <Phase 1 Name>
├── PH-2 ─── <Phase 2 Name>
│   ├── PH-3 ─── <Phase 3 Name>
│   └── PH-4 ─── <Phase 4 Name>
└── PH-5 ─── <Phase 5 Name>

---

## External Dependencies

| Dependency | Required By | Blocking Phase | Notes |
|------------|------------|----------------|-------|
| `@pkg/dep` vX.Y | PH-N | PH-M must deliver first | <notes> |

---

## Product Track

| # | Milestone | Aligned Phases | Success Metric |
|---|-----------|---------------|----------------|
| PT-1 | <milestone> | PH-1, PH-2 | <metric> |
| PT-2 | <milestone> | PH-3, PH-4 | <metric> |

---

## Status Summary

| Phase | Name | Status | Items | Delivered | Remaining |
|-------|------|--------|-------|-----------|-----------|
| PH-1 | <name> | Planned | N | 0 | N |
| PH-2 | <name> | Planned | N | 0 | N |
```

### Promotion Criteria

This is the largest profile. No promotion needed — if the roadmap grows beyond this, consider splitting into multiple product roadmaps.

---

## Library Profile

**When to use:** Single library with 3–11 independent features, document control needed for change tracking, medium traceability to spec sections. Inspired by the Guard roadmap pattern.

**Example reference:** [examples/library/roadmap.md](../examples/library/roadmap.md)

### Axis Configuration

- Grouping: `feature-based` — Features numbered FT-1 through FT-N
- Scale: `medium` — 3–11 features, 10–50 work items
- Traceability: `medium` — Spec section references (§NN), package names
- Dependencies: `implicit` — Feature ordering implies dependency
- Product Alignment: `none`
- Document Control: `explicit-block` — Document ID, revision, CCR
- External Deps: `advancement-criteria` — Per-feature criteria referencing external packages
- Risk: `per-item-notes` — Risk field on deliverable tables
- Status: 5-state lifecycle
- Research: `none`

### Full Template

```markdown
# <Library Name> — Roadmap

**Variant:** library
**Status:** Draft | Active | Frozen
**Last Updated:** YYYY-MM-DD

## Document Control

| Field | Value |
|-------|-------|
| Document ID | RM-<name> |
| Revision | 1 |
| Last Updated | YYYY-MM-DD |
| Change Control | CCR-NNN |

## Goal

<1–3 sentence description of what the library roadmap achieves>

---

## 1. <Feature Name>

**Status:** Planned | Specified | In Progress | Delivered | Deferred

### Scope

<description of what this feature covers>

### Deliverables

| # | Deliverable | Spec Section | Package | Risk | Status |
|---|-------------|-------------|---------|------|--------|
| WI-FT-1-1 | <name> | §NN–§NN | `@pkg/name` | <risk or "—"> | Planned |
| WI-FT-1-2 | <name> | §NN | `@pkg/name` | — | Planned |

### Exit Criteria

- [ ] EC-FT-1-1: <measurable criterion>
- [ ] EC-FT-1-2: <measurable criterion>

---

<!-- Repeat Feature sections for FT-2 through FT-N -->

---

## Status Summary

| # | Feature | Status | Items | Delivered | Remaining |
|---|---------|--------|-------|-----------|-----------|
| FT-1 | <name> | Planned | N | 0 | N |
| FT-2 | <name> | Planned | N | 0 | N |

---

## Version History

| Revision | Date | Change | CCR |
|----------|------|--------|-----|
| 1 | YYYY-MM-DD | Initial roadmap | CCR-NNN |
```

### Promotion Criteria

Promote to **Product** profile when:
- Features exceed 11
- Cross-package dependencies become significant (need explicit table)
- Business stakeholders need milestone tracking (need product track)
- Research docs exist and should be formally linked

---

## Package Profile

**When to use:** Single package with release-scoped items, blocking dependencies between releases, advancement criteria for deferred work. Inspired by the HTTP Client roadmap pattern.

### Axis Configuration

- Grouping: `release-based` — Releases named REL-X.Y.Z
- Scale: `small` — 1–5 releases, fewer than 10 work items per release
- Traceability: `medium` — Behavior ID ranges + invariant references
- Dependencies: `blocking-graph` — Arrow notation with constraint descriptions
- Product Alignment: `none`
- Document Control: `git-derived` — Version from git history
- External Deps: `advancement-criteria` — Per-release criteria
- Risk: `per-item-notes` — Risk field on item tables
- Status: 5-state lifecycle
- Research: `none`

### Full Template

```markdown
# <Package Name> — Roadmap

**Variant:** package
**Status:** Draft | Active | Frozen
**Last Updated:** YYYY-MM-DD

## Document Control

Version derived from git. See `git log -- roadmap.md` for history.

## Goal

<1–3 sentence description of what the package roadmap achieves>

---

## Release 0.1.0

**Status:** Planned | Specified | In Progress | Delivered | Deferred

### Items

| # | Item | Behaviors | Invariants | Risk | Status |
|---|------|-----------|-----------|------|--------|
| WI-REL-010-1 | <name> | BEH-XX-NNN–NNN | INV-XX-N | <risk or "—"> | Planned |
| WI-REL-010-2 | <name> | BEH-XX-NNN–NNN | — | — | Planned |

### Advancement Criteria

- [ ] EC-REL-010-1: <criterion for release readiness>
- [ ] EC-REL-010-2: <criterion>

---

<!-- Repeat Release sections -->

---

## Dependency Graph

REL-0.1.0 → REL-0.2.0    (<constraint description>)
REL-0.2.0 → REL-0.3.0    (<constraint description>)

---

## Deferred

Items not scheduled for any current release:

| # | Item | Advancement Criteria | Notes |
|---|------|---------------------|-------|
| WI-DEF-1 | <name> | <what must happen before scheduling> | <notes> |

---

## Status Summary

| Release | Status | Items | Delivered | Remaining |
|---------|--------|-------|-----------|-----------|
| 0.1.0 | Planned | N | 0 | N |
| 0.2.0 | Planned | N | 0 | N |
```

### Promotion Criteria

Promote to **Library** profile when:
- Releases become feature-oriented rather than version-oriented
- Need formal document control (CCR references)
- Work items exceed 10 per release
- Spec section references replace behavior IDs

---

## Minimal Profile

**When to use:** Fewer than 5 items, no dependencies between items, quick planning for a small scope. Inspired by the Result roadmap pattern.

### Axis Configuration

- Grouping: `flat` — No containers, items listed directly
- Scale: `small` — Fewer than 5 items
- Traceability: `light` — Deliverable references only
- Dependencies: `none`
- Product Alignment: `none`
- Document Control: `none`
- External Deps: `none`
- Risk: `none`
- Status: 5-state lifecycle
- Research: `none`

### Full Template

```markdown
# <Package Name> — Roadmap

**Variant:** minimal
**Status:** Draft | Active | Frozen
**Last Updated:** YYYY-MM-DD

## Goal

<1–3 sentence description>

## Deliverables

| # | Item | Status | Notes |
|---|------|--------|-------|
| 1 | <name> | Planned | <notes> |
| 2 | <name> | Planned | <notes> |
| 3 | <name> | Planned | <notes> |
```

### Promotion Criteria

Promote to **Package** profile when:
- Items exceed 5
- Dependencies emerge between items
- Need exit criteria or behavior traceability
- Multiple release versions planned
