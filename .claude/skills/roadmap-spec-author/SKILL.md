---
name: roadmap-spec-author
description: Author Roadmap Specifications following the Roadmap Lifecycle Cycle (Research → Grouping → Item → Delivery → Verification). Produces structured roadmap documents with variant-aware templates, traceability, and verification.
user_invocable: true
---

# Roadmap Spec Author

Author **Roadmap Specifications** following the **Roadmap Lifecycle Cycle** (Research → Grouping → Item → Delivery → Verification). Every roadmap is built from **6 entity types**, configured along **10 axes**, and verified with **8 automated checks**.

## Entity Types

| Entity | ID Pattern | Purpose | Contains |
|--------|------------|---------|----------|
| **Roadmap** | `RM-<name>` | Root container, variant selection, metadata | Grouping containers, product milestones, dependency graph |
| **Grouping Container** | `PH-N` / `REL-X.Y.Z` / `FT-N` | Organizes work items by phase, release, or feature | Work items, exit criteria, external dependencies |
| **Work Item** | `WI-<container>-N` | Single deliverable within a container | Scope, deliverables, status, behavior allocation, risk notes |
| **Dependency Edge** | `DEP-<source>→<target>` | Blocking relationship between containers or items | Source, target, constraint description |
| **Exit Criterion** | `EC-<container>-N` | Measurable completion condition for a container | Metric, threshold, verification method |
| **Product Milestone** | `PT-N` | Business milestone aligned to technical work | Goal, aligned containers, success metric |

## Lifecycle Cycle

```
┌──────────────────────────────────────────────────────────────────┐
│                    ROADMAP LIFECYCLE CYCLE                        │
└──────────────────────────────────────────────────────────────────┘

Research/Discovery ──► Grouping Container ──► Work Item ──► Behavior Allocation
       ▲                                                          │
       │                                                          ▼
Product Milestone ◄── Verification ◄── Delivery ◄── Exit Criteria
```

Every roadmap item must complete this chain:

1. **Research** informs what needs building (spec docs, discovery notes)
2. **Grouping** (phase/release/feature) organizes related work
3. **Item** defines scope, deliverables, and status
4. **Behaviors** are allocated to items (traceability to spec)
5. **Exit criteria** define measurable completion conditions
6. **Delivery** confirms criteria met, status transitions to `Delivered`
7. **Verification** validates cross-references and consistency
8. **Product milestone** (optional) aligns technical work to business value
9. **Research loop** — delivery insights feed the next cycle

## Variant Selection

Choose the variant that matches your scope:

```
                        How large is the scope?
                               │
              ┌────────────────┼────────────────┐
              ▼                ▼                 ▼
         Multi-package    Single package     Single package
         14+ phases       3–11 features      per-release items
              │                │                  │
              ▼                ▼                  ▼
          PRODUCT          LIBRARY            PACKAGE
     (phase-based)    (feature-based)    (release-based)
                                                 │
                                          Fewer than 5 items,
                                          no dependencies?
                                                 │
                                                 ▼
                                             MINIMAL
                                          (flat list)
```

| Variant | Grouping | When to Use |
|---------|----------|-------------|
| **Product** | Phase-based (`PH-N`) | Multi-package products, 10+ phases, full traceability |
| **Library** | Feature-based (`FT-N`) | Single library, 3–11 features, document control needed |
| **Package** | Release-based (`REL-X.Y.Z`) | Single package, release-scoped items, blocking deps |
| **Minimal** | Flat list | Under 5 items, no dependencies, quick planning |

See [Variant Profiles](./references/variant-profiles.md) for full axis configurations.

## Markdown Templates

### Roadmap Header (All Variants)

```markdown
---
id: RM-<name>
kind: roadmap
title: "<Project Name> — Roadmap"
status: Draft
phase: "<variant: product|library|package|minimal>"
dependencies: []
---

# <Project Name> — Roadmap

## Goal

<1–3 sentence description of what the roadmap achieves>
```

### Grouping Container — Phase (Product Variant)

```markdown
## Phase N: <Phase Name>

**Status:** Planned | Specified | In Progress | Delivered | Deferred
**Source:** <path/to/research-doc.md> | <spec section reference>

### Scope

- <scope item 1>
- <scope item 2>

### Deliverables

| # | Deliverable | Package | Behaviors | Status |
|---|-------------|---------|-----------|--------|
| WI-PH-N-1 | <name> | `@pkg/name` | BEH-XX-NNN–NNN | Planned |
| WI-PH-N-2 | <name> | `@pkg/name` | BEH-XX-NNN–NNN | Planned |

### Exit Criteria

- [ ] EC-PH-N-1: <measurable criterion>
- [ ] EC-PH-N-2: <measurable criterion>

### Risk

<risk description or "None identified">
```

### Grouping Container — Feature (Library Variant)

```markdown
## N. <Feature Name>

**Status:** Planned | Specified | In Progress | Delivered | Deferred

### Scope

<description of what this feature covers>

### Deliverables

| # | Deliverable | Spec Section | Status |
|---|-------------|-------------|--------|
| WI-FT-N-1 | <name> | §NN | Planned |
| WI-FT-N-2 | <name> | §NN | Planned |

### Exit Criteria

- [ ] EC-FT-N-1: <measurable criterion>
```

### Grouping Container — Release (Package Variant)

```markdown
## Release X.Y.Z

**Status:** Planned | Specified | In Progress | Delivered | Deferred

### Items

| # | Item | Behaviors | Status |
|---|------|-----------|--------|
| WI-REL-XYZ-1 | <name> | BEH-XX-NNN–NNN | Planned |

### Advancement Criteria

- [ ] EC-REL-XYZ-1: <criterion for release readiness>
```

### Dependency Graph (Product/Package Variants)

```markdown
## Dependency Graph

<!-- Product variant: ASCII tree -->
PH-1 ─── Foundation
├── PH-2 ─── Core Types
│   ├── PH-3 ─── Runtime
│   └── PH-4 ─── Adapters
└── PH-5 ─── Integration
    └── PH-6 ─── Testing

<!-- Package variant: blocking notation -->
REL-0.1.0 → REL-0.2.0    (types must stabilize)
REL-0.2.0 → REL-0.3.0    (runtime before adapters)
```

### External Dependencies

```markdown
## External Dependencies

| Dependency | Required By | Blocking Phase | Notes |
|------------|------------|----------------|-------|
| `@pkg/core` v2.0 | PH-3 | PH-2 must deliver first | Types needed |
```

### Product Milestones (Product Variant)

```markdown
## Product Track

| # | Milestone | Aligned Phases | Success Metric |
|---|-----------|---------------|----------------|
| PT-1 | <milestone name> | PH-1, PH-2 | <metric> |
| PT-2 | <milestone name> | PH-3, PH-4 | <metric> |
```

### Document Control (Library Variant)

```markdown
## Document Control

| Field | Value |
|-------|-------|
| Document ID | RM-<name> |
| Revision | N |
| Last Updated | YYYY-MM-DD |
| Change Control | CCR-NNN |
```

### Status Summary (All Variants)

```markdown
## Status Summary

| Container | Status | Items | Delivered | Remaining |
|-----------|--------|-------|-----------|-----------|
| PH-1 | Delivered | 5 | 5 | 0 |
| PH-2 | In Progress | 8 | 3 | 5 |
| PH-3 | Planned | 6 | 0 | 6 |
```

## Workflow

1. **Select variant** — Choose product/library/package/minimal based on scope scale. See [Variant Profiles](./references/variant-profiles.md).
2. **Define grouping strategy** — Create phases, releases, or features with names and ordering.
3. **Enumerate work items** — List scope + deliverables per container. Assign `WI-` IDs.
4. **Allocate behaviors** — Assign BEH-ID ranges from spec `behaviors/` directory (if traceability depth warrants).
5. **Set exit criteria** — Write measurable criteria per container. Assign `EC-` IDs.
6. **Model dependencies** — Build dependency graph/tree between containers. Assign `DEP-` edges.
7. **Map external dependencies** — List cross-package blockers with blocking phase.
8. **Add product milestones** — Align business goals to technical phases (product variant only).
9. **Link research sources** — Reference research/discovery docs that informed each container.
10. **Verify** — Run verification script; all 8 checks must PASS.

## Verification

Run from the spec directory containing the roadmap:

```bash
./scripts/verify-roadmap.sh
```

The script performs 8 checks:

| # | Check | Description |
|---|-------|-------------|
| 1 | Status Validity | All status values are from `{Planned, Specified, In Progress, Delivered, Deferred}` |
| 2 | Behavior ID Ranges | All `BEH-XX-NNN–NNN` ranges reference existing behaviors in `behaviors/` |
| 3 | No Orphan Containers | Every grouping container appears in the dependency graph or status summary |
| 4 | Exit Criteria Presence | Every grouping container has at least one exit criterion |
| 5 | Dependency Acyclicity | The dependency graph contains no cycles |
| 6 | Spec File References | All referenced spec file paths exist on disk |
| 7 | Product Milestone Alignment | Every `PT-N` references valid grouping containers |
| 8 | External Dep Validity | External dependency spec paths exist |

Output: Markdown table, exit code 0 (all pass) or 1 (any fail).

## Quick Reference

- [Axes Catalog](./references/axes-catalog.md) — 10 axes with decision criteria and templates
- [Lifecycle Cycle](./references/lifecycle.md) — Cycle diagram, 8 enforcement rules, status state machine
- [Variant Profiles](./references/variant-profiles.md) — 4 pre-configured profiles with full templates
- [Verify Template](./assets/verify-roadmap-template.sh) — Bash verification script
- [Product Example](./examples/product/roadmap.md) — 3-phase product roadmap
- [Library Example](./examples/library/roadmap.md) — 3-feature library roadmap
